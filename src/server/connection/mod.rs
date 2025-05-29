use std::net::SocketAddr;
use std::ops::Deref;
use bytes::{Buf, BytesMut};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_rustls::server::TlsStream;
#[cfg(feature = "debugging")]
use tracing::{info,debug, trace};
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
             debug!("new connection from  : {:?}",self.address);
        }
        match  self.io {
            WaterStream::TLS( stream) => {
                #[cfg(feature = "debugging")]
                {
                    debug!("{:?} connected by tls layer",self.address);
                }
                if let Some(alpn_preface) = stream.get_ref().1.alpn_protocol() {
                    if alpn_preface == b"h2" {
                        let handshake
                        = h2::server::handshake(stream).await;
                        if let Ok(mut connection) = handshake {
                            let mut reading_buffer =
                            BodyReadingBuffer::with_capacity(EACH_REQUEST_BODY_READING_BUFFER);
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
                                BodyReadingBuffer::with_capacity(EACH_REQUEST_BODY_READING_BUFFER);
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
                #[cfg(feature = "debugging")]
                {
                    debug!("{:?} connection is using http1 protocol",self.address);
                }
                Self::handle_h1_connections(
                    &mut HttpStream::Async(stream),&self.address,controller).await;
            }
        };

        #[cfg(feature = "debugging")]
        {
            debug!("connection {:?} has been closed",self.address);
        }
    }


    async fn handle_h1_connections
    <Holder:Send + 'static,
        const HS:usize,
        const QS:usize,>
    (stream:&mut HttpStream,peer:&SocketAddr,
     controller:&'static  CapsuleWaterController<Holder,HS,QS>

    ){
        let mut each_request_body_reading_buffer =
            BodyReadingBuffer::with_capacity(EACH_REQUEST_BODY_READING_BUFFER);
        let mut reading_buffer = BytesMut::with_capacity(READING_BUF_LEN);
        let mut response_buffer = BytesMut::with_capacity(WRITING_BUF_LEN);
       'main_loop: loop {
           reserve_buf(&mut reading_buffer);
           if let Ok(read_size)
               = stream.read_buf(&mut reading_buffer).await
               {
                // when connection is closed
                if read_size == 0 {
                    return;
                }

                loop {
                    let buf_bytes = reading_buffer.chunk();

                    #[cfg(feature = "debugging")]
                    {
                        info!("the new red data is {}",String::from_utf8_lossy(buf_bytes))
                    }

                    if buf_bytes.is_empty() {
                        break;
                    }

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

                            let total_request_size = request.total_headers_bytes;
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

                            #[cfg( feature = "count_connection_parsing_speed")]
                            let t1 = std::time::SystemTime::now();


                            _= match  context.serve(controller).await {
                                ServingRequestResults::Stop => {return;}
                                ServingRequestResults::Done => {
                                    #[cfg( feature = "count_connection_parsing_speed")]
                                    {
                                        let end = std::time::SystemTime::now();
                                        println!("request from {:?}  served in {:?}",
                                          peer,
                                         end.duration_since(t1)
                                        );
                                    }
                                    match content_length {
                                        None => {
                                            let br = total_request_size >= buf_bytes.len();
                                            if br { reading_buffer.clear(); break ;}
                                            else {
                                                if let Some(h) = context.get_from_headers("Transfer-Encoding"){
                                                    if h == "chunked" {
                                                        drop(h);
                                                        reading_buffer.clear();
                                                        continue;
                                                    }
                                                }
                                                reading_buffer.advance(total_request_size);
                                            }

                                        }
                                        Some(content_length) => {
                                            reading_buffer.advance(total_request_size);
                                            let mut rem = content_length;
                                            if each_request_body_reading_buffer.bytes_consumed > 0 {
                                                rem -= reading_buffer.len().min(rem);
                                                reading_buffer.clear();
                                                rem -= each_request_body_reading_buffer.bytes_consumed.min(rem);
                                                if !each_request_body_reading_buffer.is_empty() {
                                                    reading_buffer.extend_from_slice(each_request_body_reading_buffer.chunk());
                                                }
                                                each_request_body_reading_buffer.reset();
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

                                            if reading_buffer.is_empty() {break;}

                                        }
                                    }


                                    continue;
                                }
                            };


                        }
                        FormingRequestResult::ReadMore => {
                            if reading_buffer.len() > 250 {
                                return
                            }
                            break;
                        }
                        FormingRequestResult::Err => {
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



pub (crate) struct BodyReadingBuffer {
    buffer:BytesMut,
    pub (crate ) bytes_consumed:usize,
    pub (crate ) advanced_bytes:usize,
}


impl BodyReadingBuffer {



    // #[inline]
    // pub (crate) fn len(&self) -> usize {
    //     self.buffer.len()
    // }






    #[inline]
    pub (crate) fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub (crate) fn with_capacity(len:usize)->Self{
        Self {
            buffer:BytesMut::with_capacity(len),
            bytes_consumed:0,
            advanced_bytes:0,
        }
    }
    pub (crate) fn clear(&mut self){
        self.buffer.clear();
    }

    pub (crate) fn reset(&mut self){
        self.bytes_consumed = 0;
        self.advanced_bytes = 0;
        self.clear();
    }


    pub (crate) fn extend_from_slice(&mut self,slice:&[u8]) {
        self.buffer.extend_from_slice(slice);
    }


    //
    // #[inline]
    // pub (crate) fn as_str(&self)->Cow<str>{
    //     String::from_utf8_lossy(self.chunk())
    // }

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
    fn as_ref(&self) -> &[u8] {
        self.buffer.as_ref()
    }
}

impl Deref for  BodyReadingBuffer {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}


impl Buf for BodyReadingBuffer {
    fn remaining(&self) -> usize {
        self.buffer.remaining()
    }

    fn chunk(&self) -> &[u8] {
        self.buffer.chunk()
    }

    fn advance(&mut self, cnt: usize) {
        self.advanced_bytes +=cnt;
        self.buffer.advance(cnt)
    }
}





