

use std::net::SocketAddr;
use std::ops::Deref;
use bytes::{Buf, BytesMut};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;
#[cfg(feature = "support_tls")]
use tokio_rustls::server::TlsStream;
#[cfg(feature = "debugging")]
use tracing::{info,debug, trace};
use crate::server::{CapsuleWaterController, HttpStream, READING_BUF_LEN, WaterTcpStream};
#[cfg(not(feature = "use_only_http1"))]
use crate::server::sr_context::{Http2Context, HttpContext, Protocol, ServingRequestResults};


pub enum WaterStream {
    #[cfg(feature = "support_tls")]
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
     shared_factory:SHARED
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
                                    match  context.serve(controller).await {
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
                        shared_factory
                    ).await;
                    #[cfg(not(feature = "thread_shared_struct"))]
                    Self::handle_h1_connections(
                        &mut HttpStream::AsyncSecure(stream)
                        ,&self.address,
                    controller
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
                }


                #[cfg(feature = "debugging")]
                {
                    debug!("{:?} connection is using http1 protocol",self.address);
                }

                #[cfg(feature = "thread_shared_struct")]
                Self::handle_h1_connections(
                    &mut HttpStream::Async(stream),&self.address,controller,shared_factory).await;
                #[cfg(not(feature = "thread_shared_struct"))]
                Self::handle_h1_connections(
                    &mut HttpStream::Async(stream),&self.address,controller).await;
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
     controller:&'static  CapsuleWaterController<Holder,HS,QS>,
     #[cfg(feature = "thread_shared_struct")]
     shared_factory:SHARED
    ){

        #[cfg(feature = "thread_shared_struct")]
        WaterTcpStream::serve(stream,peer,controller,shared_factory).await;
        #[cfg(not(feature = "thread_shared_struct"))]
        WaterTcpStream::serve(stream,peer,controller).await;

        // old implementation
       /*
       //  let mut each_request_body_reading_buffer =
       //      BodyReadingBuffer::with_capacity(EACH_REQUEST_BODY_READING_BUFFER);
       //  let mut reading_buffer = BytesMut::with_capacity(READING_BUF_LEN);
       //  let mut response_buffer = BytesMut::with_capacity(WRITING_BUF_LEN);
       // 'main_loop: loop {
       //     reserve_buf(&mut reading_buffer);
       //
       //     if let Ok(read_size)
       //         = stream.read_buf(&mut reading_buffer).await
       //         {
       //          // when connection is closed
       //          if read_size == 0 {
       //              return;
       //          }
       //
       //
       //          loop {
       //              let buf_bytes = reading_buffer.chunk();
       //
       //              #[cfg(feature = "debugging")]
       //              {
       //                  info!("the new red data is {}",String::from_utf8_lossy(buf_bytes))
       //              }
       //
       //              if buf_bytes.is_empty() { break; }
       //
       //              #[cfg(feature = "count_connection_parsing_speed")]
       //              let t1 = std::time::SystemTime::now();
       //              let request =
       //                      IncomingRequest::<HS,QS>::new(buf_bytes);
       //              #[cfg(feature = "count_connection_parsing_speed")]
       //              {
       //                  let t2 = std::time::SystemTime::now();
       //                  let dif = t2.duration_since(t1);
       //                  println!("request from {:?}  parsed in  {:?}",peer,dif);
       //
       //              }
       //
       //              match request {
       //                  FormingRequestResult::Success(request) => {
       //
       //                      #[cfg(feature = "debugging")]
       //                      {
       //                          debug!("new request has been received ");
       //                      }
       //
       //                      let total_request_size = request.get_total_headers_length();
       //                      let left_bytes = &buf_bytes[total_request_size..];
       //
       //                      #[cfg(feature = "debugging")]
       //                      debug!("left bytes {:?}",String::from_utf8_lossy(left_bytes));
       //                      let mut context =
       //                      HttpContext::new(
       //                          Protocol::from_http1_context(
       //                              Http1Context::new(
       //                                  stream,
       //                                  &mut response_buffer,
       //                                  &mut each_request_body_reading_buffer,
       //                                  left_bytes,
       //                                  request
       //                              )
       //                          ),
       //                          peer
       //                      );
       //
       //                      #[cfg( feature = "count_connection_parsing_speed")]
       //                      let t1 = std::time::SystemTime::now();
       //
       //
       //                      _= match  context.serve(controller).await {
       //
       //                          ServingRequestResults::Stop => {return;}
       //
       //                          ServingRequestResults::Done => {
       //
       //                              #[cfg( feature = "count_connection_parsing_speed")]
       //                              {
       //                                  let end = std::time::SystemTime::now();
       //                                  println!("request from {:?}  served in {:?}",
       //                                    peer,
       //                                   end.duration_since(t1)
       //                                  );
       //                              }
       //                              // if context.method() == "GET" {
       //                              //     if total_request_size >= buf_bytes.len() {
       //                              //         reading_buffer.clear();
       //                              //         break;
       //                              //     } else {
       //                              //         reading_buffer.advance(total_request_size);
       //                              //         continue;
       //                              //     }
       //                              // }
       //                              let content_length = context.content_length().copied();
       //
       //                              match content_length {
       //                                  None => {
       //                                      let br = total_request_size >= buf_bytes.len();
       //                                      if br { reading_buffer.clear(); break ;}
       //                                      else {
       //                                          if let Some(h) = context.get_from_headers("Transfer-Encoding"){
       //                                              if h == "chunked" {
       //                                                  drop(h);
       //                                                  reading_buffer.clear();
       //                                                  continue;
       //                                              }
       //                                          }
       //                                          reading_buffer.advance(total_request_size);
       //                                      }
       //
       //                                  }
       //                                  Some(content_length) => {
       //                                      reading_buffer.advance(total_request_size);
       //                                      let mut rem = content_length;
       //                                      if each_request_body_reading_buffer.bytes_consumed > 0 {
       //                                          rem -= reading_buffer.len().min(rem);
       //                                          reading_buffer.clear();
       //                                          rem -= each_request_body_reading_buffer.bytes_consumed.min(rem);
       //                                          if !each_request_body_reading_buffer.is_empty() {
       //                                              reading_buffer.extend_from_slice(each_request_body_reading_buffer.chunk());
       //                                          }
       //                                          each_request_body_reading_buffer.reset();
       //                                      }
       //
       //                                      while rem > 0  {
       //                                          if reading_buffer.is_empty() {
       //                                              if stream.read_buf(&mut reading_buffer).await.is_err() {
       //                                                  return;
       //                                              }
       //                                          }
       //                                          let to_advance = rem.min(reading_buffer.len());
       //                                          reading_buffer.advance(to_advance);
       //                                          rem -= to_advance;
       //                                      }
       //
       //                                      if reading_buffer.is_empty() {break;}
       //
       //                                  }
       //                              }
       //
       //
       //                              continue;
       //                          }
       //                      };
       //
       //
       //                  }
       //                  FormingRequestResult::ReadMore => {
       //                      // why I need to return if reading_buf less than 250
       //                      // if reading_buffer.len() > 250 {
       //                      //     return
       //                      // }
       //                      continue 'main_loop;
       //                  }
       //                  FormingRequestResult::Err(_) => {
       //                      return;
       //                  }
       //              }
       //          }
       //
       //         if !response_buffer.is_empty() {
       //             if let Err(_) = handle_responding(&mut response_buffer,stream).await {
       //                 return;
       //             }
       //         }
       //         continue 'main_loop;
       //      }
       //     else {
       //         if !response_buffer.is_empty() {
       //             if let Err(_) = handle_responding(&mut response_buffer,stream).await {
       //                 return;
       //             }
       //         }
       //         break;
       //     }
       //  }
       */
    }

}


#[inline(always)]
pub (crate) async fn handle_responding<'e,Stream:AsyncWrite+Unpin>(response_buf:&mut BytesMut,
                                                          stream:&mut Stream)
                                                          ->Result<(),&'e str>{
    if let Err(_) = stream.write_all(&response_buf).await {
        return Err("can not write data to given buffer");
    }
    response_buf.clear();
    Ok(())
}
//
#[inline(always)]
pub (crate) fn reserve_buf(buffer: &mut BytesMut) {
    let rem = buffer.capacity() - buffer.len() ;
    if READING_BUF_LEN < rem {
        buffer.reserve(rem);
    }
}

// #[inline(always)]
// pub(crate) fn reserve_buf(buffer: &mut BytesMut) {
//     const MIN_RESERVE: usize = 1024;
//     let remaining = buffer.capacity() - buffer.len();
//     if remaining < MIN_RESERVE {
//         buffer.reserve(READING_BUF_LEN);
//     }
// }

// #[inline(always)]
// pub(crate) fn reserve_buf(buffer: &mut BytesMut) {
//     if buffer.remaining_mut() < 1024 {
//         buffer.reserve(1024);
//     }
// }
#[derive(Debug)]
pub (crate) struct BodyReadingBuffer {
    buffer:BytesMut,
    pub (crate ) bytes_consumed:usize,
    pub (crate ) extended_bytes:usize,
    pub (crate ) advanced_bytes:usize,
}


impl BodyReadingBuffer {



    // #[inline]
    // pub (crate) fn len(&self) -> usize {
    //     self.buffer.len()
    // }





    #[inline(always)]
    pub (crate) fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    #[inline(always)]
    pub (crate) fn with_capacity(len:usize)->Self{
        Self {
            buffer:BytesMut::with_capacity(len),
            bytes_consumed:0,
            extended_bytes:0,
            advanced_bytes:0,
        }
    }


    #[inline(always)]
    pub (crate) fn clear(&mut self){
        self.buffer.clear();
    }

    #[inline(always)]
    pub (crate) fn reset(&mut self){
        self.bytes_consumed = 0;
        self.advanced_bytes = 0;
        self.clear();
    }

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

    #[inline(always)]
    pub (crate) async fn read_buf<Stream>(&mut self,stream:&mut Stream) ->  tokio::io::Result<usize>
    where Stream:AsyncRead + Unpin {
        let res =  stream.read_buf(&mut self.buffer).await;
        if let Ok(s) = res {
            #[cfg(feature = "debugging")]
            {
                debug!("the red data from buffer is {} {} ",self.buffer.len(),String::from_utf8_lossy(&self.buffer))
            }
            self.bytes_consumed +=s;
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





