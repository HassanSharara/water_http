// use std::collections::HashMap;
// use std::net::{SocketAddr, ToSocketAddrs};
// use tokio::task::LocalSet;
// use crate::server::{CapsuleWaterController, get_server_config, serve_connection, ServerConfigurations, STATIC_SERVER_CONFIGURATION};
// use crate::server::connection::{ConnectionStream, WaterStream};
// use crate::server::matcher::{DynamicPathVec, Matcher, MatcherInitializer, PathHolder};
//
// #[inline]
// pub fn serve_connections_by_channels<
//     #[cfg(feature = "use_tokio_send")] Holder: Send + 'static + std::fmt::Debug,
//     #[cfg(not(feature = "use_tokio_send"))] Holder,
//     #[cfg(all(feature = "thread_shared_struct", not(feature = "use_tokio_send")))] SHARED: Clone,
//     #[cfg(all(feature = "thread_shared_struct", feature = "use_tokio_send"))] SHARED: Clone + Send + 'static,
//     const HS: usize, const QS: usize,
// >(
//     config: ServerConfigurations,
//     #[cfg(feature = "thread_shared_struct")] controller: &'static mut CapsuleWaterController<Holder, SHARED, HS, QS>,
//     #[cfg(not(feature = "thread_shared_struct"))] controller: &'static mut CapsuleWaterController<Holder, HS, QS>,
//     #[cfg(all(feature = "thread_shared_struct", feature = "use_tokio_send"))] shared_factory: fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = SHARED> + Send>>,
//     #[cfg(all(feature = "thread_shared_struct", not(feature = "use_tokio_send")))] shared_factory: fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = SHARED>>>,
//     #[cfg(feature = "thread_shared_struct")] static_path: &'static mut Option<HashMap<String, PathHolder<Holder, SHARED, HS, QS>>>,
//     #[cfg(feature = "thread_shared_struct")] dynamic_path: &'static mut Option<HashMap<usize, DynamicPathVec<Holder, SHARED, HS, QS>>>,
//     #[cfg(not(feature = "thread_shared_struct"))] static_path: &'static mut Option<HashMap<String, PathHolder<Holder, HS, QS>>>,
//     #[cfg(not(feature = "thread_shared_struct"))] dynamic_path: &'static mut Option<HashMap<usize, DynamicPathVec<Holder, HS, QS>>>,
// ) {
//     // 1. BOOTSTRAP CONFIGURATION
//     unsafe { STATIC_SERVER_CONFIGURATION = Some(config); }
//     let conf = get_server_config();
//
//     controller.set_up(String::new());
//     controller.____insure_binding();
//
//     // 2. INITIALIZE MATCHER
//     *static_path = Some(HashMap::new());
//     *dynamic_path = Some(HashMap::new());
//     _ = MatcherInitializer::serialize(static_path.as_mut().unwrap(), dynamic_path.as_mut().unwrap(), controller);
//
//     #[cfg(feature = "thread_shared_struct")]
//         let controller_ptr: &'static CapsuleWaterController<Holder, SHARED, HS, QS> = unsafe { &*(controller as *const _) };
//     #[cfg(not(feature = "thread_shared_struct"))]
//         let controller_ptr: &'static CapsuleWaterController<Holder, HS, QS> = unsafe { &*(controller as *const _) };
//
//     let matcher = Matcher::new(static_path.as_ref().unwrap(), dynamic_path.as_ref().unwrap());
//
//     // 3. PRE-ALLOCATE CHANNELS
//     let mut channel_senders = Vec::with_capacity(conf.worker_threads_count);
//     let mut channel_receivers = Vec::with_capacity(conf.worker_threads_count);
//
//     for _ in 0..conf.worker_threads_count {
//         // High capacity (4096) ensures acceptor never blocks even during massive bursts
//         let (tx, rx) = tokio::sync::mpsc::channel::<(tokio::net::TcpStream, SocketAddr)>(4096);
//         channel_senders.push(tx);
//         channel_receivers.push(rx);
//     }
//
//     let mut workers = vec![];
//     let worker_count = conf.worker_threads_count;
//     let is_pow2 = worker_count > 0 && (worker_count & (worker_count - 1)) == 0;
//     let mask = worker_count.wrapping_sub(1);
//
//     // 4. SPAWN WORKER THREADS
//     for worker_id in 0..worker_count {
//         let matcher = matcher.clone();
//         let all_senders = channel_senders.clone();
//         let mut my_receiver = channel_receivers.remove(0);
//         let addresses = conf.addresses.clone();
//         let backlog = conf.backlog;
//
//         workers.push(std::thread::spawn(move || {
//             // OPTIONAL: core_affinity::set_for_current(...) would go here
//
//             let rt = tokio::runtime::Builder::new_current_thread()
//                 .enable_all()
//                 .build()
//                 .expect("Failed to build runtime");
//
//             let local = LocalSet::new();
//
//             rt.block_on(local.run_until(async move {
//                 #[cfg(feature = "thread_shared_struct")]
//                     let shared_data = shared_factory().await;
//
//                 // --- COMPONENT A: MULTI-ACCEPTOR MESH ---
//                 for (address, port) in addresses {
//                     let senders = all_senders.clone();
//                     let m_acc = matcher.clone();
//                     #[cfg(feature = "thread_shared_struct")]
//                         let s_acc = shared_data.clone();
//
//                     tokio::task::spawn_local(async move {
//                         let listener = setup_optimized_listener(&address, port, backlog);
//                         let mut rr_index = worker_id;
//
//                         loop {
//                             if let Ok((stream, addr)) = listener.accept().await {
//                                 // Bitwise RR is ~2x faster than Modulo
//                                 let target = if is_pow2 { rr_index & mask } else { rr_index % worker_count };
//
//                                 if target == worker_id {
//                                     // BYPASS: Internal task spawn (No channel overhead)
//                                     let m = m_acc.clone();
//                                     #[cfg(feature = "thread_shared_struct")]
//                                         let s = s_acc.clone();
//                                     tokio::task::spawn_local(async move {
//                                         let conn = ConnectionStream::new(WaterStream::TOStream(stream), addr);
//                                         #[cfg(feature = "thread_shared_struct")]
//                                         serve_connection(conn, controller_ptr, s, m).await;
//                                         #[cfg(not(feature = "thread_shared_struct"))]
//                                         serve_connection(conn, controller_ptr, m).await;
//                                     });
//                                 } else {
//                                     // OFFLOAD: Move to target thread's queue
//                                     let _ = senders[target].send((stream, addr)).await;
//                                 }
//                                 rr_index = rr_index.wrapping_add(1);
//                             }
//                         }
//                     });
//                 }
//
//                 // --- COMPONENT B: THE WORKER CONSUMER ---
//                 while let Some((stream, addr)) = my_receiver.recv().await {
//                     let m = matcher.clone();
//                     #[cfg(feature = "thread_shared_struct")]
//                         let s = shared_data.clone();
//
//                     tokio::task::spawn_local(async move {
//                         let connection = ConnectionStream::new(WaterStream::TOStream(stream), addr);
//                         #[cfg(feature = "thread_shared_struct")]
//                         serve_connection(connection, controller_ptr, s, m).await;
//                         #[cfg(not(feature = "thread_shared_struct"))]
//                         serve_connection(connection, controller_ptr, m).await;
//                     });
//                 }
//             }));
//         }));
//     }
//
//     for w in workers { let _ = w.join(); }
// }
//
// /// Configure socket with low-level Linux performance hints
// fn setup_optimized_listener(addr: &str, port: u16, backlog: u32) -> tokio::net::TcpListener {
//     let addr_str = format!("{}:{}", addr, port);
//     let socket_addr = addr_str.to_socket_addrs().unwrap().next().expect("Invalid Bind Addr");
//
//     let socket = if socket_addr.is_ipv4() {
//         tokio::net::TcpSocket::new_v4().unwrap()
//     } else {
//         tokio::net::TcpSocket::new_v6().unwrap()
//     };
//
//     socket.set_reuseaddr(true).ok();
//
//     #[cfg(target_os = "linux")]
//     socket.set_reuseport(true).ok(); // Enables true parallel accept
//
//     socket.bind(socket_addr).expect("Bind failed");
//
//     let listener = socket.listen(backlog).expect("Listen failed");
//
//     // TCP_NODELAY is usually desired for small HTTP responses
//     // but on the listener it ensures inherited sockets carry the flag.
//
//     listener
// }