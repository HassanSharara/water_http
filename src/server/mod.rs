

pub (crate) mod connection;
mod configurations;
mod tls;
mod sr_context;

use std::collections::HashMap;
#[cfg(feature = "thread_shared_struct")]
use std::future::Future;
pub  use sr_context::*;

#[doc(hidden)]
pub mod errors;
mod capsule;
#[cfg(feature = "auto_encode_response")]
mod encoding;
// pub(crate) mod io;

#[cfg(feature = "auto_encode_response")]
pub use encoding::*;

pub use capsule::*;

use std::io as stdio;
use std::net::{SocketAddr, ToSocketAddrs};
#[cfg(feature = "debugging")]
use std::ops::Deref;
#[cfg(feature = "cpu_affinity")]
use std::ops::DerefMut;
#[cfg(feature = "support_tls")]
use std::sync::{Arc};
#[cfg(not(feature = "use_tokio_send"))]
use tokio::task::LocalSet;
#[cfg(feature = "support_tls")]
use tokio_rustls::TlsAcceptor;
#[cfg(feature = "debugging")]
use tracing::{debug};
pub use configurations::*;
use crate::server::connection::{ConnectionStream, WaterStream};
use crate::server::matcher::{DynamicPathVec, Matcher, MatcherInitializer, PathHolder};

pub (crate) static mut STATIC_SERVER_CONFIGURATION:Option<ServerConfigurations> = None;
#[allow(static_mut_refs)]
pub (crate)  fn get_server_config()->&'static ServerConfigurations{
    unsafe  {
           STATIC_SERVER_CONFIGURATION.as_ref().unwrap()
    }
}


/// running given server configurations with Controller Root
pub  fn run_server<
    #[cfg(feature = "use_tokio_send")]
    Holder:Send + 'static + std::fmt::Debug,
    #[cfg(not(feature = "use_tokio_send"))]
    Holder,


    #[cfg(all(feature = "thread_shared_struct",not(feature = "use_tokio_send")))]
    SHARED:Clone,

    #[cfg(all(feature = "thread_shared_struct",feature = "use_tokio_send"))]
    SHARED:Clone + Send + 'static,
    const HS:usize,const QS:usize,

 >(
    config:ServerConfigurations,
    #[cfg(feature = "thread_shared_struct")]
    controller:&'static mut CapsuleWaterController<Holder,SHARED,HS,QS>,




    #[cfg(not(feature = "thread_shared_struct"))]
    controller:&'static mut CapsuleWaterController<Holder,HS,QS>,

    #[cfg(all(feature = "use_tokio_send",feature = "thread_shared_struct"))]
    shared_factory:fn()->std::pin::Pin<Box<dyn Future<Output=SHARED>+Send>>,
    #[cfg(all(feature = "thread_shared_struct",not(feature = "use_tokio_send")))]
    shared_factory:fn()->std::pin::Pin<Box<dyn Future<Output=SHARED>>>,

    #[cfg(feature = "thread_shared_struct")]
    static_path:&'static mut Option<HashMap<String,PathHolder<Holder,SHARED,HS,QS>>>,
    #[cfg(feature = "thread_shared_struct")]
    dynamic_path:&'static mut Option<HashMap<usize,DynamicPathVec<Holder,SHARED,HS,QS>>>,

    #[cfg(not(feature = "thread_shared_struct"))]
    static_path:&'static mut Option<HashMap<String,PathHolder<Holder,HS,QS>>>,
    #[cfg(not(feature = "thread_shared_struct"))]
    dynamic_path:&'static mut Option<HashMap<usize,DynamicPathVec<Holder,HS,QS>>>,
)
{
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

    let listener_count = conf.listeners_count;
    // 3. MULTI-THREADED (TOKIO SEND) ARCHITECTURE
    #[cfg(feature = "use_tokio_send")]
    {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .worker_threads(conf.worker_threads_count)
            .build()
            .unwrap();

        rt.block_on(async move {
            for address in &conf.addresses {
                let matcher = Matcher::new(static_path.as_ref().unwrap(), dynamic_path.as_ref().unwrap());
                let addr = address.clone();
                tokio::spawn(async move {
                    #[cfg(feature = "thread_shared_struct")]
                        let _ = run_server_with_address(&addr, controller_ptr, shared_factory, matcher).await;
                    #[cfg(not(feature = "thread_shared_struct"))]
                        let _ = run_server_with_address(&addr, controller_ptr, matcher).await;
                });
            }
            // Keep runtime alive
            std::future::pending::<()>().await;
        });
    }

    // 4. SILOED (LOCALSET / IO_URING) ARCHITECTURE
    #[cfg(not(feature = "use_tokio_send"))]
    {
        let mut os_threads = vec![];
        #[cfg(feature = "cpu_affinity")]
            let core_ids = if conf.core_affinity { core_affinity::get_core_ids() } else { None };

        // We spawn exactly worker_threads_count.
        // Each thread will bind to ALL addresses in the config via SO_REUSEPORT.
        for _i in 0..conf.worker_threads_count {
            let addresses = conf.addresses.clone();
            let matcher = Matcher::new(static_path.as_ref().unwrap(), dynamic_path.as_ref().unwrap());

            #[cfg(feature = "cpu_affinity")]
                let core_id = core_ids.as_ref().and_then(|ids| ids.get(_i % ids.len()).cloned());

            let thread = std::thread::spawn(move || {
                // Set CPU Affinity
                #[cfg(feature = "cpu_affinity")]
                if let Some(id) = core_id {
                    core_affinity::set_for_current(id);
                }

                // A: IO_URING PATH
                #[cfg(feature = "use_io_uring")]
                {
                    tokio_uring::start(async move {
                        for addr in addresses {
                            let matcher = matcher.clone();
                            tokio_uring::spawn(async move {
                                #[cfg(feature = "thread_shared_struct")]
                                    let _ = run_server_with_address(&addr, controller_ptr, shared_factory, matcher).await;
                                #[cfg(not(feature = "thread_shared_struct"))]
                                    let _ = run_server_with_address(&addr, controller_ptr, matcher).await;
                            });
                        }
                        std::future::pending::<()>().await;
                    });
                }

                // B: TOKIO LOCAL SET PATH
                #[cfg(not(feature = "use_io_uring"))]
                {


                    #[cfg(tokio_unstable)]
                    {
                        let rt = tokio::runtime::Builder::new_current_thread()
                            .enable_all()
                            .build_local(Default::default())
                            .unwrap();
                        rt.block_on(async move {
                            for addr in addresses {
                                let matcher = matcher.clone();
                                for _ in 0..listener_count {
                                    let addr = addr.clone();
                                    let matcher = matcher.clone();
                                    tokio::task::spawn_local(async move {
                                        #[cfg(feature = "thread_shared_struct")]
                                            let _ = run_server_with_address(&addr, controller_ptr, shared_factory, matcher).await;
                                        #[cfg(not(feature = "thread_shared_struct"))]
                                            let _ = crate::server::run_server_with_address(&addr, controller_ptr, matcher).await;
                                    });
                                }

                            }
                            std::future::pending::<()>().await;
                        });
                    }

                    #[cfg(not(tokio_unstable))]
                    {
                        let rt = tokio::runtime::Builder::new_current_thread()
                            .enable_all()
                            .build()
                            .unwrap();
                        let local = LocalSet::new();

                        rt.block_on(local.run_until(async move {
                            for addr in addresses {
                                let matcher = matcher.clone();
                                for _ in 0..listener_count {
                                    let matcher = matcher.clone();
                                    let addr = addr.clone();
                                    tokio::task::spawn_local(async move {
                                        #[cfg(feature = "thread_shared_struct")]
                                            let _ = run_server_with_address(&addr, controller_ptr, shared_factory, matcher).await;
                                        #[cfg(not(feature = "thread_shared_struct"))]
                                            let _ = run_server_with_address(&addr, controller_ptr, matcher).await;
                                    });
                                }
                            }
                            std::future::pending::<()>().await;
                        }));
                    }


                }
            });
            os_threads.push(thread);
        }

        for thread in os_threads {
            let _ = thread.join();
        }
    }
}
// {
//     unsafe  { STATIC_SERVER_CONFIGURATION = Some(config); }
//     controller.set_up(String::new());
//     #[cfg(not(feature = "thread_shared_struct"))]
//     let pointer = controller as *const CapsuleWaterController<Holder,HS,QS>;
//
//
//     *static_path = Some(HashMap::new());
//     *dynamic_path = Some(HashMap::new());
//
//
//
//     #[cfg(feature = "thread_shared_struct")]
//     let pointer = controller as *const CapsuleWaterController<Holder,SHARED,HS,QS>;
//     controller.____insure_binding();
//     let sp = static_path.as_mut().unwrap();
//     let dp = dynamic_path.as_mut().unwrap();
//
//     _=MatcherInitializer::serialize(sp,dp,controller);
//
//
//
//     let controller = unsafe {pointer.as_ref().unwrap()};
//     let conf = get_server_config();
//
//     #[cfg(feature = "use_tokio_send")]
//     {
//         #[cfg(feature = "debugging")]
//             let mut workers_count = 0_usize;
//
//         let rt = tokio::runtime::Builder::new_multi_thread()
//             .enable_all()
//             .worker_threads(conf.worker_threads_count * 2)
//             .build()
//             .unwrap();
//         let mut workers = vec![];
//         for _ in 0..conf.worker_threads_count {
//
//             for  address in conf.addresses.clone() {
//                 let matcher = Matcher::new(static_path.as_ref().unwrap(),dynamic_path.as_ref().unwrap());
//                 workers.push(
//                     rt.spawn(async move {
//                         #[cfg(feature = "debugging")]
//                         {
//                             debug!("listening on ip: {} port: {}",address.0,address.1);
//                             workers_count +=1;
//                             debug!("count of running workers {workers_count}");
//                         }
//
//                         #[cfg(feature = "thread_shared_struct")]
//                         {
//                             _=run_server_with_address(&address, controller,shared_factory,matcher).await;
//                         }
//
//                         #[cfg(not(feature = "thread_shared_struct"))]
//                         {
//                             _= crate::server::run_server_with_address(&address, controller, matcher).await;
//                         }
//                     })
//                 );
//             }
//         }
//
//         rt.block_on(async move {
//             for worker in workers {
//                 let _ = worker.await;
//             }
//         });
//     }
//
//     #[cfg(not(feature = "use_tokio_send"))]
//     {
//         let mut os_threads = vec![];
//
//             #[cfg(feature = "cpu_affinity")]
//             let cores = std::sync::Arc::new(
//             if conf.core_affinity  {
//                 core_affinity::get_core_ids()
//             } else {
//                 None
//             }
//         );
//         #[cfg(feature = "cpu_affinity")]
//         let  core_index = std::sync::Arc::new(std::sync::Mutex::new(0_usize));
//         for _ in 0..conf.worker_threads_count {
//             for address in &conf.addresses {
//                 let address = address.clone();
//                 let controller = controller;
//                 let matcher = Matcher::new(static_path.as_ref().unwrap(),dynamic_path.as_ref().unwrap());
//                 #[cfg(feature = "use_io_uring")]
//                 {
//
//                     let thread = std::thread::spawn(move || {
//                         tokio_uring::start(async move {
//                             #[cfg(feature = "thread_shared_struct")]
//                                 let _ = crate::server::run_server_with_address(&address, controller,shared_factory,matcher).await;
//
//                             #[cfg(not(feature = "thread_shared_struct"))]
//                                 let _ = crate::server::run_server_with_address(&address, controller,matcher).await;
//                         });
//                     });
//                     os_threads.push(thread);
//
//                 }
//                 #[cfg(not(feature = "use_io_uring"))]
//                 {
//                     #[cfg(feature = "cpu_affinity")]
//                     let core_index = core_index.clone();
//                     #[cfg(feature = "cpu_affinity")]
//                         let  value = cores.clone();
//                     let thread = std::thread::spawn(move || {
//                         #[cfg(feature = "cpu_affinity")]
//                         if let Some(core) = value.as_ref() {
//                            let mut core_index_guard = core_index.lock();
//                            let core_index = core_index_guard.as_mut().unwrap();
//                            let m = core_index.deref_mut();
//                            core_affinity::set_for_current((core[*m]).clone());
//                            if *m + 1 >= core.len() {
//                                *m = 0;
//                            } else {
//                                *m+=1;
//                            }
//                        }
//                         let rt = tokio::runtime::Builder::new_current_thread()
//                             .enable_all()
//                             .max_blocking_threads(90)
//                             .build()
//                             .unwrap();
//
//                         let local = LocalSet::new();
//
//                         rt.block_on(async {
//                             local.run_until(async {
//                                 #[cfg(feature = "thread_shared_struct")]
//                                     let _ = run_server_with_address(&address, controller,shared_factory,matcher).await;
//
//                                 #[cfg(not(feature = "thread_shared_struct"))]
//                                     let _ = run_server_with_address(&address, controller,matcher).await;
//                             }).await;
//                         });
//                     });
//                     os_threads.push(thread);
//                 }
//
//             }
//         }
//         for thread in os_threads {
//             let _ = thread.join();
//         }
//     }
//
// }


#[cfg(feature = "use_io_uring")]
fn create_tokio_uring_listener(address: &str, port: &u16, backlog: u32) -> tokio_uring::net::TcpListener {
    let ad = format!("{address}:{port}");
    let listener = tokio_uring::net::TcpListener::bind(ad.parse().unwrap());
    listener.unwrap()
    // use tokio_uring::net::TcpListener;
    // use std::net::TcpListener as StdListener;
    // // Resolve address
    // let address_string = format!("{}:{}", address, port);
    // let socket_address: SocketAddr = address_string
    //     .to_socket_addrs()
    //     .unwrap()
    //     .next()
    //     .expect("error while parsing address");
    //
    // // Create std listener
    // let std_listener = match socket_address {
    //     SocketAddr::V4(_) => StdListener::bind(socket_address).expect("bind failed"),
    //     SocketAddr::V6(_) => StdListener::bind(socket_address).expect("bind failed"),
    // };
    //
    // // Configure options
    // std_listener
    //     .set_nonblocking(true)
    //     .expect("cannot set non-blocking");
    //
    // std_listener
    //     .set_reuse_address(true)
    //     .expect("cannot set reuse address");
    // #[cfg(target_os = "linux")]
    // std_listener
    //     .set_reuse_port(true)
    //     .expect("cannot set reuse port");
    //
    // // backlog is handled by bind+listen in std_listener
    // // On Rust 1.70+ TcpListener::bind already handles backlog internally
    //
    // // Convert to tokio-uring listener
    // TcpListener::from_std(std_listener).expect("cannot convert to tokio-uring listener")
}

#[inline(always)]
async fn run_server_with_address<
    #[cfg(feature = "use_tokio_send")]
    Holder:Send + 'static + std::fmt::Debug,

    #[cfg(not(feature = "use_tokio_send"))]
    Holder,



    #[cfg(all(feature = "thread_shared_struct",not(feature = "use_tokio_send")))]
    SHARED:Clone,

    #[cfg(all(feature = "thread_shared_struct",feature = "use_tokio_send"))]
    SHARED:Clone + Send + 'static,
    const HS:usize,const QS:usize,>(
    (address,port):&(String,u16),
    #[cfg(feature = "thread_shared_struct")]
    controller:&'static  CapsuleWaterController<Holder,SHARED,HS,QS>,
    #[cfg(not(feature = "thread_shared_struct"))]
    controller:&'static  CapsuleWaterController<Holder,HS,QS>,
    #[cfg(all(feature = "use_tokio_send",feature = "thread_shared_struct"))]
    shared_factory:fn()->std::pin::Pin<Box<dyn Future<Output=SHARED>+Send>>,
    #[cfg(all(feature = "thread_shared_struct",not(feature = "use_tokio_send")))]
    shared_factory:fn()->std::pin::Pin<Box<dyn Future<Output=SHARED>>>,

    #[cfg(feature = "thread_shared_struct")]
    matcher:Matcher<Holder,SHARED,HS,QS>,
    #[cfg(not(feature = "thread_shared_struct"))]
    matcher:Matcher<Holder,HS,QS>
)->stdio::Result<()>
{
    let server_config = get_server_config();

    // 1. Listener Initialization
    #[cfg(feature = "use_io_uring")]
        let listener = create_tokio_uring_listener(address, port, server_config.backlog);

    #[cfg(not(feature = "use_io_uring"))]
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
        socket.listen(server_config.backlog).expect("Listen failed")
    };

    // 2. Resource & Security Setup
    #[cfg(feature = "thread_shared_struct")]
        let shared_struct = shared_factory().await;

    #[cfg(feature = "support_tls")]
        let (tls_acceptor, is_secure) = {
        let mut acceptor = None;
        if let Some(cfg) = server_config.tls_certificate.as_ref() {
            if let Ok(gen) = tls::generate_tls_configurations(cfg) {
                acceptor = Some(TlsAcceptor::from(Arc::new(gen)));
            }
        }
        (acceptor, server_config.tls_ports.contains(port))
    };

    // 3. High-Performance Debugging Counter
    #[cfg(feature = "debugging")]
        let connections_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));

    // 4. Optimized Accept Loop
    // let mut accept_batch: u32 = 0;

    loop {
        match listener.accept().await {
            Ok((stream, socket_addr)) => {
                #[cfg(feature = "debugging")]
                connections_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

                // Clone shared handles for the spawned task
                let matcher = matcher.clone();
                #[cfg(feature = "thread_shared_struct")]
                    let shared_struct = shared_struct.clone();
                #[cfg(feature = "support_tls")]
                    let tls_acceptor = tls_acceptor.clone();
                #[cfg(feature = "debugging")]
                    let connections_count = connections_count.clone();

                let handler_future = async move {
                    #[cfg(feature = "support_tls")]
                    if is_secure {
                        if let Some(acc) = tls_acceptor {
                            if let Ok(tls_stream) = acc.accept(stream).await {
                                let connection = ConnectionStream::new(WaterStream::TLS(tls_stream), socket_addr);
                                #[cfg(feature = "thread_shared_struct")]
                                serve_connection(connection, controller, shared_struct, matcher).await;
                                #[cfg(not(feature = "thread_shared_struct"))]
                                serve_connection(connection, controller, matcher).await;
                            }
                        }
                        #[cfg(feature = "debugging")]
                        connections_count.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
                        return;
                    }

                    // Plain HTTP path
                    let connection = ConnectionStream::new(WaterStream::TOStream(stream), socket_addr);
                    #[cfg(feature = "thread_shared_struct")]
                    serve_connection(connection, controller, shared_struct, matcher).await;
                    #[cfg(not(feature = "thread_shared_struct"))]
                    serve_connection(connection, controller, matcher).await;

                    #[cfg(feature = "debugging")]
                    connections_count.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
                };

                // 5. Context-Aware Spawning
                #[cfg(feature = "use_tokio_send")]
                tokio::spawn(handler_future);

                #[cfg(not(feature = "use_tokio_send"))]
                {
                    #[cfg(feature = "use_io_uring")]
                    tokio_uring::spawn(handler_future);
                    #[cfg(not(feature = "use_io_uring"))]
                    tokio::task::spawn_local(handler_future);
                }

                // 6. Cooperative Yielding (Batching)
                // Prevents the accept loop from starving processing tasks under heavy load
                // accept_batch += 1;
                // if accept_batch >= 256 {
                //     accept_batch = 0;
                //     tokio::task::yield_now().await;
                // }
            }
            Err(_) => {
                // Yield on errors to prevent a hot-loop (e.g. EMFILE)
                // tokio::task::yield_now().await;
            }
        }
    }
}



#[inline(always)]
async fn serve_connection<
    #[cfg(feature = "use_tokio_send")]
    Holder:Send + 'static,

    #[cfg(all(feature = "thread_shared_struct",not(feature = "use_tokio_send")))]
    SHARED:Clone,


    #[cfg(all(feature = "thread_shared_struct",feature = "use_tokio_send"))]
    SHARED:Clone + Send + 'static,
    #[cfg(not(feature = "use_tokio_send"))]
    Holder,
    const HS:usize,const QS:usize,>
(connection:ConnectionStream,
 #[cfg(feature = "thread_shared_struct")]
 controller:&'static  CapsuleWaterController<Holder,SHARED,HS,QS>,
 #[cfg(not(feature = "thread_shared_struct"))]
 controller:&'static  CapsuleWaterController<Holder,HS,QS>,
 #[cfg(feature = "thread_shared_struct")]
 shared_factory:SHARED,
 #[cfg(feature = "thread_shared_struct")]
 matcher:Matcher<Holder,SHARED,HS,QS>,
 #[cfg(not(feature = "thread_shared_struct"))]
 matcher:Matcher<Holder,HS,QS>
){
    #[cfg(feature = "thread_shared_struct")]
    connection.serve(controller,shared_factory.clone(),matcher).await;
    #[cfg(not(feature = "thread_shared_struct"))]
    connection.serve(controller,matcher).await;
}
