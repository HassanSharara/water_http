use std::net::SocketAddr;
use std::ops::Deref;
use bytes::{Buf};
use  water_buffer::WaterBuffer as BM; type BytesMut = BM<u8>;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
#[cfg(feature = "use_io_uring")]
use tokio_uring::net::TcpStream;

#[cfg(not(feature = "use_io_uring"))]
use tokio::net::TcpStream;
#[cfg(feature = "support_tls")]
use tokio_rustls::server::TlsStream;
#[cfg(feature = "use_io_uring")]
use tokio_uring::BufResult;


#[cfg(feature = "debugging")]
use tracing::{debug};

use crate::http::request::{FormingRequestResult, IncomingRequest};
use crate::server::{CapsuleWaterController, Http1Context, HttpContext, HttpStream, Protocol, ServingRequestResults};



use crate::server::matcher::Matcher;
#[cfg(not(feature = "use_only_http1"))]
use crate::server::sr_context::{Http2Context};


pub enum WaterStream {
    #[cfg(feature = "support_tls")]
    TLS(TlsStream<TcpStream>),

    #[cfg(feature = "use_io_uring")]
    TOStream(tokio_uring::net::TcpStream),
    #[cfg(not(feature = "use_io_uring"))]
    TOStream(TcpStream)
}
pub (crate) struct ConnectionStream {
    io:WaterStream,
    address:SocketAddr
}


impl  ConnectionStream {

    pub (crate) fn new(io:WaterStream,address:SocketAddr)->Self{
        Self {
            io,
            address
        }
    }

    #[inline(always)]
    pub (crate) async fn serve<
        #[cfg(feature = "use_tokio_send")]
        Holder:Send + 'static,
        #[cfg(not(feature = "use_tokio_send"))]
        Holder,
        #[cfg(all(feature = "thread_shared_struct",not(feature = "use_tokio_send")))]
        SHARED:Clone,
        #[cfg(all(feature = "thread_shared_struct",feature = "use_tokio_send"))]
        SHARED:Clone + Send + 'static,
        const HS:usize,const QS:usize,>
    (self,
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
        #[cfg(feature = "debugging")]
        {
             debug!("new connection from  : {:?}",self.address);
        }
        match  self.io {
            #[cfg(feature = "support_tls")]
            WaterStream::TLS( stream) => {
                #[cfg(feature = "debugging")]
                {
                    debug!("{:?} connected by tls layer",self.address);
                }
                if let Some(alpn_preface) = stream.get_ref().1.alpn_protocol() {

                    #[cfg(not(feature = "use_only_http1"))]
                    {
                        if alpn_preface == b"h2" {
                            let handshake
                                = h2::server::handshake(stream).await;
                            if let Ok(mut connection) = handshake {
                                let mut reading_buffer =
                                    BodyReadingBuffer::with_capacity(crate::server::configurations::EACH_REQUEST_BODY_READING_BUFFER);

                                while let Some(
                                    Ok(batch))
                                    = connection.accept().await {
                                    let mut context:HttpContext<Holder,HS,QS> =
                                        HttpContext::new(
                                            Protocol::<'_,HS, QS>::from_http2_context(
                                                Http2Context
                                                    ::<'_>
                                                ::new(batch, &mut reading_buffer)
                                            ),
                                            &self.address
                                        );
                                    context.macher = Some(matcher);
                                    match  context.serve_ef(controller).await {
                                        ServingRequestResults::Stop => {
                                            return;
                                        }
                                        ServingRequestResults::Done => {
                                            continue;
                                        }
                                    };
                                }
                            }
                            return;
                        }
                    }

                    #[cfg(feature = "thread_shared_struct")]
                    Self::handle_h1_connections(
                        &mut HttpStream::AsyncSecure(stream)
                        ,&self.address,
                        controller,
                        shared_factory,
                        matcher
                    ).await;
                    #[cfg(not(feature = "thread_shared_struct"))]
                    Self::handle_h1_connections(
                        &mut HttpStream::AsyncSecure(stream)
                        ,&self.address,
                    controller,
                        matcher.clone()
                    ).await;

                }
            }
            WaterStream::TOStream(stream) => {

                #[cfg(feature = "debugging")]
                {
                    debug!("{:?} connected without secure layer (tls)",self.address);
                }

                #[cfg(not(feature = "use_only_http1"))]
                {
                    let mut preface :[u8;3]=[0;3];
                    _=stream.peek(&mut preface).await;
                    if preface == *b"PRI" {
                        #[cfg(feature = "debugging")]
                        {
                            debug!("{:?} connection is using http2 protocol",self.address);
                        }
                        if let Ok(mut connection) =  h2::server::handshake(stream).await {
                            while let Some(Ok(batch)) = connection.accept().await {
                                let mut reading_buffer =
                                    BodyReadingBuffer::with_capacity(crate::server::configurations::EACH_REQUEST_BODY_READING_BUFFER);
                                #[cfg(feature = "thread_shared_struct")]
                                    let mut context =
                                    HttpContext::<'_,Holder,SHARED,HS,QS>::new(
                                        Protocol::<'_,HS, QS>::from_http2_context(
                                            Http2Context
                                                ::<>
                                            ::new(
                                                batch,
                                                &mut reading_buffer
                                            )
                                        ),
                                        &self.address
                                    );
                                #[cfg(feature = "thread_shared_struct")]
                                {
                                    context.thread_shared_struct = Some(shared_factory.clone());
                                }


                                #[cfg(not(feature = "thread_shared_struct"))]
                                    let mut context =
                                    HttpContext::new(
                                        Protocol::<'_,HS, QS>::from_http2_context(
                                            Http2Context
                                                ::<>
                                            ::new(
                                                batch,
                                                &mut reading_buffer
                                            )
                                        ),
                                        &self.address
                                    );
                                match  context.serve_ef(matcher.clone()).await {
                                    ServingRequestResults::Stop => {return;}
                                    ServingRequestResults::Done => {
                                        continue;
                                    }
                                };
                            }
                        }
                        return;
                    }
                }


                #[cfg(feature = "debugging")]
                {
                    debug!("{:?} connection is using http1 protocol",self.address);
                }

                #[cfg(feature = "thread_shared_struct")]
                Self::handle_h1_connections(
                    &mut HttpStream::Async(stream),&self.address,controller,shared_factory,matcher).await;
                #[cfg(not(feature = "thread_shared_struct"))]
                Self::handle_h1_connections(
                    &mut HttpStream::Async(stream),&self.address,controller,matcher).await;
            }
        };

        #[cfg(feature = "debugging")]
        {
            debug!("connection {:?} has been closed",self.address);
        }
    }


    #[inline(always)]
    async fn handle_h1_connections
    <
        #[cfg(feature = "use_tokio_send")]
        Holder:Send + 'static,
        #[cfg(not(feature = "use_tokio_send"))]
        Holder,
        #[cfg(all(feature = "thread_shared_struct",not(feature = "use_tokio_send")))]
        SHARED:Clone,


        #[cfg(all(feature = "thread_shared_struct",feature = "use_tokio_send"))]
        SHARED:Clone + Send + 'static,
        const HS:usize,
        const QS:usize,>
    (stream:&mut HttpStream,peer:&SocketAddr,
     #[cfg(feature = "thread_shared_struct")]
     controller:&'static  CapsuleWaterController<Holder,SHARED,HS,QS>,
     #[cfg(not(feature = "thread_shared_struct"))]
     _controller:&'static  CapsuleWaterController<Holder,HS,QS>,
     #[cfg(feature = "thread_shared_struct")]
     shared_factory:SHARED,
     #[cfg(feature = "thread_shared_struct")]
     matcher:Matcher<Holder,SHARED,HS,QS>,
     #[cfg(not(feature = "thread_shared_struct"))]
     matcher:Matcher<Holder,HS,QS>
    ){
        use crate::server::io::buf::{PooledWaterBuffer,PooledBufferType as BufType};
        use futures::FutureExt;
        let mut er_pool = PooledWaterBuffer::new(BufType::Body);
        let mut rb_pool = PooledWaterBuffer::new(BufType::Read);
        let mut wb_pool = PooledWaterBuffer::new(BufType::Write);
        let er  = er_pool.take_inner();
        let rb  = rb_pool.take_inner();
        let wb  = wb_pool.take_inner();
        #[cfg(not(feature = "use_io_uring"))]
        {

            let mut each_request_body_reading_buffer =
                BodyReadingBuffer::new(er);
            let mut reading_buffer = rb;
            let mut response_buffer = wb;
            'main_loop: loop {
                reserve_buf(&mut reading_buffer);

                let read_size: usize = match stream.poll_read(&mut reading_buffer).now_or_never() {
                    None => {
                        if !response_buffer.is_empty() {
                            if handle_responding(&mut response_buffer, stream).await.is_err() {
                                break 'main_loop;
                            }
                        }
                        match stream.read(&mut reading_buffer).await {
                            Ok(n)  => n,
                            _ => break 'main_loop, // Connection closed or error
                        }
                    }
                    // Case B: Data was already sitting in the OS buffer
                    Some(Ok(n)) => n,
                    Some(Err(_)) => break 'main_loop,
                };
                {
                    #[cfg(feature = "debugging")]
                    {
                        debug!("new red data is {:?}",String::from_utf8_lossy(reading_buffer.chunk()));
                    }
                    // when connection is closed
                    if read_size == 0 {
                        if !response_buffer.is_empty() {
                            _=handle_responding(&mut response_buffer,stream).await
                        }
                        break 'main_loop;
                    }
                    reading_buffer.advance_mut(read_size);




                    loop {
                        let buf_bytes = reading_buffer.chunk();

                        #[cfg(feature = "debugging")]
                        {
                            tracing::info!("the new red data is {}",String::from_utf8_lossy(buf_bytes))
                        }

                        if buf_bytes.is_empty() { break }
                        #[cfg(feature = "count_connection_parsing_speed")]
                            let t1 = std::time::SystemTime::now();
                        let request =
                            IncomingRequest::<HS,QS>::new(buf_bytes);
                        #[cfg(feature = "count_connection_parsing_speed")]
                        {
                            let t2 = std::time::SystemTime::now();
                            let dif = t2.duration_since(t1);
                            println!("request from {:?}  parsed in  {:?}",peer,dif);

                        }


                        match request {
                            FormingRequestResult::Success(request) => {

                                #[cfg(feature = "debugging")]
                                {
                                    debug!("new request has been received ");
                                }

                                let total_request_size = request.get_total_headers_length();
                                let left_bytes = &buf_bytes[total_request_size..];

                                #[cfg(feature = "debugging")]
                                debug!("left bytes {:?}",String::from_utf8_lossy(left_bytes));
                                #[cfg(feature = "thread_shared_struct")]
                                    let mut context = HttpContext::<Holder,SHARED, HS, QS>::new(
                                    Protocol::Http1(Http1Context::new(stream,
                                                                      &mut response_buffer,
                                                                      &mut each_request_body_reading_buffer,
                                                                      left_bytes,
                                                                      request)),
                                    peer
                                );

                                #[cfg(not(feature = "thread_shared_struct"))]
                                    let mut context = HttpContext::<Holder, HS, QS>::new(
                                    Protocol::Http1(Http1Context::new(
                                        stream,
                                        &mut response_buffer,
                                        &mut each_request_body_reading_buffer,
                                        left_bytes,
                                        request
                                    )
                                    ),
                                    peer
                                );

                                #[cfg(feature = "thread_shared_struct")]
                                {
                                    context.thread_shared_struct = Some(shared_factory.clone());
                                }

                                #[cfg( feature = "count_connection_parsing_speed")]
                                    let t1 = std::time::SystemTime::now();


                                _= match  context.serve_ef(matcher.clone()).await {

                                    ServingRequestResults::Stop => { break 'main_loop; }

                                    ServingRequestResults::Done => {

                                        #[cfg( feature = "count_connection_parsing_speed")]
                                        {
                                            let end = std::time::SystemTime::now();
                                            println!("request from {:?}  served in {:?}",
                                                     peer,
                                                     end.duration_since(t1)
                                            );
                                        }

                                        let content_length = context.content_length();

                                        match content_length {
                                            None => {
                                                let br = total_request_size >= buf_bytes.len();
                                                if br { reading_buffer.clear(); break ;}
                                                else {
                                                    #[cfg(feature = "accept_transfer_chunked")]
                                                    if let Some(h) = context.get_from_headers("Transfer-Encoding"){
                                                        if h == "chunked" {
                                                            drop(h);
                                                            #[cfg(feature = "debugging")]
                                                            {
                                                                debug!("request completed by chunked");
                                                            }
                                                            reading_buffer.clear();
                                                            each_request_body_reading_buffer.clear();
                                                            continue;
                                                        }
                                                    }
                                                    reading_buffer.advance(total_request_size);
                                                }
                                            }
                                            Some(content_length) => {
                                                let content_length = *content_length;
                                                reading_buffer.advance(total_request_size);
                                                let mut rem = content_length;
                                                if rem == 0 { continue }
                                                // advancing reading buffer length if there is remaining
                                                let read_buff_len = reading_buffer.len();
                                                #[cfg(feature = "debugging")]
                                                {
                                                    tracing::info!("\
                                                  \
                                                  \
                                                  consumed {}  advanced bytes : {} remaining after serving request with content length {} while reading\
                                                   buffer is {} while extended bytes is {}"
                                                      ,
                                                      each_request_body_reading_buffer.bytes_red_by_buffer,
                                                      each_request_body_reading_buffer.advanced_bytes,
                                                    rem,
                                                      read_buff_len,
                                                      each_request_body_reading_buffer.extended_bytes,
                                                  );
                                                }


                                                if read_buff_len > 0 {
                                                    let td = rem.min(read_buff_len);
                                                    rem -= td;
                                                    reading_buffer.advance(td);
                                                    if rem == 0 {
                                                        each_request_body_reading_buffer.clear();
                                                        continue
                                                    }
                                                }

                                                #[cfg(feature = "debugging")]
                                                {
                                                    debug!("start advancing extended bytes which is {}",each_request_body_reading_buffer.extended_bytes);
                                                }

                                                while each_request_body_reading_buffer.extended_bytes > 0 {
                                                    if each_request_body_reading_buffer.advanced_bytes > 0 {
                                                        let td = each_request_body_reading_buffer.extended_bytes.min(
                                                            each_request_body_reading_buffer.advanced_bytes
                                                        );
                                                        each_request_body_reading_buffer.extended_bytes -= td;
                                                        each_request_body_reading_buffer.advanced_bytes -= td;
                                                        continue
                                                    }
                                                    each_request_body_reading_buffer
                                                        .advance(each_request_body_reading_buffer.extended_bytes);
                                                    each_request_body_reading_buffer.advanced_bytes -=  each_request_body_reading_buffer.extended_bytes;
                                                    each_request_body_reading_buffer.extended_bytes = 0;
                                                    break
                                                }

                                                #[cfg(feature = "debugging")]
                                                {
                                                    debug!("end advancing extended bytes ! ");
                                                    tracing::info!("\
                                                  \
                                                  \
                                                  consumed {}  advanced bytes : {} remaining after serving request with content length {} while reading \
                                                   buffer is {} while extended bytes is {} while body buffer len is {}"
                                                      ,
                                                      each_request_body_reading_buffer.bytes_red_by_buffer,
                                                      each_request_body_reading_buffer.advanced_bytes,
                                                      rem,
                                                      reading_buffer.len(),
                                                      each_request_body_reading_buffer.extended_bytes,
                                                      each_request_body_reading_buffer.len(),
                                                  );
                                                }

                                                if each_request_body_reading_buffer.advanced_bytes > 0 {
                                                    let t = each_request_body_reading_buffer.advanced_bytes.min(rem);
                                                    rem-=t;
                                                }
                                                if rem == 0 {
                                                    reading_buffer.extend_from_slice(each_request_body_reading_buffer.chunk());
                                                    each_request_body_reading_buffer.clear();
                                                    continue
                                                }
                                                if each_request_body_reading_buffer.len() >  0 {
                                                    let l = rem.min(each_request_body_reading_buffer.len());
                                                    rem-=l;
                                                    each_request_body_reading_buffer.advance(l);
                                                    if rem == 0 {
                                                        if each_request_body_reading_buffer.len() > 0 {
                                                            reading_buffer.extend_from_slice(each_request_body_reading_buffer.chunk());
                                                        }
                                                        each_request_body_reading_buffer.clear();
                                                        continue
                                                    }
                                                }
                                                each_request_body_reading_buffer.clear();
                                                #[cfg(feature = "debugging")]
                                                {
                                                    debug!("start draining remaining content length while rem is {rem}");
                                                }
                                                while rem > 0 {

                                                    let r = match stream {
                                                        #[cfg(feature = "support_tls")]
                                                        HttpStream::AsyncSecure(s) => {
                                                            s.read(reading_buffer.chunk_mut()).await
                                                        }
                                                        HttpStream::Async(s) => {
                                                            let r = s.read(reading_buffer.chunk_mut()).await;

                                                            r

                                                        }

                                                    };
                                                    if let Ok( r) = r {
                                                        let l = r.min(rem);
                                                        rem -= l;
                                                        reading_buffer.advance(l);
                                                    } else {                         break 'main_loop; }
                                                }
                                                #[cfg(feature = "debugging")]
                                                {
                                                    debug!("[end] draining remaining content length");
                                                }
                                            }

                                        }


                                        continue;
                                    }
                                };


                            }
                            FormingRequestResult::ReadMore => {

                                #[cfg(feature = "debugging")]
                                {
                                    tracing::info!("incoming request is not enough: now we need to read more ");
                                }
                                continue 'main_loop;
                            }
                            FormingRequestResult::Err(_e) => {

                                #[cfg(feature = "debugging")]
                                {
                                    tracing::error!("incoming request has error {:?} \n the request is {:?}",_e,
                                    String::from_utf8_lossy(reading_buffer.chunk())
                                  );
                                }
                                break 'main_loop
                            }
                        }
                    }

                    if   !response_buffer.is_empty() {
                        if  handle_responding(&mut response_buffer,stream).await.is_err() {
                            break 'main_loop;
                        }
                    }
                    continue 'main_loop;
                }
            }
            PooledWaterBuffer::recycle(each_request_body_reading_buffer.buffer,BufType::Body);
            PooledWaterBuffer::recycle(reading_buffer,BufType::Read);
            PooledWaterBuffer::recycle(response_buffer,BufType::Write);
        }



       #[cfg(feature = "use_io_uring")]
       {

           let mut each_request_body_reading_buffer =
               BodyReadingBuffer::new(er);
           let mut reading_buffer = rb;
           let mut response_buffer = wb;

          'main_loop: loop {
              reserve_buf(&mut reading_buffer);

              if let Ok(read_size)
                  = match stream {
                  #[cfg(feature = "support_tls")]
                  HttpStream::AsyncSecure(s) => {
                      let (r,b) = s.read(unsafe{reading_buffer.unsafe_clone()}).await;
                      r
                  }
                  HttpStream::Async(s) => {
                      let (r,b) = s.read(unsafe{reading_buffer.unsafe_clone()}).await;
                       r
                  }

              }
              {
                  // when connection is closed
                  if read_size == 0 {
                      break 'main_loop;
                  }
                  loop {
                      let buf_bytes = reading_buffer.chunk();
                      // each_request_body_reading_buffer.clear();
                      #[cfg(feature = "debugging")]
                      {
                          tracing::info!("the new red data is {}",String::from_utf8_lossy(buf_bytes))
                      }

                      if buf_bytes.is_empty() { break; }
                      use crate::{http::request::{IncomingRequest,FormingRequestResult},server::Http1Context};
                      #[cfg(feature = "count_connection_parsing_speed")]
                          let t1 = std::time::SystemTime::now();
                      let request =
                          IncomingRequest::<HS,QS>::new(buf_bytes);
                      #[cfg(feature = "count_connection_parsing_speed")]
                      {
                          let t2 = std::time::SystemTime::now();
                          let dif = t2.duration_since(t1);
                          println!("request from {:?}  parsed in  {:?}",peer,dif);

                      }


                      match request {
                          FormingRequestResult::Success(request) => {

                              #[cfg(feature = "debugging")]
                              {
                                  debug!("new request has been received ");
                              }

                              let total_request_size = request.get_total_headers_length();
                              let left_bytes = &buf_bytes[total_request_size..];

                              #[cfg(feature = "debugging")]
                              debug!("left bytes {:?}",String::from_utf8_lossy(left_bytes));


                                  let mut context =
                                  HttpContext::new(
                                      Protocol::from_http1_context(
                                          Http1Context::new(
                                              stream,
                                              unsafe{response_buffer.unsafe_clone()},
                                              &mut each_request_body_reading_buffer,
                                              left_bytes,
                                              request
                                          )
                                      ),
                                      peer
                                  );


                              #[cfg(feature = "thread_shared_struct")]
                              {
                                  context.thread_shared_struct = Some(shared_factory.clone());
                              }

                              #[cfg( feature = "count_connection_parsing_speed")]
                              let t1 = std::time::SystemTime::now();

                              _= match  context.serve_ef(matcher.clone()).await {

                                  ServingRequestResults::Stop => {break}

                                  ServingRequestResults::Done => {

                                      #[cfg( feature = "count_connection_parsing_speed")]
                                      {
                                          let end = std::time::SystemTime::now();
                                          println!("request from {:?}  served in {:?}",
                                                   peer,
                                                   end.duration_since(t1)
                                          );
                                      }

                                      let content_length = context.content_length();

                                      match content_length {
                                          None => {
                                              reading_buffer.advance(total_request_size);
                                              if reading_buffer.is_empty() {
                                                  continue 'main_loop
                                              }
                                              #[cfg(feature = "accept_transfer_chunked")]
                                              {
                                                  if let Some(h) = context.get_from_headers("Transfer-Encoding"){


                                                      if h == "chunked" {
                                                          drop(h);
                                                          #[cfg(feature = "debugging")]
                                                          {
                                                              debug!("request completed by chunked");
                                                          }
                                                          reading_buffer.clear();
                                                          each_request_body_reading_buffer.clear();
                                                          each_request_body_reading_buffer.reset();
                                                          continue;
                                                      }
                                                  }

                                              }
                                              continue
                                          }
                                          Some(content_length) => {
                                              let content_length = *content_length;
                                              reading_buffer.advance(total_request_size);
                                              let mut rem = content_length;
                                              if rem == 0 { continue }
                                              // advancing reading buffer length if there is remaining
                                              let read_buff_len = reading_buffer.len();
                                              #[cfg(feature = "debugging")]
                                              {
                                                  tracing::info!("\
                                                  \
                                                  \
                                                  consumed {}  advanced bytes : {} remaining after serving request with content length {} while reading\
                                                   buffer is {} while extended bytes is {}"
                                                      ,
                                                      each_request_body_reading_buffer.bytes_red_by_buffer,
                                                      each_request_body_reading_buffer.advanced_bytes,
                                                    rem,
                                                      read_buff_len,
                                                      each_request_body_reading_buffer.extended_bytes,
                                                  );
                                              }


                                              if read_buff_len > 0 {
                                                  let td = rem.min(read_buff_len);
                                                  rem -= td;
                                                  reading_buffer.advance(td);
                                                  if rem == 0 {
                                                      each_request_body_reading_buffer.clear();
                                                      continue
                                                  }
                                              }

                                              #[cfg(feature = "debugging")]
                                              {
                                                  debug!("start advancing extended bytes which is {}",each_request_body_reading_buffer.extended_bytes);
                                              }

                                              while each_request_body_reading_buffer.extended_bytes > 0 {
                                                   if each_request_body_reading_buffer.advanced_bytes > 0 {
                                                       let td = each_request_body_reading_buffer.extended_bytes.min(
                                                           each_request_body_reading_buffer.advanced_bytes
                                                       );
                                                       each_request_body_reading_buffer.extended_bytes -= td;
                                                       each_request_body_reading_buffer.advanced_bytes -= td;
                                                       continue
                                                   }
                                                   each_request_body_reading_buffer
                                                       .advance(each_request_body_reading_buffer.extended_bytes);
                                                   each_request_body_reading_buffer.advanced_bytes -=  each_request_body_reading_buffer.extended_bytes;
                                                   each_request_body_reading_buffer.extended_bytes = 0;
                                                   break
                                               }

                                              #[cfg(feature = "debugging")]
                                              {
                                                  debug!("end advancing extended bytes ! ");
                                                  tracing::info!("\
                                                  \
                                                  \
                                                  consumed {}  advanced bytes : {} remaining after serving request with content length {} while reading \
                                                   buffer is {} while extended bytes is {} while body buffer len is {}"
                                                      ,
                                                      each_request_body_reading_buffer.bytes_red_by_buffer,
                                                      each_request_body_reading_buffer.advanced_bytes,
                                                      rem,
                                                      reading_buffer.len(),
                                                      each_request_body_reading_buffer.extended_bytes,
                                                      each_request_body_reading_buffer.len(),
                                                  );
                                              }

                                              if each_request_body_reading_buffer.advanced_bytes > 0 {
                                                  let t = each_request_body_reading_buffer.advanced_bytes.min(rem);
                                                  rem-=t;
                                              }
                                              if rem == 0 {
                                                  reading_buffer.extend_from_slice(each_request_body_reading_buffer.chunk());
                                                  each_request_body_reading_buffer.clear();
                                                  continue
                                              }
                                              if each_request_body_reading_buffer.len() >  0 {
                                                  let l = rem.min(each_request_body_reading_buffer.len());
                                                  rem-=l;
                                                  each_request_body_reading_buffer.advance(l);
                                                  if rem == 0 {
                                                      if each_request_body_reading_buffer.len() > 0 {
                                                          reading_buffer.extend_from_slice(each_request_body_reading_buffer.chunk());
                                                      }
                                                      each_request_body_reading_buffer.clear();
                                                      continue
                                                  }
                                              }
                                              each_request_body_reading_buffer.clear();
                                              #[cfg(feature = "debugging")]
                                              {
                                                  debug!("start draining remaining content length while rem is {rem}");
                                              }
                                              while rem > 0 {

                                                  let r = match stream {
                                                      #[cfg(feature = "support_tls")]
                                                      HttpStream::AsyncSecure(s) => {
                                                          let (r,_) = s.read(unsafe{reading_buffer.unsafe_clone()}).await ;
                                                          r
                                                      }
                                                      HttpStream::Async(s) => {
                                                          let (r,_) = s.read(unsafe{reading_buffer.unsafe_clone()}).await ;
                                                          r
                                                      }
                                                  };
                                                  if let Ok( r) = r {
                                                      let l = r.min(rem);
                                                      rem -= l;
                                                      reading_buffer.advance(l);
                                                  } else {  break 'main_loop; }
                                              }
                                              #[cfg(feature = "debugging")]
                                              {
                                                  debug!("[end] draining remaining content length");
                                              }
                                          }

                                      }


                                      continue;
                                  }
                              };


                          }
                          FormingRequestResult::ReadMore => {
                              #[cfg(feature = "debugging")]
                              {
                                  tracing::info!("incoming request is not enough: now we need to read more ");
                              }
                              continue 'main_loop;
                          }
                          FormingRequestResult::Err(e) => {
                              #[cfg(feature = "debugging")]
                              {
                                  tracing::error!("incoming request has error {:?} \n the request is {:?}",e,
                                    String::from_utf8_lossy(reading_buffer.chunk())
                                  );
                              }
                              break 'main_loop;
                              ;
                          }
                      }
                  }
                  if !response_buffer.is_empty() {
                      if  handle_responding(unsafe{response_buffer.unsafe_clone()},stream).await.is_err() {
                          break 'main_loop;
                      }
                  }
                  continue 'main_loop;
              }
              else {
                  if !response_buffer.is_empty() {
                      _= handle_responding(unsafe{response_buffer.unsafe_clone()},stream).await;
                  }
                  break 'main_loop;
              }
          }

           PooledWaterBuffer::recycle(each_request_body_reading_buffer.buffer,BufType::Body);
           PooledWaterBuffer::recycle(reading_buffer,BufType::Read);
           PooledWaterBuffer::recycle(response_buffer,BufType::Write);
      }


    }
}




#[cfg(feature = "use_io_uring")]
#[inline(always)]
pub (crate) async fn handle_responding<'e>
(response_buf:BytesMut,stream:&mut HttpStream) ->Result<BytesMut,&'e str>{

    match stream {
        #[cfg(feature = "support_tls")]
        HttpStream::AsyncSecure(h) => {
            todo!()
        }
        HttpStream::Async(h) => {
           let (r,mut b) =  h.write_all(response_buf).await;
            if r.is_err() { return Err("can not write data to given buffer")}
             b.clear();
            return  Ok(b);
        }
    };

}


#[cfg(not(feature = "use_io_uring"))]
#[inline(always)]
pub (crate) async fn handle_responding<'e,
    Stream:AsyncWrite+Unpin,

>
(response_buf:&mut BytesMut,stream:&mut Stream) ->Result<(),&'e str>{
    if let Err(_) = stream.write_all(&response_buf).await {
        return Err("can not write data to given buffer");
    }
    response_buf.clear();
    Ok(())
}


//
#[inline(always)]
pub (crate) fn reserve_buf(buffer: &mut BytesMut) {
    if buffer.is_empty() {
        buffer.reset();
        return
    }

    let remaining = buffer.mut_len() ;
    const LIMIT:usize = 1024 * 1 ;
    if remaining > LIMIT { return }
    buffer.reserve( LIMIT * 16 - remaining );
}

#[derive(Debug)]
pub (crate) struct BodyReadingBuffer {
    buffer:BytesMut,
    pub (crate ) bytes_red_by_buffer:usize,
    pub (crate ) extended_bytes:usize,
    pub (crate ) advanced_bytes:usize,
}


impl BodyReadingBuffer {





    // #[inline(always)]
    // pub (crate) fn is_empty(&self) -> bool {
    //     self.buffer.is_empty()
    // }


    #[inline(always)]
    pub (crate) fn new(buffer:BytesMut)->Self{
        Self {
            buffer,
            bytes_red_by_buffer:0,
            extended_bytes:0,
            advanced_bytes:0
        }
    }
    // #[inline(always)]
    // pub (crate) fn with_capacity(len:usize)->Self{
    //     Self {
    //         buffer:BytesMut::with_capacity(len),
    //         bytes_red_by_buffer:0,
    //         extended_bytes:0,
    //         advanced_bytes:0,
    //     }
    // }


    #[inline(always)]
    pub (crate) fn clear(&mut self){
        self.buffer.clear();
        self.extended_bytes = 0;
        self.bytes_red_by_buffer = 0;
        self.advanced_bytes = 0;
    }

    // #[inline(always)]
    // pub (crate) fn reset(&mut self){
    //     self.bytes_red_by_buffer = 0;
    //     self.advanced_bytes = 0;
    //     self.clear();
    // }

    #[inline(always)]
    pub (crate) fn extend_from_slice(&mut self,slice:&[u8]) {
        self.extended_bytes += slice.len();
        self.buffer.extend_from_slice(slice);
    }


    //
    // #[inline]
    // pub (crate) fn as_str(&self)->Cow<'_,str>{
    //     String::from_utf8_lossy(self.chunk())
    // }




    #[cfg(feature = "use_io_uring")]
    #[inline(always)]
    pub (crate) async fn read_buf(&mut self,stream:&mut HttpStream) ->  Result<usize,()>

    {
        let res = match  stream {
            #[cfg(feature = "support_tls")]
            HttpStream::AsyncSecure(h) => {                h.read(&mut self.buffer)
            }
            HttpStream::Async(h) => {
                let (r,_) = h.read(unsafe{self.buffer.unsafe_clone()}).await;
                r
            }
        };
        if let Ok(s) = res {
            #[cfg(feature = "debugging")]
            {
                debug!("the red data from buffer is {} {:?} ",self.buffer.len(),String::from_utf8_lossy(&self.buffer))
            }
            self.bytes_red_by_buffer +=s;
            return Ok(s)
        }
        return Err(());
    }

    #[cfg(not(feature = "use_io_uring"))]
    #[inline(always)]
    pub (crate) async fn read_buf<Stream:AsyncRead + Unpin, >(&mut self,stream:&mut Stream) ->  tokio::io::Result<usize>
       {
        let res =  stream.read_buf(&mut self.buffer).await;
        if let Ok(s) = res {
            #[cfg(feature = "debugging")]
            {
                debug!("the red data from buffer is {} {} ",self.buffer.len(),String::from_utf8_lossy(&self.buffer))
            }
            self.bytes_red_by_buffer +=s;
        }
        return res;
    }


}


impl AsRef<[u8]> for  BodyReadingBuffer {
    #[inline(always)]
    fn as_ref(&self) -> &[u8] {
        self.buffer.as_ref()
    }
}

impl Deref for  BodyReadingBuffer {
    type Target = [u8];

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}


impl Buf for BodyReadingBuffer {
    #[inline(always)]

    fn remaining(&self) -> usize {
        self.buffer.remaining()
    }

    #[inline(always)]

    fn chunk(&self) -> &[u8] {
        self.buffer.chunk()
    }

    #[inline(always)]

    fn advance(&mut self, cnt: usize) {
        self.advanced_bytes +=cnt;
        self.buffer.advance(cnt)
    }
}





