//
//
// pub (crate) mod connection;
// mod configurations;
// mod tls;
// mod sr_context;
// pub  use sr_context::*;
//
// #[doc(hidden)]
// pub mod errors;
// mod capsule;
// mod encoding;
// pub use encoding::*;
//
// pub use capsule::*;
//
// use std::io;
// use std::net::{SocketAddr, ToSocketAddrs};
// #[cfg(feature = "debugging")]
// use std::ops::Deref;
// #[cfg(feature = "support_tls")]
// use std::sync::{Arc};
// use tokio::runtime::Builder;
// use tokio::task::LocalSet;
// #[cfg(feature = "support_tls")]
// use tokio_rustls::TlsAcceptor;
// #[cfg(feature = "debugging")]
// use tracing::{debug};
// pub use configurations::*;
// use crate::server::connection::{ConnectionStream, WaterStream};
//
// pub (crate) static mut STATIC_SERVER_CONFIGURATION:Option<ServerConfigurations> = None;
// #[allow(static_mut_refs)]
// pub (crate)  fn get_server_config()->&'static ServerConfigurations{
//     unsafe  {
//         STATIC_SERVER_CONFIGURATION.as_ref().unwrap()
//     }
// }
//
//
// /// running given server configurations with Controller Root
// pub async fn run_server<Holder: Send + 'static + std::fmt::Debug, const HS: usize, const QS: usize>(
//     config: ServerConfigurations,
//     controller: &'static mut CapsuleWaterController<Holder, HS, QS>,
// ) {
//     // initialize global config (you may want to replace static mut with OnceCell in future)
//     unsafe { STATIC_SERVER_CONFIGURATION = Some(config); }
//     controller.set_up(String::new());
//     let pointer = controller as *const CapsuleWaterController<Holder, HS, QS>;
//     controller.____insure_binding();
//
//     // controller ref that is safe to read (we only pass as &'static)
//     let controller = unsafe { pointer.as_ref().unwrap() };
//
//     let conf = get_server_config();
//
//     // spawn one OS thread per address; each thread runs its own current-thread runtime + LocalSet
//     let mut handles = Vec::with_capacity(conf.addresses.len());
//     for address in conf.addresses.clone() {
//         // make static clones of controller pointer and address for the thread
//         let controller_arc = std::sync::Arc::new(StaticPtr(controller as *const CapsuleWaterController<Holder, HS, QS>));
//         let addr = address.clone();
//
//         let handle = std::thread::Builder::new()
//             .name(format!("listener-{}:{}", addr.0, addr.1))
//             .spawn({
//                 let controller_arc = controller_arc.clone();
//                 move || {
//                     let rt = Builder::new_current_thread().enable_all().build().unwrap();
//                     rt.block_on(async move {
//                         let controller_ref: &'static CapsuleWaterController<Holder, HS, QS> =
//                             unsafe { (*controller_arc).0.as_ref().unwrap() };
//                         let _ = run_server_with_address_in_thread(&addr, controller_ref).await;
//                     });
//                 }
//             })
//             .unwrap();
//
//         handles.push(handle);
//     }
//
//     // join all listener threads (this function is async but blocking join is acceptable here
//     // because we want to keep the server alive; alternatively you can detach threads as desired)
//     for h in handles {
//         let _ = h.join();
//     }
// }
// #[derive(Clone, Copy)]
// struct StaticPtr<T>(*const T);
//
// unsafe impl<T> Send for StaticPtr<T> {}
// unsafe impl<T> Sync for StaticPtr<T> {}
// /// This function is the async body that will be executed inside *the thread's runtime*
// /// and will create and run a LocalSet. It must be executed only from the thread that owns it.
// async fn run_server_with_address_in_thread<Holder: Send + 'static + std::fmt::Debug,
//     const HS: usize, const QS: usize>(
//     (address, port): &(String, u16),
//     controller: &'static CapsuleWaterController<Holder, HS, QS>,
// ) -> io::Result<()> {
//     // fetch server config
//     let server_config = get_server_config();
//
//     // bind inside this thread (we recreate socket here to ensure it is owned by this thread)
//     let address_string = format!("{}:{}", address, port);
//     let socket_address = (&address_string)
//         .to_socket_addrs()
//         .unwrap()
//         .next()
//         .expect("error while parsing address");
//
//     // create std listener with desired socket options (you can use socket2 if you need reuseport)
//     // We'll use tokio::net::TcpSocket here but create it inside the thread, it's fine.
//     let socket = match &socket_address {
//         SocketAddr::V4(_) => tokio::net::TcpSocket::new_v4(),
//         SocketAddr::V6(_) => tokio::net::TcpSocket::new_v6(),
//     }
//         .expect("can not create tcp socket from given address");
//
//     socket.set_reuseaddr(true).ok();
//     socket.set_nodelay(true).ok();
//     #[cfg(target_os = "linux")]
//     socket.set_reuseport(true).ok();
//     socket.bind(socket_address).expect("can not bind to given address");
//     let listener = socket
//         .listen(server_config.backlog)
//         .expect("tcp listen failed");
//
//     // build TLS acceptor if needed (clone inside thread)
//     #[cfg(feature = "support_tls")]
//         let mut tls_acceptor: Option<TlsAcceptor> = None;
//     #[cfg(feature = "support_tls")]
//     if let Some(tls_config) = server_config.tls_certificate.as_ref() {
//         if let Ok(server_tls_config) = tls::generate_tls_configurations(tls_config) {
//             tls_acceptor = Some(TlsAcceptor::from(Arc::new(server_tls_config)));
//         }
//     }
//     #[cfg(feature = "support_tls")]
//         let is_port_secure = server_config.tls_ports.contains(port) && tls_acceptor.is_some();
//
//     // Debugging counter - prefer AtomicUsize for light-weight counting (no await)
//     #[cfg(feature = "debugging")]
//     use std::sync::atomic::{AtomicUsize, Ordering};
//     #[cfg(feature = "debugging")]
//         let connections_count = Arc::new(AtomicUsize::new(0));
//
//     // create a LocalSet for this thread and run it
//     let local = LocalSet::new();
//     local
//         .run_until(async  {
//             loop {
//                 #[cfg(feature = "debugging")]
//                     let connections_count = connections_count.clone();
//
//                 // wait for a connection
//                 match listener.accept().await {
//                     Ok((stream, socket_addr)) => {
//                         #[cfg(feature = "debugging")]
//                         {
//                             // atomic increment
//                             connections_count.fetch_add(1, Ordering::Relaxed);
//                         }
//
//                         #[cfg(feature = "support_tls")]
//                             let tls = tls_acceptor.clone();
//
//                         // spawn the per-connection task as local (runs on same thread / LocalSet)
//                         local.spawn_local({
//                             // clone data for the async move block
//                             let controller = controller;
//                             let socket_address = socket_addr;
//                             async move {
//                                 // TLS path
//                                 #[cfg(feature = "support_tls")]
//                                 {
//                                     if is_port_secure {
//                                         if let Some(tls) = tls {
//                                             match tls.accept(stream).await {
//                                                 Ok(tls_stream) => {
//                                                     let connection = ConnectionStream::new(
//                                                         WaterStream::TLS(tls_stream),
//                                                         socket_address,
//                                                     );
//                                                     serve_connection(connection, controller).await;
//                                                 }
//                                                 Err(_e) => {
//                                                     // handle TLS accept error if needed
//                                                 }
//                                             }
//                                         }
//                                         #[cfg(feature = "debugging")]
//                                         {
//                                             // decrement counter
//                                             // (Atomic used, so no await)
//                                             // If you prefer logging, do it here.
//                                         }
//                                         return;
//                                     }
//                                 }
//
//                                 // Plain path
//                                 let connection =
//                                     ConnectionStream::new(WaterStream::TOStream(stream), socket_address);
//                                 serve_connection(connection, controller).await;
//
//                                 #[cfg(feature = "debugging")]
//                                 {
//                                     connections_count.fetch_sub(1, Ordering::Relaxed);
//                                 }
//                             }
//                         });
//                     }
//                     Err(e) => {
//                         // accept error; log or sleep a bit to avoid hot loop
//                         eprintln!("accept failed: {:?}", e);
//                         tokio::time::sleep(std::time::Duration::from_millis(10)).await;
//                     }
//                 }
//             }
//         })
//         .await;
//
//     Ok(())
// }
//
// #[inline(always)]
// async fn serve_connection<Holder: Send + 'static + std::fmt::Debug,
//     const HS: usize, const QS: usize,>(
//     connection: ConnectionStream,
//     controller: &'static CapsuleWaterController<Holder, HS, QS>,
// ) {
//     connection.serve(controller).await;
// }



pub (crate) mod connection;
mod configurations;
mod tls;
mod sr_context;
pub  use sr_context::*;

#[doc(hidden)]
pub mod errors;
mod capsule;
mod encoding;
mod io;

pub(crate) use io::*;
pub use encoding::*;

pub use capsule::*;

use std::io as stdio;
use std::net::{SocketAddr, ToSocketAddrs};
#[cfg(feature = "debugging")]
use std::ops::Deref;
#[cfg(feature = "support_tls")]
use std::sync::{Arc};
use tokio::task::LocalSet;
#[cfg(feature = "support_tls")]
use tokio_rustls::TlsAcceptor;
#[cfg(feature = "debugging")]
use tracing::{debug};
pub use configurations::*;
use crate::server::connection::{ConnectionStream, WaterStream};

pub (crate) static mut STATIC_SERVER_CONFIGURATION:Option<ServerConfigurations> = None;
#[allow(static_mut_refs)]
pub (crate)  fn get_server_config()->&'static ServerConfigurations{
    unsafe  {
           STATIC_SERVER_CONFIGURATION.as_ref().unwrap()
    }
}


/// running given server configurations with Controller Root
pub  fn run_server<Holder:Send + 'static + std::fmt::Debug,const HS:usize,const QS:usize,>(
    config:ServerConfigurations,
    controller:&'static mut CapsuleWaterController<Holder,HS,QS>,
){
    unsafe  { STATIC_SERVER_CONFIGURATION = Some(config); }
    controller.set_up(String::new());
    let pointer = controller as *const CapsuleWaterController<Holder,HS,QS>;
    controller.____insure_binding();

    let controller = unsafe {pointer.as_ref().unwrap()};
    let conf = get_server_config();


    #[cfg(feature = "use_tokio_send")]
    {
        #[cfg(feature = "debugging")]
            let mut workers_count = 0_usize;

        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .worker_threads(conf.worker_threads_count * 2)
            .build()
            .unwrap();
        let mut workers = vec![];
        for _ in 0..conf.worker_threads_count {

            for  address in conf.addresses.clone() {
                workers.push(
                    rt.spawn(async move {
                        #[cfg(feature = "debugging")]
                        {
                            debug!("listening on ip: {} port: {}",address.0,address.1);
                            workers_count +=1;
                            debug!("count of running workers {workers_count}");
                        }

                        let _ = crate::server::run_server_with_address(&address, controller).await;
                    })
                );
            }
        }

        rt.block_on(async move {
            for worker in workers {
                let _ = worker.await;
            }
        });
        return;
    }
    let mut os_threads = vec![];
    for tid in 0..conf.worker_threads_count {
        for address in &conf.addresses {
            let address = address.clone();
            let controller = controller;
            let threads = std::thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .max_blocking_threads(90)
                    .build()
                    .unwrap();

                let local = LocalSet::new();

                local.block_on(&rt, async move {
                    let _ = run_server_with_address(&address,controller).await;
                });
            });
            os_threads.push(threads);

        }
    }
    for thread in os_threads {
        let _ = thread.join();
    }

}


async fn run_server_with_address<Holder:Send + 'static + std::fmt::Debug,const HS:usize,const QS:usize,>(
    (address,port):&(String,u16),
    controller:&'static  CapsuleWaterController<Holder,HS,QS>

)->stdio::Result<()>{
    // defining configuration object
    let server_config = get_server_config();


    // building tcp listener with defined backlog
    let address_string = format!("{}:{}",address,port);
    let socket_address = (&address_string).to_socket_addrs()
        .unwrap().next()
        .expect("error while parsing address");
    let socket = match &socket_address {
        SocketAddr::V4(_) => { tokio::net::TcpSocket::new_v4()}
        SocketAddr::V6(_) => {tokio::net::TcpSocket::new_v6()}
    }.expect("can not create tcp socket from given address");
    socket.set_reuseaddr(true).expect("can not set reuse address");
    socket.set_nodelay(true).expect("");
    #[cfg(target_os = "linux")]
    socket.set_reuseport(true).expect("could not reuse port on linux");
    socket.bind(socket_address).expect("can not bind to given address");
    let listener = socket.listen(
        server_config.backlog
    ).expect("");

    //


    // building tls acceptor
    #[cfg(feature = "support_tls")]
    let mut tls_acceptor:Option<TlsAcceptor> = None;
    #[cfg(feature = "support_tls")]
    if let Some(tls_config) = server_config.tls_certificate.as_ref() {
        let server_tls_config =
            tls::generate_tls_configurations(tls_config);
        if let Ok(server_tls_config ) = server_tls_config {
            tls_acceptor = Some(TlsAcceptor::from(Arc::new(server_tls_config)));
        }
    }

    #[cfg(feature = "support_tls")]
    let is_port_should_be_securely_handled=
        server_config.tls_ports.contains(port)
        && tls_acceptor.is_some();


    #[cfg(feature = "debugging")]
    use std::ops::DerefMut;
    #[cfg(feature = "debugging")]
    let  connections_count = Arc::new(tokio::sync::Mutex::new(0_usize));



    loop {
        #[cfg(feature = "debugging")]
        let connections_count = connections_count.clone();
        if let Ok((stream,socket)) = listener.accept().await {
            #[cfg(feature = "debugging")]
            {
                let mut con = connections_count.lock().await;
                let m = con.deref_mut();
                *m +=1;
            }
            #[cfg(feature = "support_tls")]
            let tls = tls_acceptor.clone();

            #[cfg(feature = "use_tokio_send")]
            {
                tokio::spawn(async move {
                    // checking if the current port should be handled
                    // with tls configurations if it`s exist
                    #[cfg(feature = "support_tls")]
                    {
                        if is_port_should_be_securely_handled {
                            let tls = tls.unwrap();
                            let tls_stream = tls.accept(stream).await;
                             if let Ok(tls_stream) = tls_stream {
                                let connection =  ConnectionStream::new(
                                    WaterStream::TLS(tls_stream),
                                    socket_address
                                );
                                crate::server::serve_connection(connection, controller).await;
                            }
                            #[cfg(feature = "debugging")]
                            {
                                let mut con = connections_count.lock().await;
                                debug!("last connections count where port is not secure {:?}",con.deref());
                                let m = con.deref_mut();
                                if *m == 1 {
                                    *m = 0;
                                } else {
                                    *m -=1;
                                }

                            }
                            return ;
                        }
                    }

                    // handling connection normally
                    let connection
                        = ConnectionStream::new(WaterStream::TOStream(stream),socket);
                    crate::server::serve_connection(connection, controller).await;
                    #[cfg(feature = "debugging")]
                    {
                        let mut con = connections_count.lock().await;
                        debug!("last connections count {:?}",con.deref());
                        let m = con.deref_mut();
                        if *m == 1 {
                            *m = 0;
                        } else {
                            *m -=1;
                        }

                    }
                });

                continue;
            }

            tokio::task::spawn_local(async move {
                // checking if the current port should be handled
                // with tls configurations if it`s exist
                #[cfg(feature = "support_tls")]
                {
                    if is_port_should_be_securely_handled {
                        let tls = tls.unwrap();
                        let tls_stream = tls.accept(stream).await;
                        if let Ok(tls_stream) = tls_stream {
                            let connection =  ConnectionStream::new(
                                WaterStream::TLS(tls_stream),
                                socket_address
                            );
                            serve_connection(connection, controller).await;
                        }
                        #[cfg(feature = "debugging")]
                        {
                            let mut con = connections_count.lock().await;
                            debug!("last connections count where port is not secure {:?}",con.deref());
                            let m = con.deref_mut();
                            if *m == 1 {
                                *m = 0;
                            } else {
                                *m -=1;
                            }

                        }
                        return ;
                    }
                }

                // handling connection normally
                let connection
                    = ConnectionStream::new(WaterStream::TOStream(stream),socket);
                crate::server::serve_connection(connection, controller).await;
                #[cfg(feature = "debugging")]
                {
                    let mut con = connections_count.lock().await;
                    debug!("last connections count {:?}",con.deref());
                    let m = con.deref_mut();
                    if *m == 1 {
                        *m = 0;
                    } else {
                        *m -=1;
                    }

                }
            });

        }
    }
}



#[inline(always)]
async fn serve_connection<Holder:Send + 'static + std::fmt::Debug,
    const HS:usize,const QS:usize,>
(connection:ConnectionStream,
 controller:&'static  CapsuleWaterController<Holder,HS,QS>
){
    connection.serve(controller).await;
}
