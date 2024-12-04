
use std::net::SocketAddr;
use bytes::{Buf, BytesMut};
use tokio::io::{AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_rustls::server::TlsStream;
#[cfg(feature = "debugging")]
use tracing::info;
use crate::http::request::{FormingRequestResult, IncomingRequest};
use crate::server::{CapsuleWaterController, EACH_REQUEST_BODY_READING_BUFFER, HttpStream, READING_BUF_LEN, WRITING_BUF_LEN};
use crate::server::sr_context::{Http1Context, Http2Context, HttpContext, Protocol, ServingRequestResults};


pub enum WaterStream {
    TLS(TlsStream<TcpStream>),
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
    pub (crate) async fn serve<Holder:Send + 'static ,const HS:usize,const QS:usize,>
    (self,
     controller:&'static  CapsuleWaterController<Holder,HS,QS>

    ){
        #[cfg(feature = "debugging")]
        {
            info!("new connection from ip : {:?}",self.address);
        }
        match  self.io {
            WaterStream::TLS( mut stream) => {
                if let Some(alpn_preface) = stream.get_ref().1.alpn_protocol() {
                    if alpn_preface == b"h2" {
                        let handshake
                        = h2::server::handshake(stream).await;
                        if let Ok(mut connection) = handshake {
                            let mut reading_buffer =
                            BytesMut::with_capacity(EACH_REQUEST_BODY_READING_BUFFER);
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
                                match  context.serve(controller).await {
                                    ServingRequestResults::Stop => {return;}
                                    ServingRequestResults::Done => {
                                        continue;
                                    }
                                };
                                 }
                        }
                        return;
                    }
                    Self::handle_h1_connections(
                        &mut HttpStream::AsyncSecure(stream)
                        ,&self.address,
                    controller
                    ).await;

                }
            }
            WaterStream::TOStream(stream) => {
                let mut preface :[u8;3]=[0;3];
                _=stream.peek(&mut preface).await;
                if preface == *b"PRI" {
                    if let Ok(mut connection) =  h2::server::handshake(stream).await {
                        while let Some(Ok(batch)) = connection.accept().await {
                            let mut reading_buffer =
                                BytesMut::with_capacity(EACH_REQUEST_BODY_READING_BUFFER);
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
                            match  context.serve(controller).await {
                                ServingRequestResults::Stop => {return;}
                                ServingRequestResults::Done => {
                                    continue;
                                }
                            };
                        }
                    }
                    return;
                }
                Self::handle_h1_connections(
                    &mut HttpStream::Async(stream),&self.address,controller).await;
            }
        };
    }


    async fn handle_h1_connections
    <Holder:Send + 'static,
        const HS:usize,
        const QS:usize,>
    (stream:&mut HttpStream,peer:&SocketAddr,
     controller:&'static  CapsuleWaterController<Holder,HS,QS>

    ){

        let mut each_request_body_reading_buffer =
            BytesMut::with_capacity(EACH_REQUEST_BODY_READING_BUFFER);
        let mut reading_buffer = BytesMut::with_capacity(READING_BUF_LEN);
        let mut response_buffer = BytesMut::with_capacity(WRITING_BUF_LEN);
       'main_loop: loop {
           reserve_buf(&mut reading_buffer);
           // println!("reading from buffer {requests_count} {:?}",peer);

           // handling body reading buffer
           if !each_request_body_reading_buffer.is_empty() {
               reading_buffer.clear();
               reading_buffer.extend_from_slice(&each_request_body_reading_buffer);
               each_request_body_reading_buffer.clear();
           }

           if let Ok(read_size)
               = stream.read_buf(&mut reading_buffer).await {

                // when connection is closed
                if read_size == 0 {
                    return;
                }

                loop {
                    let buf_bytes = reading_buffer.chunk();
                    if buf_bytes.is_empty() {
                        break;
                    }

                    #[cfg(feature = "count_connection_parsing_speed")]
                    let t1 = std::time::SystemTime::now();
                    let mut
                        request =
                            IncomingRequest::<HS,QS>::new(buf_bytes);
                    #[cfg(feature = "count_connection_parsing_speed")]
                    {
                        let t2 = std::time::SystemTime::now();
                        let dif = t2.duration_since(t1);
                        println!("difference is {:?}",dif);

                    }

                    match request {
                        FormingRequestResult::Success(request) => {
                            let mut total_request_size = request.total_headers_bytes;
                            let left_bytes = &buf_bytes[total_request_size..];
                            let mut context =
                            HttpContext::new(
                                Protocol::from_http1_context(
                                    Http1Context::new(
                                        stream,
                                        &mut response_buffer,
                                        &mut each_request_body_reading_buffer,
                                        left_bytes,
                                        request
                                    )
                                ),
                                peer
                            );
                            let content_length = context.content_length().copied();

                            _= match  context.serve(controller).await {
                                ServingRequestResults::Stop => {return;}
                                ServingRequestResults::Done => {
                                    match content_length {
                                        None => {
                                            let br = total_request_size >= buf_bytes.len();
                                            if br { reading_buffer.clear(); break ;}
                                            else {
                                                reading_buffer.advance(total_request_size);
                                            }
                                        }
                                        Some(content_length) => {
                                            reading_buffer.advance(total_request_size);
                                            let mut rem = content_length;
                                            if !each_request_body_reading_buffer.is_empty() {
                                                reading_buffer.clear();
                                                while rem > 0  {
                                                    if each_request_body_reading_buffer.is_empty() { break; }
                                                    let to_advance =   rem.min(each_request_body_reading_buffer.len());
                                                    each_request_body_reading_buffer
                                                        .advance(
                                                            to_advance
                                                        );
                                                    rem -= to_advance;
                                                }
                                            }
                                            while rem > 0  {
                                                if reading_buffer.is_empty() {
                                                    if stream.read_buf(&mut reading_buffer).await.is_err() {
                                                        return;
                                                    }
                                                }
                                                let to_advance = rem.min(reading_buffer.len());
                                                reading_buffer.advance(to_advance);
                                                rem -= to_advance;
                                            }
                                            if reading_buffer.is_empty() {
                                                break;
                                            }
                                            continue;
                                        }
                                    }


                                    continue;
                                }
                            };


                        }
                        FormingRequestResult::ReadMore => {
                            break;
                        }
                        FormingRequestResult::Err(_) => {
                            return;
                        }
                    }
                }

               if !response_buffer.is_empty() {
                   if let Err(_) = handle_responding(&mut response_buffer,stream).await {
                       return;
                   }
               }
               continue 'main_loop;
            }
           else {
               if !response_buffer.is_empty() {
                   if let Err(_) = handle_responding(&mut response_buffer,stream).await {
                       return;
                   }
               }
               break;
           }
        }
    }

}
#[inline]
pub (crate) async fn handle_responding<'e,Stream:AsyncWrite+Unpin>(response_buf:&mut BytesMut,
                                                          stream:&mut Stream)
                                                          ->Result<(),&'e str>{
    if let Err(_) = stream.write_all(&response_buf).await {
        return Err("can not write data to given buffer");
    }
    response_buf.clear();
    Ok(())
}

#[inline]
pub (crate) fn reserve_buf(buffer: &mut BytesMut) {
    let rem = buffer.capacity() - buffer.len() ;
    if READING_BUF_LEN < rem {
        buffer.reserve(rem);
    } else if rem < 1024 {
        buffer.reserve(READING_BUF_LEN - rem);
    }
}



