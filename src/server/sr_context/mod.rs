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
use serde::{Deserialize, Serialize};
use serde::ser::Error;
use tokio::net::TcpStream;
use crate::http::{FileRSender, Http1Sender, Http2Sender, HttpSender, HttpSenderTrait, request::IncomingRequest, ResponseData, SendingFileResults};
use crate::http::request::{ DynamicBodyMap, FormDataAll, HeapXWWWFormUrlEncoded, Http1Getter, Http2Getter, HttpGetter, HttpGetterTrait, IBody, IBodyChunks, ParsingBodyMechanism, ParsingBodyResults};
use crate::http::request::ParsingBodyResults::{Chunked, FullBody};
use crate::http::status_code::HttpStatusCode;
use crate::server::{CapsuleWaterController, MiddlewareCallback, MiddlewareResult};
use crate::server::connection::BodyReadingBuffer;
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
    reading_buffer:&'a mut BodyReadingBuffer,
    path_query:Option<HashMap<String,String>>
}

 fn parse_query_string_to_map(query:Option<&str>,map:&mut Option<HashMap<String,String>>){
     let mut query = match query {
         None => { return }
         Some(q) => {q}
     };
     if query.starts_with("?") {
         if query.len() > 2 {
             query = &query[1..];
         }
     }
     println!("invoked {query}");
     let splitter = query.split("&");
     for q in splitter {
         let s :Vec<&str> = q.split("=").collect();
         if s.len() != 2 {continue}
         match map {
             None => {
                 let mut m = HashMap::new();
                 m.insert(s.first().unwrap().to_string(),s.last().unwrap().to_string());
                 *map = Some(m);
             }
             Some(m) => {
                 m.insert(s.first().unwrap().to_string(),s.last().unwrap().to_string());
             }
         }
     }
 }
impl<'a> Http2Context<'a> {

    pub (crate) fn new(request_batch:(Request<RecvStream>,SendResponse<Bytes>),reading_buffer:&'a mut BodyReadingBuffer)->Self{
        let mut path_query = None;
        parse_query_string_to_map(request_batch.0.uri().query(),&mut path_query);
        Http2Context {
            request_batch,
            content_length:None,
            reading_buffer,
            path_query
        }
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
    /// for holding data trough multiple middlewares and functions or handlers
    pub holder:Option<H>,
    protocol: Protocol<'a,HEADERS_COUNT,PATH_QUERY_COUNT>,
    peer:&'a SocketAddr,
    /// saving generic parameters injected with requested path
    pub path_params_map:Option<HashMap<String,String>>,
    body_bytes_holder:Option<Vec<u8>>
}


impl <'a,H:Send + 'static,const HEADERS_COUNT:usize
    ,const PATH_QUERY_COUNT:usize> HttpContext<'a,H,HEADERS_COUNT,PATH_QUERY_COUNT> {

    /// for getting connected socket Address
    /// # return `&SocketAddr`
    pub fn get_peer_socket(&self)->&SocketAddr {
        self.peer
    }
    pub (crate) fn new(
        protocol: Protocol<'a,HEADERS_COUNT,PATH_QUERY_COUNT>,
        socket:&'a SocketAddr
    )->
    HttpContext<'a,H,HEADERS_COUNT,PATH_QUERY_COUNT>
    {
        HttpContext {holder:None,protocol,peer:socket,path_params_map:None,body_bytes_holder:None}
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
    /// this function will make context allocate heap memory for holding bytes together
    /// and not use the same buffer bytes because the buffer would be busy handling another request and
    /// can not be interpreted by the current thread, and also we need to hold the body bytes for you
    /// to the next use
    /// # Note :
    /// this function is generate body bytes in heap and it`s less efficient than using getter() function
    /// but also using getter is much more code you need to write , and also you need to know what are you doing
    /// to make it super efficient
    /// [HttpGetter]
    ///
    /// to use getter you can call
    /// ```shell
    /// context.getter()
    /// ```
    pub async fn get_body_full_bytes(&mut self)->Result<Option<&Vec<u8>>,WaterErrors>{
        if self.body_bytes_holder.is_some() {
            return Ok(self.body_bytes_holder.as_ref())
        }
        let mut getter = self.getter();
        let puller = getter.get_body_by_mechanism(
            ParsingBodyMechanism::JustBytes
        ).await;
        match puller {
            Chunked(chunks) => {
                if let IBodyChunks::Bytes(mut puller) = chunks {
                    let mut bytes = vec![];
                    if let Ok(()) = puller.on_chunk(
                        |c| {
                            bytes.extend_from_slice(c);
                            return Ok(())
                        }
                    ).await {
                        self.body_bytes_holder = Some(bytes);
                        return  Ok(self.body_bytes_holder.as_ref())
                    }
                }

            }
            FullBody(body) => {
                if let IBody::Bytes(body_bytes) = body {
                    self.body_bytes_holder = Some(body_bytes.to_vec());
                    return  Ok(self.body_bytes_holder.as_ref())
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


    /// for getting the body parsed as [Deserialize] json struct
    pub async fn get_body_as_json<'b,V:Deserialize<'b>>(&mut self)->Result<V,serde_json::Error>{
        let body = self.get_body_full_bytes().await;
        match body {
            Ok(Some(data)) => {

                let res = serde_json::from_slice(
                    unsafe  {
                        (data.as_ref() as *const [u8]).as_ref().unwrap()
                    }
                );
                 return res
            }
            _ => {Err(serde_json::Error::custom("can not retrieve incoming body bytes"))}
        }
    }


    /// getting body as multipart form data [FormDataAll]
    pub async fn get_body_as_multipart(&mut self)->Result<FormDataAll,WaterErrors>{
        let mut body = self.getter();
        let  body = body.get_body_by_mechanism(
            ParsingBodyMechanism::FormData
        ).await;
         match body {
            Chunked(a)=>{
                if let IBodyChunks::FormData( data) = a {
                    if let Ok( data) = data.to_form_data_all().await {
                        return  Ok(data)
                    }
                }
            }
            FullBody(f)=> {
                if let IBody::MultiPartFormData(data) = f {
                    return Ok(data)
                }
            }
            _ => {}
        };

        return  Err(
            WaterErrors::Http(HttpStatusCode::BAD_REQUEST)
        )

    }



    /// returning dynamic trait that would be for getting values from body using
    /// keys
    pub async fn get_body_map(&mut self)-> Result<DynamicBodyMap,WaterErrors> {

        let mut getter = self.getter();
        match getter.get_body().await {
            Chunked(bo) => {

                match bo {
                    IBodyChunks::FormData(mut multipart_form) => {
                        let mut fu = FormDataAll::new();
                        if multipart_form.on_field_detected(
                            |field,data|{
                                fu.push(field,data);
                                Ok(None)
                            }
                        ).await .is_ok(){

                        }
                    }
                    _ =>{}
                }

            }



            FullBody(full_body) => {
                match full_body {

                    IBody::MultiPartFormData(data) => {
                        return Ok(DynamicBodyMap::FormField(data))
                    }
                    IBody::XWWWFormUrlEncoded(data) => {
                        return Ok(DynamicBodyMap::Xww(
                            HeapXWWWFormUrlEncoded::new(
                                &data
                            )
                        ))
                    }
                    _ => {}
                }
            }
            _ => {}
        };
        Err::<DynamicBodyMap,WaterErrors<'a>>(
            WaterErrors::Http(
                HttpStatusCode::BAD_REQUEST
            )
        )
    }

    /// this function return the original data on the request buffer on memory ,and
    /// it is very fast and memory safe function ,and it has zero allocation for data
    pub fn get_from_headers_as_bytes(&'a self,key:&str)->Option<&[u8]>{
        match &self.protocol {
            Protocol::Http2(h2) => {
                return  h2.get_from_headers(key)
            }
            Protocol::Http1(h1) => {
                h1.request.headers().get_as_bytes(key)
            }
        }
    }

    /// this function just convert the bytes that come from [self.get_from_headers]
    /// to `Cow<str>`
    ///  return [`Cow<str>`]
    ///
    /// please note that rust could allocate new memory for holding [Cow] when it`s converted
    /// from clean bytes
    pub fn get_from_headers(&'a self,key:&str)->Option<Cow<str>>{
        if let Some(data) = self.get_from_headers_as_bytes(key) {
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
    pub fn sender(&mut self)->HttpSender<'_,'a,HEADERS_COUNT,PATH_QUERY_COUNT>{
        return match &mut self.protocol {
            Protocol::Http2(h2) => {
                HttpSender::H2(Http2Sender::new(h2))
            }
            Protocol::Http1(h1) => {
               HttpSender::H1( Http1Sender::new(h1))
            }
        }
    }


    /// for sending [`&str`] values to the client
    pub async fn send_str(&mut self,value:&'static str)->Result<(),()>{
        let mut sender = self.sender();
        sender.send_str(value).await
    }

    /// for sending back status code as final response
    pub async fn send_status_code_as_final_response(&mut self,status:HttpStatusCode<'_>){
        let mut sender = self.sender();
        sender.send_status_code(status);
        _=sender.send_data_as_final_response(ResponseData::Str("")).await;
    }


    /// for sending html text
    /// this function is basically set the content type of http response to text/html
    /// to let the browsers or the client knows what is coming
    pub async fn send_html_text(&mut self,value:&str)->Result<(),()>{
        let mut sender = self.sender();
        sender.set_header("Content-Type","Text/html");
        sender.send_data_as_final_response(ResponseData::Slice(value.as_bytes())).await
    }

    /// for sending json data
    pub async fn send_json(&mut self,json:&impl Serialize)->serde_json::Result<()>{
        let mut sender = self.sender();
        return sender.send_json(json).await;
    }


    /// for sending normal str data without static lifetime
    pub async fn send_string_slice(&mut self,value:&str)->Result<(),()>{
        let mut sender = self.sender();
        sender.send_status_code(HttpStatusCode::OK);
        sender.send_data_as_final_response(
            ResponseData::Slice(value.as_bytes())
        ).await
    }


    /// for returning redirect response to the client
    pub async fn redirect(&mut self,url:&str)->Result<(),()>{
        let mut sender = self.sender();
        sender.send_status_code(HttpStatusCode::TEMPORARY_REDIRECT);
        sender.set_header("Location",url);
        sender.send_data_as_final_response(ResponseData::Slice(&[])).await
    }


    /// getter is for getting data from incoming request like body and request quires
    // pub fn  getter<'b>(&'b mut self) ->HttpGetter<'b,'a,Stream, HEADERS_COUNT, PATH_QUERY_COUNT> {
    //     HttpGetter::new(&mut self.protocol)
    // }


    /// for sending files
    /// this function auto support for sending videos
    pub async fn send_file(&mut self,mut file:FileRSender<'_>)->SendingFileResults{
        if !file.path.exists() { return SendingFileResults::FileNotFound}
        let range = self.get_from_headers("Range");
        if let Some(range) = range {
            let mut range = range.split("=").last().unwrap_or("").split("-");
            let mut start = None;
            let mut end = None;
            if let Some(s) = range.next() {
                if let Ok(s) = s.parse::<usize>() {
                    start = Some(s);
                }
            } else {}
            if let Some(e) = range.next() {
                if let Ok(e) = e.parse::<usize>() {
                    end = Some(e);
                }
            }
            file.set_bytes_range(start,end);
        }
        let mut  sender = self.sender();
         sender.send_file(file).await
    }


    /// getting the path from incoming request
    pub fn path(&self)->&str{
        match &self.protocol {
            Protocol::Http2(h2) => {
                let ref request = h2.request_batch.0;
                request.uri().path()
            }
            Protocol::Http1(h1) => {
                h1.request.path()
            }
        }
    }

    /// getting incoming request method
    pub fn method(&self)->&str{
        match &self.protocol {
            Protocol::Http2(h2) => {
                let ref request = h2.request_batch.0;
                request.method().as_str()
            }
            Protocol::Http1(h1) => {
               h1.request.method()
            }
        }
    }


    /// getting from path generic injected parameters
    /// like <http://example.com/test/{id}>
    /// here id is a generic parameter
    pub fn ____get_from_generic_path_params(&'a self,key:&str)->Option<&'a String>{
        if let Some(p) = &self.path_params_map {
            return p.get(key)
        }
        None
    }


    /// getting data from path query
    pub  fn get_from_path_query(&self,key:&str)->Option<Cow<str>>{
        match &self.protocol {
            Protocol::Http2(h2) => {
                if let Some(pq)  = h2.path_query.as_ref() {
                    if let Some(v) = pq.get(key){
                        return Some(Cow::from(v.to_string()));
                    }
                }
                None
            }
            Protocol::Http1(h1) => {
                h1.request.get_from_path_query(key)
            }
        }
    }

    pub (crate) async fn serve(
        &mut self,
        controller:&'static  CapsuleWaterController<H,HEADERS_COUNT,PATH_QUERY_COUNT>

    )->
    ServingRequestResults
    {

        let content_length = self.content_length().copied();
        let method = self.method();
        if let Some(content_length )  = content_length {
            if (content_length > 0) && ["GET","HEAD","DELETE","TRACE"].contains(&method) {
                let mut sender = self.sender();
                sender.send_status_code(HttpStatusCode::BAD_REQUEST);
                _=sender.write_custom_bytes(&[]).await;
                return  ServingRequestResults::Stop;
            }
        }
        let path = self.path();
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
            _=sender.write_custom_bytes(&[]).await;
            return  ServingRequestResults::Stop;
        }

        #[cfg(feature = "debugging")]
        {
            use tracing::info;
            info!("request has been served {:?}",self.peer);
        }
        ServingRequestResults::Done
    }






}



#[derive(Debug)]

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

/// defining http1 implementations for request context
#[doc(hidden)]
pub struct Http1Context<'a,const HEADERS_COUNT:usize
    ,const PATH_QUERY_COUNT:usize> {
  pub request: IncomingRequest<'a, HEADERS_COUNT, PATH_QUERY_COUNT>,
  pub (crate) stream:&'a mut HttpStream,
  pub (crate) body_reading_buffer:&'a mut BodyReadingBuffer,
  pub(crate) response_buffer:&'a mut BytesMut,
  pub (crate) left_bytes:&'a [u8],
}


impl <'a,const HEADERS_COUNT:usize
    ,const PATH_QUERY_COUNT:usize> Http1Context<'a,HEADERS_COUNT,PATH_QUERY_COUNT> {
    pub (crate) fn new(
                       stream:&'a mut HttpStream,
                       response_buffer:&'a mut BytesMut,
                       body_reading_buffer:&'a mut BodyReadingBuffer,
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


}


pub (crate) enum ServingRequestResults{
    Stop,
    Done,
}




