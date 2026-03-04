use std::collections::HashMap;
use std::net::{SocketAddr, ToSocketAddrs};
use tokio::task::LocalSet;
use crate::server::{CapsuleWaterController, get_server_config, serve_connection, ServerConfigurations, STATIC_SERVER_CONFIGURATION};
use crate::server::connection::{ConnectionStream, WaterStream};
use crate::server::matcher::{DynamicPathVec, Matcher, MatcherInitializer, PathHolder};

#[inline]
pub fn serve_connections_by_channels<
    #[cfg(feature = "use_tokio_send")] Holder: Send + 'static + std::fmt::Debug,
    #[cfg(not(feature = "use_tokio_send"))] Holder,
    #[cfg(all(feature = "thread_shared_struct", not(feature = "use_tokio_send")))] SHARED: Clone,
    #[cfg(all(feature = "thread_shared_struct", feature = "use_tokio_send"))] SHARED: Clone + Send + 'static,
    const HS: usize, const QS: usize,
>(
    config: ServerConfigurations,
    #[cfg(feature = "thread_shared_struct")] controller: &'static mut CapsuleWaterController<Holder, SHARED, HS, QS>,
    #[cfg(not(feature = "thread_shared_struct"))] controller: &'static mut CapsuleWaterController<Holder, HS, QS>,
    #[cfg(all(feature = "thread_shared_struct", feature = "use_tokio_send"))] shared_factory: fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = SHARED> + Send>>,
    #[cfg(all(feature = "thread_shared_struct", not(feature = "use_tokio_send")))] shared_factory: fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = SHARED>>>,
    #[cfg(feature = "thread_shared_struct")] static_path: &'static mut Option<HashMap<String, PathHolder<Holder, SHARED, HS, QS>>>,
    #[cfg(feature = "thread_shared_struct")] dynamic_path: &'static mut Option<HashMap<usize, DynamicPathVec<Holder, SHARED, HS, QS>>>,
    #[cfg(not(feature = "thread_shared_struct"))] static_path: &'static mut Option<HashMap<String, PathHolder<Holder, HS, QS>>>,
    #[cfg(not(feature = "thread_shared_struct"))] dynamic_path: &'static mut Option<HashMap<usize, DynamicPathVec<Holder, HS, QS>>>,
) {
    // FIX: Ensure configuration is set BEFORE any potential return
    unsafe { STATIC_SERVER_CONFIGURATION = Some(config); }
    let conf = get_server_config();

    controller.set_up(String::new());
    controller.____insure_binding();

    *static_path = Some(HashMap::new());
    *dynamic_path = Some(HashMap::new());
    _ = MatcherInitializer::serialize(static_path.as_mut().unwrap(), dynamic_path.as_mut().unwrap(), controller);

    #[cfg(feature = "thread_shared_struct")]
        let controller_ptr: &'static CapsuleWaterController<Holder, SHARED, HS, QS> = unsafe { &*(controller as *const _) };
    #[cfg(not(feature = "thread_shared_struct"))]
        let controller_ptr: &'static CapsuleWaterController<Holder, HS, QS> = unsafe { &*(controller as *const _) };

    let matcher = Matcher::new(static_path.as_ref().unwrap(), dynamic_path.as_ref().unwrap());
    let mut workers = vec![];
    let mut channel_senders = vec![];
    let mut channel_receivers = vec![];

    // 1. Setup Mesh Channels
    for _ in 0..conf.worker_threads_count {
        let (tx, rx) = tokio::sync::mpsc::channel::<(tokio::net::TcpStream, SocketAddr)>(1024);
        channel_senders.push(tx);
        channel_receivers.push(rx);
    }

    let shared_senders = std::sync::Arc::new(channel_senders);

    // 2. Spawn Threads
    for worker_id in 0..conf.worker_threads_count {
        let matcher = matcher.clone();
        let all_senders = shared_senders.clone();
        let mut my_receiver = channel_receivers.remove(0);
        let addresses = conf.addresses.clone();
        let backlog = conf.backlog;

        workers.push(std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
            let local = LocalSet::new();

            rt.block_on(local.run_until(async move {
                #[cfg(feature = "thread_shared_struct")]
                    let shared_data = shared_factory().await;

                // ACCEPTOR TASK
                for (address, port) in addresses {
                    let all_senders = all_senders.clone();
                    let matcher_acc = matcher.clone();
                    #[cfg(feature = "thread_shared_struct")]
                        let shared_acc = shared_data.clone();

                    tokio::task::spawn_local(async move {
                        let listener = setup_optimized_listener(&address, port, backlog);
                        let mut rr_index = worker_id;
                        let pool_size = all_senders.len();

                        loop {
                            if let Ok((stream, addr)) = listener.accept().await {
                                let target = rr_index % pool_size;
                                // Local-bypass to save on channel overhead
                                if target == worker_id {
                                    let m = matcher_acc.clone();
                                    #[cfg(feature = "thread_shared_struct")]
                                        let s = shared_acc.clone();
                                    tokio::task::spawn_local(async move {
                                        let conn = ConnectionStream::new(WaterStream::TOStream(stream), addr);
                                        #[cfg(feature = "thread_shared_struct")]
                                        serve_connection(conn, controller_ptr, s, m).await;
                                        #[cfg(not(feature = "thread_shared_struct"))]
                                        serve_connection(conn, controller_ptr, m).await;
                                    });
                                } else {
                                    let _ = all_senders[target].send((stream, addr)).await;
                                }
                                rr_index += 1;
                            }
                        }
                    });
                }

                // WORKER TASK
                while let Some((stream, addr)) = my_receiver.recv().await {
                    let m = matcher.clone();
                    #[cfg(feature = "thread_shared_struct")]
                        let s = shared_data.clone();

                    tokio::task::spawn_local(async move {
                        let connection = ConnectionStream::new(WaterStream::TOStream(stream), addr);
                        #[cfg(feature = "thread_shared_struct")]
                        serve_connection(connection, controller_ptr, s, m).await;
                        #[cfg(not(feature = "thread_shared_struct"))]
                        serve_connection(connection, controller_ptr, m).await;
                    });
                }
            }));
        }));
    }

    for w in workers { let _ = w.join(); }
}

fn setup_optimized_listener(addr: &str, port: u16, backlog: u32) -> tokio::net::TcpListener {
    let addr_str = format!("{}:{}", addr, port);
    let socket_addr = addr_str.to_socket_addrs().unwrap().next().unwrap();
    let socket = tokio::net::TcpSocket::new_v4().unwrap();
    socket.set_reuseaddr(true).ok();
    #[cfg(target_os = "linux")]
    socket.set_reuseport(true).ok();
    socket.bind(socket_addr).unwrap();
    socket.listen(backlog).expect("Listen failed")
}