

pub (crate) mod connection;
mod configurations;
mod tls;
mod sr_context;
pub  use sr_context::*;

#[doc(hidden)]
pub mod errors;
mod capsule;
mod encoding;
pub use encoding::*;

pub use capsule::*;

use std::io;
use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::Arc;
use tokio_rustls::TlsAcceptor;
#[cfg(feature = "debugging")]
use tracing::{debug};
pub use configurations::*;
use crate::server::connection::{ConnectionStream, WaterStream};

pub (crate) static mut STATIC_SERVER_CONFIGURATION:Option<ServerConfigurations> = None;
pub (crate) fn get_server_config()->&'static ServerConfigurations{
    unsafe  { STATIC_SERVER_CONFIGURATION.as_ref().unwrap() }
}


/// running given server configurations with Controller Root
pub async fn run_server<Holder:Send + 'static + std::fmt::Debug,const HS:usize,const QS:usize,>(
    config:ServerConfigurations,
    controller:&'static mut CapsuleWaterController<Holder,HS,QS>,
){
    unsafe  { STATIC_SERVER_CONFIGURATION = Some(config); }
    controller.set_up(String::new());
    let pointer = controller as *const CapsuleWaterController<Holder,HS,QS>;
    controller.____insure_binding();

    let controller = unsafe {pointer.as_ref().unwrap()};
    let conf = get_server_config();
    let mut workers = vec![];
    #[cfg(feature = "debugging")]
    let mut workers_count = 0_usize;




    for  address in &conf.addresses {
        workers.push(tokio::spawn(async move {
            #[cfg(feature = "debugging")]
            {
                debug!("listening on ip: {} port: {}",address.0,address.1);
                workers_count +=1;
                debug!("count of running workers {workers_count}");
            }

            let _ = run_server_with_address(address,controller).await;
        }));
    }
    for worker in workers {
        let _ = worker.await;
    }
}


async fn run_server_with_address<Holder:Send + 'static + std::fmt::Debug,const HS:usize,const QS:usize,>(
    (address,port):&(String,u16),
    controller:&'static  CapsuleWaterController<Holder,HS,QS>

)->io::Result<()>{
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
    socket.bind(socket_address).expect("can not bind to given address");
    let listener = socket.listen(
        server_config.backlog
    ).expect("");

    //


    // building tls acceptor
    let mut tls_acceptor:Option<TlsAcceptor> = None;
    if let Some(tls_config) = server_config.tls_certificate.as_ref() {
        let server_tls_config =
            tls::generate_tls_configurations(tls_config);
        if let Ok(server_tls_config ) = server_tls_config {
            tls_acceptor = Some(TlsAcceptor::from(Arc::new(server_tls_config)));
        }
    }
    let is_port_should_be_securely_handled=
        server_config.tls_ports.contains(port)
        && tls_acceptor.is_some();
    loop {
        if let Ok((stream,socket)) = listener.accept().await {
            let tls = tls_acceptor.clone();

            tokio::task::spawn(async move {
                // checking if the current port should be handled
                // with tls configurations if it`s exist
                if is_port_should_be_securely_handled {
                    let tls = tls.unwrap();
                    let tls_stream = tls.accept(stream).await;
                    if let Ok(tls_stream) = tls_stream {
                        let connection =  ConnectionStream::new(
                            WaterStream::TLS(tls_stream),
                            socket_address
                        );
                        serve_connection(connection,controller).await;
                    }
                    return ;
                }

                // handling connection normally
                let connection
                    = ConnectionStream::new(WaterStream::TOStream(stream),socket);
                serve_connection(connection,controller).await;
            });
        }
    }
}




async fn serve_connection<Holder:Send + 'static + std::fmt::Debug,
    const HS:usize,const QS:usize,>
(connection:ConnectionStream,
 controller:&'static  CapsuleWaterController<Holder,HS,QS>
){
    connection.serve(controller).await;
}
