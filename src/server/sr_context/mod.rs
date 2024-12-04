use std::borrow::Cow;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll};
use bytes::{Bytes, BytesMut};
use h2::RecvStream;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use h2::server::SendResponse;
use http::Request;
use tokio::net::{ TcpStream};
use crate::http::{FileRSender, Http1Sender, Http2Sender, HttpSender, HttpSenderTrait, request::IncomingRequest};
use crate::http::request::{Http1Getter, Http2Getter, HttpGetter, HttpGetterTrait, IBody, IBodyChunks, ParsingBodyMechanism, ParsingBodyResults};
use crate::http::status_code::HttpStatusCode;
use crate::server::connection::handle_responding;
use crate::server::{CapsuleWaterController, EACH_REQUEST_BODY_READING_BUFFER, MiddlewareCallback, MiddlewareResult};
use crate::server::errors::{ServerError, WaterErrors};

pub (crate) enum Protocol<'a,const HEADERS_COUNT:usize
    ,const PATH_QUERY_COUNT:usize>{
    Http2(Http2Context<'a,>),
    Http1(Http1Context<'a,HEADERS_COUNT,PATH_QUERY_COUNT>)
}
impl <'a,const HEADERS_COUNT:usize
    ,const PATH_QUERY_COUNT:usize>  Protocol<'a,HEADERS_COUNT,PATH_QUERY_COUNT> {

    pub (crate) fn from_http1_context(context:Http1Context<'a,HEADERS_COUNT,PATH_QUERY_COUNT>)
    ->Protocol<'a,HEADERS_COUNT,PATH_QUERY_COUNT>{
        Protocol::Http1(context)
    }


    pub (crate) fn from_http2_context(context:Http2Context<'a,>)
    ->Protocol<'a, HEADERS_COUNT, PATH_QUERY_COUNT> {
        Protocol::Http2(context)
    }
}



pub (crate) struct  Http2Context<'a> {
    pub request_batch:(Request<RecvStream>,SendResponse<Bytes>),
    content_length:Option<usize>,
    reading_buffer:&'a mut BytesMut
}

impl<'a> Http2Context<'a> {

    pub (crate) fn new(request_batch:(Request<RecvStream>,SendResponse<Bytes>),reading_buffer:&'a mut BytesMut)->Self{
        Http2Context {request_batch,content_length:None,reading_buffer}
    }

    pub (crate) fn get_from_headers(&self,key:&str)->Option<&[u8]>{
        let ref request =  self.request_batch.0;
        if let Some(cl) = request.headers().get(key) {
           return Some(cl.as_bytes())
        }
        None
    }
    pub (crate) fn get_from_headers_as_str(&self,key:&str)->Option<&str>{
        let ref request =  self.request_batch.0;
        if let Some(cl) = request.headers().get(key) {
            if let Ok(cl) = cl.to_str() {
                return Some(cl);
            }
        }
        None
    }
    pub (crate) fn content_length(&mut self)->Option<&usize>{
        if self.content_length.is_some() { return  self.content_length.as_ref();}
      if let Some(cl) = self.get_from_headers_as_str("Content-Length") {
          if let Ok(cl) = cl.parse::<usize>() {
               self.content_length =   Some(cl);
              return  self.content_length.as_ref();
          }
      }
        None
    }

    /// getter is for getting data from incoming request like body and request quires
    pub fn getter(&mut self)->Http2Getter<'_>{
        let  content_length = *self.content_length().unwrap_or(&0);
         Http2Getter {
             batch:&mut self.request_batch.0,
             content_length,
             reading_buffer:self.reading_buffer
         }
    }
}



/// http context is a handler wrapper for http requests and operations
/// that help providing very fast handling framework for all http requests types
pub struct HttpContext<
    'a,
    H,
    const HEADERS_COUNT:usize
    ,const PATH_QUERY_COUNT:usize>
{
    holder:Option<H>,
    protocol: Protocol<'a,HEADERS_COUNT,PATH_QUERY_COUNT>,
    peer:&'a SocketAddr,
    /// saving generic parameters injected with requested path
    pub path_params_map:Option<HashMap<String,String>>,
}


impl <'a,H:Send + 'static,const HEADERS_COUNT:usize
    ,const PATH_QUERY_COUNT:usize> HttpContext<'a,H,HEADERS_COUNT,PATH_QUERY_COUNT> {

    pub (crate) fn new(
        protocol: Protocol<'a,HEADERS_COUNT,PATH_QUERY_COUNT>,
        socket:&'a SocketAddr
    )->
    HttpContext<'a,H,HEADERS_COUNT,PATH_QUERY_COUNT>
    {
        HttpContext {holder:None,protocol,peer:socket,path_params_map:None}
    }


    /// returning getter struct for getting info from incoming request very easley and
    /// for making a less ram usage for better performance
    pub fn getter<'f>(&'f mut self)->HttpGetter<'f,'a,HEADERS_COUNT,PATH_QUERY_COUNT>{
        match &mut self.protocol {
            Protocol::Http2(h2) => {HttpGetter::H2(h2.getter())}
            Protocol::Http1(h1) => {HttpGetter::H1(h1.getter())}
        }
    }


    /// getting the full body bytes from stream
    /// # Note :
    /// this function is generate body bytes in heap and it`s less efficient than using getter() function
    /// [HttpGetter]
    ///
    /// to use getter you can call
    /// ```shell
    /// context.getter()
    /// ```
    pub async fn get_body_as_full_bytes<T:serde::Deserialize<'a>>(&mut self)->Result<Option<Bytes>,WaterErrors>{
        let mut getter = self.getter();
        let puller = getter.get_body_by_mechanism(
            ParsingBodyMechanism::JustBytes
        ).await;
        match puller {
            ParsingBodyResults::Chunked(chunks) => {
                if let IBodyChunks::Bytes(mut puller) = chunks {
                    let mut bytes = vec![];
                    if let Ok(()) = puller.on_chunk(
                        |c| {
                            bytes.extend_from_slice(c);
                            return Ok(())
                        }
                    ).await {
                       return  Ok(Some(Bytes::copy_from_slice(&bytes)))
                    }
                }

            }
            ParsingBodyResults::FullBody(body) => {
                if let IBody::Bytes(body_bytes) = body {
                    return Ok(Some(Bytes::copy_from_slice(body_bytes)))
                }
                return Err(WaterErrors::Http(HttpStatusCode::BAD_REQUEST))
            }
            ParsingBodyResults::None => {return  Ok(None)}
            ParsingBodyResults::Err(_) => { return
             Err(
                 WaterErrors::Server(ServerError::HANDLING_INCOMING_BODY_ERROR)
             )
            }
        }
        return Err( WaterErrors::Server(ServerError::HANDLING_INCOMING_BODY_ERROR))
    }

    /// this function return the original data on the request buffer on memory ,and
    /// it is very fast and memory safe function ,and it has zero allocation for data
    pub fn get_from_headers(&'a self,key:&str)->Option<&[u8]>{
        match &self.protocol {
            Protocol::Http2(h2) => {
                return  h2.get_from_headers(key)
            }
            Protocol::Http1(h1) => {
                h1.request.headers.get_as_bytes(key.as_bytes())
            }
        }
    }

    /// this function just convert the bytes that come from [self.get_from_headers]
    /// to Cow<str>
    /// so it`s return [Cow<str>]
    ///
    /// please note that rust could allocate new memory for holding [Cow] when it`s converted
    /// from clean bytes
    pub fn get_from_headers_as_str(&'a self,key:&str)->Option<Cow<str>>{
        if let Some(data) = self.get_from_headers(key) {
            return Some(String::from_utf8_lossy(data))
        }
        None
    }


    /// getting content body length if request has body it will return [usize] as content length
    /// else it's returning [None]
    pub fn content_length(&mut self) ->Option<&usize>{
        match &mut self.protocol {
            Protocol::Http2(h2) => {
                h2.content_length()
            }
            Protocol::Http1(h1) => {
                h1.content_length()
            }
        }
    }
    /// getting sender for sending all types of data to client
    pub fn sender(&mut self)->HttpSender<'_>{
        return match &mut self.protocol {
            Protocol::Http2(h2) => {
                HttpSender::H2(Http2Sender::new(&mut h2.request_batch))
            }
            Protocol::Http1(h1) => {
               HttpSender::H1( Http1Sender::new(&mut h1.response_buffer,h1.stream))
            }
        }
    }



    /// getter is for getting data from incoming request like body and request quires
    // pub fn  getter<'b>(&'b mut self) ->HttpGetter<'b,'a,Stream, HEADERS_COUNT, PATH_QUERY_COUNT> {
    //     HttpGetter::new(&mut self.protocol)
    // }


    pub async fn send_file(&mut self,mut file:FileRSender<'a>){
        let range = self.get_from_headers_as_str("Range");
        if let Some(range) = range {
            let mut range = range.split("=").last().unwrap_or("").split("-");
            let mut start = None;
            let mut end = None;
            if let Some(s) = range.next() {
                if let Ok(s) = s.parse::<usize>() {
                    start = Some(s);
                }
            }
            if let Some(e) = range.next() {
                if let Ok(e) = e.parse::<usize>() {
                    end = Some(e);
                }
            }
            file.set_bytes_range(start,end);
        }
        let mut  sender = self.sender();

        _= sender.send_file(file).await;
    }


    /// getting the path from incoming request
    pub fn path(&'a self)->Cow<str>{
        match &self.protocol {
            Protocol::Http2(h2) => {
                let ref request = h2.request_batch.0;
                Cow::from(request.uri().path())
            }
            Protocol::Http1(h1) => {
                h1.request.path()
            }
        }
    }

    /// getting incoming request method
    pub fn method(&'a self)->Cow<str>{
        match &self.protocol {
            Protocol::Http2(h2) => {
                let ref request = h2.request_batch.0;
                Cow::from(request.method().as_str())
            }
            Protocol::Http1(h1) => {
                h1.request.method()
            }
        }
    }


    /// getting from path generic injected parameters
    /// like [http://example.com/test/{id}]
    /// here id is a generic parameter
    pub fn get_from_generic_path_params(&'a self,key:&str)->Option<&'a String>{
        if let Some(p) = &self.path_params_map {
            return p.get(key)
        }
        None
    }

    pub (crate) async fn serve(
        &mut self,
        controller:&'static  CapsuleWaterController<H,HEADERS_COUNT,PATH_QUERY_COUNT>

    )->
    ServingRequestResults
    {

        let path = self.path();
        let method = self.method();
        let f = controller.find_function(path.as_ref(),method.as_ref());
        if let Some((controller,func,map)) = f {
            self.path_params_map = map;
            let mut middlewares:Vec<&'static MiddlewareCallback<H,HEADERS_COUNT,PATH_QUERY_COUNT>> = vec![];
            controller.push_all_ancestors_middlewares(&mut middlewares);
            for m in middlewares {
               match  m(self).await {
                   MiddlewareResult::Pass => {
                       continue;
                   }
                   MiddlewareResult::Stop => {
                       return ServingRequestResults::Done
                   }
               }
            }
            func(self).await;
        } else {
            let mut sender = self.sender();
            sender.send_status_code(HttpStatusCode::NOT_FOUND);
        }

        ServingRequestResults::Done
    }




}





pub (crate)enum HttpStream {
    AsyncSecure(tokio_rustls::server::TlsStream<TcpStream>),
    Async(TcpStream),
}

impl AsyncWrite for HttpStream {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<std::io::Result<usize>> {
        match self.get_mut() {
            HttpStream::AsyncSecure(stream) => Pin::new(stream).poll_write(cx, buf),
            HttpStream::Async(stream) => Pin::new(stream).poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            HttpStream::AsyncSecure(stream) => Pin::new(stream).poll_flush(cx),
            HttpStream::Async(stream) => Pin::new(stream).poll_flush(cx),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            HttpStream::AsyncSecure(stream) => Pin::new(stream).poll_shutdown(cx),
            HttpStream::Async(stream) => Pin::new(stream).poll_shutdown(cx),
        }
    }
}

impl AsyncRead for HttpStream {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            HttpStream::AsyncSecure(stream) => {
                Pin::new(stream).poll_read(cx,buf)
            }
            HttpStream::Async(stream) => {
                Pin::new(stream).poll_read(cx,buf)

            }
        }
    }
}

pub struct Http1Context<'a,const HEADERS_COUNT:usize
    ,const PATH_QUERY_COUNT:usize> {
  pub request: IncomingRequest<'a, HEADERS_COUNT, PATH_QUERY_COUNT>,
  pub (crate) stream:&'a mut HttpStream,
  pub (crate) body_reading_buffer:&'a mut BytesMut,
  response_buffer:&'a mut BytesMut,
  pub (crate) left_bytes:&'a [u8],
}


impl <'a,const HEADERS_COUNT:usize
    ,const PATH_QUERY_COUNT:usize> Http1Context<'a,HEADERS_COUNT,PATH_QUERY_COUNT> {
    pub (crate) fn new(
                       stream:&'a mut HttpStream,
                       response_buffer:&'a mut BytesMut,
                       body_reading_buffer:&'a mut BytesMut,
                       left_bytes:&'a[u8],
                       request:IncomingRequest<'a,HEADERS_COUNT,PATH_QUERY_COUNT>,

                       )->Http1Context<'a,HEADERS_COUNT,PATH_QUERY_COUNT> {
        Self {
            request,
            body_reading_buffer,
            response_buffer,
            stream,
            left_bytes,
        }
    }






    pub(crate) async fn flush_response_buffer(&mut self)->Result<(),&str>{
        match self.stream {
            HttpStream::AsyncSecure(stream) => {
                handle_responding(self.response_buffer,stream).await

            }
            HttpStream::Async(s) => {
                handle_responding(self.response_buffer,s).await

            }
        }
    }


    /// getting content-length if the current request
    #[inline]
    pub fn content_length(&self)->Option<&usize>{
        self.request.content_length()
    }




    /// getter is for getting data from incoming request like body and request quires
    pub fn getter<'b>(&'b mut self)->Http1Getter<'b,'a,HEADERS_COUNT,PATH_QUERY_COUNT>{
        Http1Getter {
            body_reading_buffer:  self.body_reading_buffer,
            left_bytes:self.left_bytes,
            stream: self.stream,
            request: &mut self.request,
        }
    }

    /// for getting the body of  incoming stream by default mechanism
    /// [ParsingBodyMechanism::Default]
    pub async fn get_body(&mut self)->ParsingBodyResults<'a>{
        self.get_body_by_mechanism(ParsingBodyMechanism::Default).await
    }

    /// for getting the body of incoming request using custom mechanism
    pub async fn get_body_by_mechanism(&mut self,mechanism:ParsingBodyMechanism)->ParsingBodyResults<'a>{
        if let Some(content_length) = self.request.content_length() {



            // let`s checks if request need to be handled alone
            if *content_length >= EACH_REQUEST_BODY_READING_BUFFER {
                if let Err(_) = self.flush_response_buffer().await {
                    return ParsingBodyResults::Err(
                        WaterErrors::Server(
                            ServerError::FLUSH_DATA_TOSTREAM_ERROR
                        )
                    );
                }
                // borrowing body reader buffer
                let body_buffer =&mut self.body_reading_buffer;


                // making sure that body buffer is clean
                if !body_buffer.is_empty() {body_buffer.clear();}
                // let`s handle getting data by chunks

            }



            match mechanism {
                ParsingBodyMechanism::Default => {}
                ParsingBodyMechanism::JustBytes => {}
                ParsingBodyMechanism::FormData => {}
                ParsingBodyMechanism::XWWWFormData => {}
            }
        }
        return ParsingBodyResults::Err(
            WaterErrors::Server(
                ServerError::HANDLING_INCOMING_BODY_ERROR
            )
        )
    }

}


pub (crate) enum ServingRequestResults{
    Stop,
    Done,
}




