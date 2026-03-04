use std::collections::HashMap;
use std::future::Future;
use std::net::{SocketAddr, ToSocketAddrs};
use tokio::task::LocalSet;
use crate::server::{CapsuleWaterController, get_server_config, serve_connection, ServerConfigurations, STATIC_SERVER_CONFIGURATION};
use crate::server::connection::{ConnectionStream, WaterStream};
use crate::server::matcher::{DynamicPathVec, Matcher, MatcherInitializer, PathHolder};

#[inline]
pub  fn serve_connections_by_channels<
    #[cfg(feature = "use_tokio_send")]
    Holder: Send + 'static + std::fmt::Debug,
    #[cfg(not(feature = "use_tokio_send"))]
    Holder,

    #[cfg(all(feature = "thread_shared_struct", not(feature = "use_tokio_send")))]
    SHARED: Clone,

    #[cfg(all(feature = "thread_shared_struct", feature = "use_tokio_send"))]
    SHARED: Clone + Send + 'static,
    const HS: usize, const QS: usize,
>(
    config: ServerConfigurations,
    #[cfg(feature = "thread_shared_struct")]
    controller: &'static mut CapsuleWaterController<Holder, SHARED, HS, QS>,
    #[cfg(not(feature = "thread_shared_struct"))]
    controller: &'static mut CapsuleWaterController<Holder, HS, QS>,

    #[cfg(all(feature = "use_tokio_send", feature = "thread_shared_struct"))]
    shared_factory: fn() -> std::pin::Pin<Box<dyn Future<Output = SHARED> + Send>>,
    #[cfg(all(feature = "thread_shared_struct", not(feature = "use_tokio_send")))]
    shared_factory: fn() -> std::pin::Pin<Box<dyn Future<Output = SHARED>>>,

    #[cfg(feature = "thread_shared_struct")]
    static_path: &'static mut Option<HashMap<String, PathHolder<Holder, SHARED, HS, QS>>>,
    #[cfg(feature = "thread_shared_struct")]
    dynamic_path: &'static mut Option<HashMap<usize, DynamicPathVec<Holder, SHARED, HS, QS>>>,

    #[cfg(not(feature = "thread_shared_struct"))]
    static_path: &'static mut Option<HashMap<String, PathHolder<Holder, HS, QS>>>,
    #[cfg(not(feature = "thread_shared_struct"))]
    dynamic_path: &'static mut Option<HashMap<usize, DynamicPathVec<Holder, HS, QS>>>,
) {
    // 1. GLOBAL INITIALIZATION
    unsafe { STATIC_SERVER_CONFIGURATION = Some(config); }
    let conf = get_server_config();

    controller.set_up(String::new());
    controller.____insure_binding();

    // 2. MATCHER SERIALIZATION
    *static_path = Some(HashMap::new());
    *dynamic_path = Some(HashMap::new());
    let sp = static_path.as_mut().unwrap();
    let dp = dynamic_path.as_mut().unwrap();
    _ = MatcherInitializer::serialize(sp, dp, controller);

    // Cast the controller to a 'static reference safely for threads
    #[cfg(feature = "thread_shared_struct")]
        let controller_ptr: &'static CapsuleWaterController<Holder, SHARED, HS, QS> = unsafe { &*(controller as *const _) };
    #[cfg(not(feature = "thread_shared_struct"))]
        let controller_ptr: &'static CapsuleWaterController<Holder, HS, QS> = unsafe { &*(controller as *const _) };

    let matcher = Matcher::new(static_path.as_ref().unwrap(), dynamic_path.as_ref().unwrap());

    // 3. CHANNEL PREPARATION
    let mut channel_senders = vec![];
    let mut channel_receivers = vec![];

    // Increase capacity to 1024 to handle high-concurrency bursts without blocking the acceptor
    for _ in 0..conf.worker_threads_count {
        let (tx, rx) = tokio::sync::mpsc::channel::<(tokio::net::TcpStream, SocketAddr)>(1024);
        channel_senders.push(tx);
        channel_receivers.push(rx);
    }

    // Wrap senders in Arc to allow all threads to Round-Robin into each other
    let shared_senders = std::sync::Arc::new(channel_senders);
    let mut workers = vec![];

    // 4. SPAWN WORKER/ACCEPTOR THREADS
    for worker_id in 0..conf.worker_threads_count {
        let matcher = matcher.clone();
        let all_senders = shared_senders.clone();

        // IMPORTANT: We take ownership of the receiver; it cannot be cloned.
        let mut my_receiver = channel_receivers.remove(0);

        let addresses = conf.addresses.clone();
        let backlog = conf.backlog;

        let t = std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            let local = LocalSet::new();
            rt.block_on(local.run_until(async move {
                // Initialize thread-local shared struct if feature is enabled
                #[cfg(feature = "thread_shared_struct")]
                    let shared_data = shared_factory().await;

                // --- TASK A: LOCAL ACCEPTORS (Distributing to the mesh) ---
                for (address, port) in addresses {
                    let all_senders = all_senders.clone();
                    tokio::task::spawn_local(async move {
                        let listener = {
                            let addr_str = format!("{}:{}", address, port);
                            let socket_addr = addr_str.to_socket_addrs().unwrap().next().expect("Invalid address");
                            let socket = match &socket_addr {
                                SocketAddr::V4(_) => tokio::net::TcpSocket::new_v4(),
                                SocketAddr::V6(_) => tokio::net::TcpSocket::new_v6(),
                            }.expect("Failed to create socket");

                            socket.set_reuseaddr(true).ok();
                            socket.set_nodelay(true).ok();
                            #[cfg(target_os = "linux")]
                            socket.set_reuseport(true).ok();

                            socket.bind(socket_addr).expect("Bind failed");
                            socket.listen(backlog).expect("Listen failed")
                        };

                        let mut rr_index = worker_id; // Start RR at own index to balance mesh
                        let worker_pool_size = all_senders.len();

                        loop {
                            if let Ok((stream, addr)) = listener.accept().await {
                                let target_worker = rr_index % worker_pool_size;
                                // Round-Robin Handoff
                                let _ = all_senders[target_worker].send((stream, addr)).await;
                                rr_index += 1;
                            }
                        }
                    });
                }
                #[cfg(feature = "thread_shared_struct")]
                let shared_struct = shared_factory().await;
                // --- TASK B: LOCAL WORKER (Processing received connections) ---

                while let Some((stream, socket_addr)) = my_receiver.recv().await {
                    let m = matcher.clone();
                    #[cfg(feature = "thread_shared_struct")]
                        let s = shared_data.clone();

                    tokio::task::spawn_local(async move {
                        let connection = ConnectionStream::new(WaterStream::TOStream(stream), socket_addr);
                        #[cfg(feature = "thread_shared_struct")]
                        serve_connection(connection, controller_ptr, s, m).await;
                        #[cfg(not(feature = "thread_shared_struct"))]
                        serve_connection(connection, controller_ptr, m).await;
                    });
                }
            }));
        });

        workers.push(t);
    }

    // 5. KEEP ALIVE
    for w in workers {
        let _ = w.join();
    }
}

