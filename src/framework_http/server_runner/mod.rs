    use std::net::{IpAddr, ToSocketAddrs};
    use std::net::SocketAddr;
    use std::sync::Arc;
    use  crate::framework_http::*;
    use h2::server;
    use tokio::net::TcpStream;
    use tokio_rustls::server::TlsStream;
    use tokio_rustls::TlsAcceptor;
    use crate::configurations::WaterIpAddressesRestriction;
    use crate::structure::{WaterCapsuleController,context_route_function_finder};



    /// running server and listen to given ports and
    /// also initializing give controllers would
    /// be done only by this function ,
    /// and also it would be an auto used by our macros
    pub async  fn start_server<DataHolderGeneric>(
        configurations:WaterServerConfigurations,
        controllers:fn () -> &'static mut Vec<WaterCapsuleController<DataHolderGeneric>>
    )

        where DataHolderGeneric : Send{

        unsafe {
            crate::___ROUTERS = Some(HashMap::new());
            ___SERVER_CONFIGURATIONS = Some(configurations);
        }
        let configurations:&'static WaterServerConfigurations = unsafe {
            ___SERVER_CONFIGURATIONS.as_ref().unwrap()
        };

        for controller in controllers() {
            controller.____insure_binding();
        }
        unsafe {
          if let Some(_) = crate::___ROUTERS.as_ref() {
              for controller in controllers() {
                  controller.___after_insure_binding_build_router_map();
              }
          }
        };
        let mut tls_acceptor:Option<TlsAcceptor> = None;
        if let Some(tls_config) = configurations.tls_certificate.as_ref() {
            let server_tls_config = tls::generate_tls_configurations(tls_config);
            if let Ok(server_tls_config ) = server_tls_config {
                tls_acceptor = Some(TlsAcceptor::from(Arc::new(server_tls_config)));
            }
        }
        let mut workers = vec![];
        for (address,port) in &configurations.addresses {
            let acceptor = tls_acceptor.clone();
               workers.push(tcp_connections_threads_generator::<DataHolderGeneric>(
                   (address,port),
                   controllers(),
                   acceptor,
                   configurations
               ));
            }
        for worker in workers {
            let _ = worker.await;
        }
        }



     async fn tcp_connections_threads_generator<DataHolderGeneric>(
        (address,port):(&str,&u16),
        controllers:&'static Vec<WaterCapsuleController<DataHolderGeneric>>,
        tls_acceptor: Option<TlsAcceptor>,
        server_configurations:&'static WaterServerConfigurations
    )
        where DataHolderGeneric : Send{
        let address = format!("{address}:{port}");
        let socket_address = (&address).to_socket_addrs()
            .unwrap().next()
            .expect("error while parsing address");

        let socket = match &socket_address {
            SocketAddr::V4(_) => { tokio::net::TcpSocket::new_v4()}
            SocketAddr::V6(_) => {tokio::net::TcpSocket::new_v6()}
        }.expect("can not create tcp socket from given address");
        socket.set_reuseaddr(true).expect("can not set reuse address");
        socket.set_nodelay(true).expect("");
        socket.bind(socket_address).expect("can not bind to given address");
        let containing_port = server_configurations.tls_ports.contains(port);
        let listener = socket.listen(1028).expect("");

        // let listener = listener_arc.clone();
        // let tls_acceptor = tls_acceptor.clone();
        // let output = local_thread_pool.spawn_pinned( move || {
        //     async move {
        //         tokio::task::spawn_local( async move {
        //             loop {
        //                 let stream = listener.accept().await;
        //                 println!("accepting connection from {}",_w+1);
        //
        //             }
        //         }).await
        //     }
        // });



        // ----------------- worker start
        // let mut threads_pool = vec![];
        //
        // for _i in 0..4 {
        //     let stream_arc = arc_listener.clone();
        //     let tls_acceptor = tls_acceptor.clone();
        //     let join_handler = tokio::spawn(async move {
        //         let stream = stream_arc;
        //           println!("thread number {_i} start listening");
        //             while let Ok( (mut stream,socket)) = stream.accept().await {
        //
        //                 if let Some( ref restriction) = server_configurations.restricted_ips {
        //                     let incoming_ip = socket.ip().to_string();
        //                     match restriction {
        //                         WaterIpAddressesRestriction::OnlyAllowedIps(ips) => {
        //                             if !ips.contains(&incoming_ip){
        //                                 let _ = stream.shutdown().await;
        //                                 continue;
        //                             }
        //                         }
        //                         WaterIpAddressesRestriction::BlacklistIps(ips) => {
        //                             if ips.contains(&incoming_ip){
        //                                 let _ = stream.shutdown().await;
        //                                 continue;
        //                             }
        //                         }
        //                     }
        //                 }
        //                 if containing_port{
        //                     let acceptor = tls_acceptor.clone();
        //                     if let Some(tls_acceptor) = acceptor {
        //                         let _ = tokio::spawn(async move {
        //                             let  acceptor = tls_acceptor.clone();
        //                             let stream =  acceptor.accept(stream).await;
        //                             if let Ok(stream) = stream {
        //                                 _build_context_from_tls_stream::<DataHolderGeneric>(
        //                                     stream,
        //                                     socket,
        //                                     controllers
        //                                 ).await;
        //                             }
        //                         }).await;
        //                         continue;
        //                     }
        //                 }
        //                 tokio::spawn(async move{
        //                     _build_context_from_stream::<DataHolderGeneric>(
        //                         stream,
        //                         socket,
        //                         controllers
        //                     ).await;
        //                     // on connection closed
        //                 });
        //             }
        //
        //
        //         ()
        //     });
        //     threads_pool.push(join_handler);
        // }
        // for thread in threads_pool {
        //    let _ =  thread.await;
        // }


        //---------------------------------------->
        loop {
            let stream = listener.accept().await;
            let acceptor = tls_acceptor.clone();
            tokio::task::spawn(  async move   {
                let _ = tokio::task::spawn(async move {
                    if let Ok((mut stream,socket)) = stream {
                        if let Some( ref restriction) = server_configurations.restricted_ips {
                            let incoming_ip = socket.ip().to_string();
                            match restriction {
                                WaterIpAddressesRestriction::OnlyAllowedIps(ips) => {
                                    if !ips.contains(&incoming_ip){
                                        let _ = stream.shutdown().await;
                                        return;
                                    }
                                }
                                WaterIpAddressesRestriction::BlacklistIps(ips) => {
                                    if ips.contains(&incoming_ip){
                                        let _ = stream.shutdown().await;
                                        return;
                                    }
                                }
                            }
                        }
                        if containing_port{
                            if let Some( tls_acceptor) = acceptor {
                                let _ = tokio::spawn(async move {
                                    let stream =  tls_acceptor.accept(stream).await;
                                    if let Ok(stream) = stream {
                                        _build_context_from_tls_stream::<DataHolderGeneric>(
                                            stream,
                                            socket,
                                            controllers
                                        ).await;
                                    }
                                }).await;
                                return;
                            }
                        }
                        _build_context_from_stream::<DataHolderGeneric>(
                            stream,
                            socket,
                            controllers
                        ).await;
                    }

                }).await;
            });
        }
    }


    async fn _build_context_from_tls_stream<DataHolderGeneric:Send>
    (mut stream:TlsStream<TcpStream>,_address:SocketAddr,
     controllers:&'static Vec<WaterCapsuleController<DataHolderGeneric>>
    ) {
        let ip: IpAddr = _address.ip();
        if let Some(preface) = stream.get_ref().1.alpn_protocol(){
            if b"h2" == &preface {
                let  h2 = server::handshake(&mut stream).await;
                match h2 {
                    Ok(mut h2_protocol_connection) => {
                        while let Some(Ok((request,send_response))) =
                            h2_protocol_connection.accept().await {
                            let context =
                                HttpContext::<DataHolderGeneric>::from_http2_connection
                                    (request,send_response);
                            if let Ok( _context) = context {
                                handle_context(ip,_context,controllers).await;
                            }
                        }
                        return ;
                    },
                    Err(_) => {},
                }
                return;
            }
        }
        let context =
            HttpContext::<DataHolderGeneric>::from_http1_connection
                (
                    (WaterTcpStream::Tls(stream),_address)
                ).await;
        match context {
            Ok(_context)=>{
                handle_context::<DataHolderGeneric>(ip,_context,controllers).await;
            },
            _ => {}
        }
    }


    async fn _build_context_from_stream<DataHolderGeneric:Send>
    (mut stream:TcpStream,
     _address:SocketAddr,
     controllers:&'static Vec<WaterCapsuleController<DataHolderGeneric>>
    )
    {
        let ip: IpAddr = _address.ip();
        let mut preface = [0u8;3];
        if let Ok(_) = stream.peek(&mut preface).await {
            if b"PRI" == &preface {
                let  h2 = server::handshake(&mut stream).await;
                match h2 {
                    Ok(mut h2_protocol_connection) => {
                        while let Some(Ok((request,send_response))) =
                            h2_protocol_connection.accept().await {

                            let context =
                                HttpContext::<DataHolderGeneric>::from_http2_connection
                                    (request,send_response);
                            if let Ok( _context) = context {
                                handle_context(ip,_context,controllers).await;
                            }
                        }
                    },
                    Err(_) => {},
                }
            }
            else {
                let context = HttpContext::<DataHolderGeneric>::from_http1_connection
                    (
                        (WaterTcpStream::Stream(stream),_address)
                    ).await;
                match context {
                    Ok(_context)=>{
                        handle_context::<DataHolderGeneric>(ip,_context,controllers).await;
                    },
                    Err(_v)=>{
                    }
                }
            }
        }
    }


    async fn handle_context<DataHolderGeneric:Send>(_ip:IpAddr,mut _context:HttpContext<DataHolderGeneric>,
                                                    controllers:&'static Vec<WaterCapsuleController<DataHolderGeneric>>
    ){
        if let Some(_connection) = _context.get_from_headers_as_string_ref("Connection") {
            let _connection = _connection.to_lowercase();
            if _connection == "keep-alive" {
                while let Ok(_) = context_framework_handler(&mut _context,controllers).await {
                    _context.wait_for_another_request().await;
                }
                return;
            }
        }
        let _ = context_framework_handler(&mut _context,controllers).await;
    }

    async fn context_framework_handler<DataHolderGeneric:Send>(context: &mut HttpContext<DataHolderGeneric>,
                                                               controllers:&'static Vec<WaterCapsuleController<DataHolderGeneric>>
    )->Result<(),String>{
        let path = context.get_route_path();
        if path.starts_with("/favicon") {
            let mut path = path.to_string();
            if !path.ends_with(".ico"){
                path.push_str(".ico");
            }
            context.send_file_from_public_resources(&path).await?;
            return Ok(());
        }
        let config = unsafe { ___SERVER_CONFIGURATIONS.as_ref().unwrap()};
        if ! config.do_not_even_check_public_resources {
            let static_path = &config.public_files_path.replace(".","");
            if let Some(index) = path.find(static_path){
                let path = &path[(index+static_path.len())..].to_string();
                let _ = context.send_file_from_public_resources(&path).await;
                return Ok(());
            }
        }
        let _ = context.serialized_body();
        let _res = context_route_function_finder::find_function_from_controllers_and_execute(
            context,
            controllers
        ).await;
        match _res {
            Ok(_res) => {
                return Ok(());
            }
            Err(_err) => {
                context.send_str_data(&_err,true).await?;
            }
        }
        Err("".to_string())
    }


