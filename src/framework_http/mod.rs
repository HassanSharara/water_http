
// #![allow(dead_code)]
// #![allow(unused)]
#![allow(unused_assignments)]
pub mod request;
pub mod response;

use h2::{server::SendResponse,SendStream, RecvStream};
use http::{HeaderName, HeaderValue};
pub use response::*;
pub use request::*;
use serde::Serialize;
use std::{collections::HashMap, net::SocketAddr, str::FromStr, vec};
use std::ffi::{OsStr, OsString};
use std::io::SeekFrom;
use std::path::Path;
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::TcpStream};
use tokio::io::{AsyncSeekExt};
use nom::{AsBytes, Parser};
use tokio_util::bytes::{Bytes, BytesMut};
use crate::framework_http::multipart_form::{HttpMultiPartFormDataField, parse_body_to_list_of_multipart_fields};
use crate::framework_http::x_www_form_urlencoded::XWWWFormUrlEncoded;

type IncomeBody =Option<Option<HttpIncomeBody>>;
const IMPORTANT_HEADERS :[&str;7] = [
    "Cookie",
    "Accept-Encoding",
    "Accept",
    "Connection",
    "Content-Length",
    "Content-Type",
    "Range"
];
const SIZE_OF_READ_WRITE_CHUNK:usize = 5200;
const SIZE_OF_FILES_WRITE_CHUNK:u64 = 100000;
#[allow(unused)]
pub struct HttpContext<DataHolderGeneric:Send> {
    pub protocol:Protocol,
    bytes_sent:usize,
    pub data_holder:Option<DataHolderGeneric>,
    pub path_params_map:HashMap<String,String>,
    body:IncomeBody,
}

unsafe impl<T:Send>  Send for HttpContext<T> {
}
 pub struct Http1 {
    _peer:(TcpStream,SocketAddr),
     pub request:Request,
    _extra_bytes_from_headers:Vec<u8>,
    _header_sent:bool,
}

type  Http2Request = http::Request<RecvStream>;
type  Http2ResponseSender = SendResponse<Bytes>;
pub struct Http2 {
    pub cached_headers:HttpHeadersMap,
    pub request:Http2Request,
    pub send_response:Http2ResponseSender,
    pub body_sender:Option<SendStream<Bytes>>,
}
pub enum Protocol {
    Http1(Http1),
    Http2(Http2)
}



impl<DataHolderGeneric:Send> HttpContext<DataHolderGeneric> {

   fn from_http2_connection(
        request:Http2Request,
        send_response:Http2ResponseSender)->Result<Self,String>{
        Ok(HttpContext{
        protocol:Protocol::Http2(Http2 { 
            cached_headers:HashMap::new(),
            request,
            send_response,
            body_sender:None,
        }),
        bytes_sent:0,
        data_holder:None,
            body:None,
        path_params_map:HashMap::new()
      })
    }

    pub fn get_route_path(&self)->&str{
       return match &self.protocol {
            Protocol::Http1(h1) => {
                return h1.request.path.as_str()
            }
            Protocol::Http2(h2) => {
                h2.request.uri().path()
            }
        };
    }


    pub fn get_method(&self)->&str{
       return match &self.protocol {
            Protocol::Http1(h1) => {
                return h1.request.method.as_str();
            }
            Protocol::Http2(h2) => {
                h2.request.method().as_str()
            }
        };
    }

   fn get_request_content_boundary(&mut self)->Option<&[u8]>{
       let _r = self.get_from_headers("Content-Type");
       const PATTERN :&str = "boundary=";
       if let Some(ve) = _r {
           for i in ve {
               for value in i {
                   if !value.contains(PATTERN){
                       continue;
                   }
                   let splitter = value.split(PATTERN).last();
                   if let Some(_boundary) = splitter { return  Some(_boundary.as_bytes());}
               }
           }
       }
       None
   }
  async fn wait_for_another_request(&mut self){
        self.refresh_stream().await;
    }

    pub fn is_http1(&self)->bool{
        if let Protocol::Http1(_) = self.protocol {
            return  true;
        }
        false
    }
    async fn refresh_stream(&mut self){
        self.bytes_sent = 0;
        if let Protocol::Http1(protocol) = &mut self.protocol {
            protocol._extra_bytes_from_headers.clear();
            protocol._header_sent = false;
            if let Ok((request,extra_body_bytes)) =
                build_headers(&mut protocol._peer.0,None).await {
                protocol.request = request;
                protocol._extra_bytes_from_headers = extra_body_bytes;
            }

        }
    }

    async  fn from_http1_connection(mut _peer:(TcpStream,SocketAddr),
     required_headers:Option<Vec<&str>>)->Result<Self,String>{
        let (request,extra_body_bytes) =
         build_headers(&mut _peer.0,required_headers).await?;
        let context = HttpContext {
            protocol:Protocol::Http1(
                Http1 {
                    _header_sent:false,
                    _peer,
                    request,
                   _extra_bytes_from_headers:extra_body_bytes,
                }
            ),
            bytes_sent:0,
            data_holder:None,
            body:None,
            path_params_map:HashMap::new()
        };
        return Ok(context);        
    }

    pub fn get_all_headers(&mut self )->&HttpHeadersMap{
        match &mut self.protocol {
            Protocol::Http1(h1) => {
                return &h1.request.headers_map;
            }
            Protocol::Http2(h2) => {
                {
                    let all_keys  = h2.request.headers();
                    for (k,value) in all_keys {
                        let key = k.to_string();
                        if h2.cached_headers.contains_key(key.as_str()){
                            continue;
                        }
                        let res = convert_string_value_to_vec_of_strings(
                            &String::from_utf8_lossy(value.as_bytes())
                        );
                        h2.cached_headers.insert(key,res);
                    }
                };
                &h2.cached_headers
            }
        }
    }

    pub fn get_from_headers(&mut self,key:&str)->Option<&Vec<Vec<String>>>{
         match &mut self.protocol {
            Protocol::Http1(_p1) => {
                return _p1.request.headers_map.get(key);
            },
            Protocol::Http2(_p2) => {
                let y = _p2.cached_headers.contains_key(key);
                if y {
                    return _p2.cached_headers.get(key);
                }else{
                  Self::from_h2_protocol_to_valid_headers_map(key,_p2)
                }
            },
            }
    }
    fn from_h2_protocol_to_valid_headers_map<'a>(key:&str,_p2:&'a mut Http2)->Option<&'a Vec<Vec<String>>>{
        if let Some(_h) = _p2.request.headers().get(key) {
            let s = String::from_utf8_lossy(_h.as_bytes());
            let res = convert_string_value_to_vec_of_strings(&s);
            _p2.cached_headers.insert(key.to_owned(), res);
            return  _p2.cached_headers.get(key);
        }
        None
    }
    pub fn get_single_value_from_headers(&mut self,key:&str)->Option<&String>{
        if let Some(c) = self.get_from_headers(key) {
            if !c.is_empty() {
                let first = c.first().unwrap().first();
                if let Some(content_length_string) = first {
                   return Some(content_length_string);
                }
            }
        }

        None
    }
    pub fn get_from_header_as<T:FromStr>(&mut self,key:&str)->Option<T>{
        if let Some(value) = self.get_single_value_from_headers(key) {
            if let Ok ( v ) = value.parse::<T>() {
                return Some(v);
            }
        }

        None
    }
    async fn write_bytes(&mut self,bytes:&[u8],end_of_stream:bool)->Result<(),String>{
        match &mut self.protocol {
            Protocol::Http1(p) => {
                if let Ok(_) = p._peer.0.write_all(&bytes).await {
                    if  end_of_stream {
                        if let Err(_) = p._peer.0.flush().await {
                            return Err("can not Flushing All The Data in The Stream".to_string());
                        }
                    }
                   return Ok(());
                } 
            },
            Protocol::Http2(_p2) => {
                if let Some(_s) = &mut _p2.body_sender {
                    let bytes = bytes.to_vec();
                    let _ = _s.send_data(Bytes::from(bytes), end_of_stream);
                    return Ok(());
                }
                return Err("cant send h2 response".to_owned());
            },
        }
       
        Err("Cant Write Data to Stream".to_owned()) 
    }
    pub async fn send_headers(
        &mut self,
        mut headers:HttpResponseHeaders, )->Result<(),String>{

        if let Some(_connection) = self.get_from_header_as::<String>("Connection") {
            if _connection.to_lowercase() == "keep-alive" {
                headers.set_header_key_value("Connection","Keep-Alive");
            }
        }
        match  &mut self.protocol {
            Protocol::Http1(h1) => {
                if h1._header_sent {
                    return Ok(());
                }
                h1._header_sent = true;

                let bytes =  headers.to_bytes();
                self.write_bytes(&bytes,false).await?;
                return Ok(());
            },
            Protocol::Http2(h2) => {
                if let Some(_) = h2.body_sender {
                    return Ok(());
                }
                let  mut response = http::Response::builder()
                .status(headers.first_line.status.code);
                let _headers = response.headers_mut();
                if let Some(h) = _headers {
                    for (k,v) in headers.headers.iter() {
                        let key =HeaderName::from_str(k);
                        let value =HeaderValue::from_str(v);
                        if let Err(_) = key {
                            return Err(format!("Can not form header name with {}",k));
                        }
                        if let Err(_) = value {
                            return Err(format!("Can not form header name with {}",v));
                        }
                        h.append(key.unwrap(), value.unwrap());
                    }
                }
                let r = response.body(());
                if let Ok(_r) = r {
                    let body_sender = h2.send_response.send_response(_r,false);
                    if let Ok(sender) = body_sender {
                        h2.body_sender = Some(sender);
                        return Ok(());
                    }
                }},
        }
        Err("None Of Protocols Succeed".to_owned())
    }

    #[async_recursion::async_recursion]
    pub async fn send_data(&mut self,bytes:&[u8],
                           end_of_stream:bool)->Result<(),String>{
        let bytes_length = bytes.len();
        match &mut self.protocol {
            Protocol::Http1(h1) => {
                if !h1._header_sent {
                let h = if end_of_stream {
                    HttpResponseHeaders::success_with_content_length(
                            bytes_length
                        )} else {
                    HttpResponseHeaders::success()
                };
                    self.send_headers(h).await?;
                    if let Ok(_) = self.write_bytes(bytes, end_of_stream).await {
                        self.bytes_sent += bytes_length;
                        return  Ok(());
                    }
                } else {
                    if let Ok(_) = self.write_bytes(bytes, end_of_stream).await {
                        self.bytes_sent += bytes_length;
                        return  Ok(());
                    }
                }
            },
            Protocol::Http2(h2) => {
                    
                  match &mut h2.body_sender {
                    Some(sender) => {
                        let bytes = bytes.to_vec();
                        match sender.send_data(Bytes::from(bytes), end_of_stream) {
                            Ok(_)=>{
                                self.bytes_sent += bytes_length;
                            },
                            Err(e)=>{
                                return Err(format!("{}",e));
                            }
                        }
                    },
                    None => {
                        if end_of_stream {
                            self.send_headers(HttpResponseHeaders::success_with_content_length(
                                bytes.len() 
                                + bytes_length
                            )).await?;
                        }
                        else {
                            self.send_headers(HttpResponseHeaders::success()).await?;
                        }
                        self.send_data(bytes,end_of_stream).await?;
                        return  Ok(());
                    },
                  }  
                
            },
        }
        Err("Cant Write Data to Stream".to_owned()) 
     }
     
   
    pub async fn send_string_data(&mut self,slice:&str,end_of_stream:bool)->Result<(),String>{
        self.send_data(slice.as_bytes(), end_of_stream).await?;
        return Ok(());
    }
   
    pub async fn send_json_data<T>(&mut self,value:&T,end_of_stream:bool)->Result<(),String> where T: ?Sized + Serialize {
        let v = serde_json::to_string(value);
        if let Ok(_v) = v {
            return self.send_data(_v.as_bytes(), end_of_stream).await;
        }
        Err("Can not convert json data to string".to_owned())
    }
    
    pub async fn body_as_chunks(&mut self,mut bytes_chunk: impl FnMut (&[u8]))->Result<(),String>{
        match &mut self.protocol {
            Protocol::Http1(p) => {
                let mut total_bytes_received = 0;

                if !p._extra_bytes_from_headers.is_empty() {
                    total_bytes_received += p._extra_bytes_from_headers.len();
                    bytes_chunk(&p._extra_bytes_from_headers);

                }
                if let Some(value) = p.request.headers_map.get("Content-Length") {
                    if !value.is_empty() && !value[0].is_empty(){
                      let length = (&value[0][0]).parse::<usize>();
                      if let Ok(length) = length {
                          if total_bytes_received >= length { return Ok(());}
                       let mut buf = BytesMut::with_capacity(SIZE_OF_READ_WRITE_CHUNK);
                        while let Ok(_c) = p._peer.0.read_buf(&mut buf).await {
                            total_bytes_received += _c;
                            bytes_chunk(&buf[.._c]);
                            if _c == 0 || total_bytes_received >= length  {
                                break;
                            }
                            buf.clear();
                        }
                      } else {
                        return Err("Can not Read Content Length from Http Request".to_string());
                      }
                      
                    }
                }
            },
            Protocol::Http2(ref mut h2) => {
               let  body = h2.request.body_mut();
                if let Some(body) = body.data().await {
                    if let Ok(bytes) = body {
                        bytes_chunk(&bytes);
                    }
                }
            },
        }
        Ok(())
    }

    pub async fn body_as_string(&mut self)->Result<Option<String>,String>{
        match self.whole_body_as_bytes().await {
            Ok(bytes) => {
                if let Some(bytes) = bytes {
                    if let Ok(_data) = String::from_utf8(bytes) {
                        return Ok(Some(_data));
                    }
                }
                return Err("Cant Creating String from this Body".to_string());
            },
            Err(_e) => Err(_e),
        }
    }

    pub async fn whole_body_as_bytes(&mut self)->Result<Option<Vec<u8>>,String>{
        let content_length: Option<usize> = self.get_from_header_as::<usize>("Content-Length");
        match &mut self.protocol {
            Protocol::Http1(p1) => {
                let mut bytes:Vec<u8> = vec![];
                bytes.append(&mut p1._extra_bytes_from_headers);
                if let Some(content_length) = content_length {
                    if bytes.len() >= content_length {
                        return Ok(Some(bytes));
                    }
                }else {
                    return Ok(None);
                }
                let mut buf = BytesMut::with_capacity(SIZE_OF_READ_WRITE_CHUNK);
                loop {
                    if let Ok(_s) = p1._peer.0.read_buf(&mut buf).await {
    
                        if  _s == 0 {
                            return  Ok(Some(bytes));
                        }
                        bytes.extend(&buf);
        
                        if _s < buf.capacity()   {
                            return  Ok(Some(bytes));
                        }
                        buf.clear();
                    }else {
                        return  Err("There An Error Happen While Reading Bytes From Stream".to_string());
                    }
                }
            }
            Protocol::Http2(_p2) => {
                let _body =  _p2.request.body_mut().data().await;
                if let None = _body {
                    return Ok(None);
                }
                else if let Some(body) = _body {
                    return if let Ok(body) = body {
                        Ok(Some(body.into()))
                    } else {
                        Err("Error reading body bytes".to_string())
                    }
                }
            },
            }
            Err("can not handle whole body as bytes".to_owned())
        }



     pub async fn get_body<'a>(&'a mut self )-> &'a Option<HttpIncomeBody>{
         match (self).body {
             None => {}
             Some(ref body) => {
                 return body;
             }
         };
         let body = self.serialized_body().await;
         return match body {
             None => {
                 self.body = Some(None);
                  &None
             }
             Some(body) => {
                 self.body = Some(Some(body));
                   self.body.as_ref().unwrap()
             }
         }
     }
     async fn serialized_body(&mut self)->Option<HttpIncomeBody>{
        let content_type = self.get_from_header_as::<String>("Content-Type");
        if let Some(content_type) = content_type {
            if content_type == "multipart/form-data" {
                let fields = parse_body_to_list_of_multipart_fields(self).await;
                return  HttpIncomeBody::MultiPartFormat(fields).into();
            }
            else if content_type == "application/x-www-form-urlencoded" {
                let body = self.whole_body_as_bytes().await;
                if let Ok(body) = body {
                    if let Some(body) = body {
                        let x_body = XWWWFormUrlEncoded::from_str(
                            &String::from_utf8_lossy(&body)
                        );
                        if let Ok(x_body)  = x_body{
                            return  HttpIncomeBody::XWWWForm(x_body).into();
                        }
                        return  HttpIncomeBody::Unit8Vec(body).into();
                    }
                }
            }
        }
        None
    }



    pub async fn send_file_as_response(&mut self,path:&str)
     ->Result<(),String>{
        let file_path = Path::new(path);
        if !file_path .exists() {
            let mut headers = HttpResponseHeaders::not_found_headers();
            let msg = b"the path is not satisfied ! ";
            headers.set_header_key_value(
                "Content-Type",
                "text/plain"
            );
            headers.set_header_key_value("Content-Length",msg.len());
            let _ = self.send_headers(headers).await;
            let _  = self.send_data(msg.as_bytes(),true).await;
        }
        let content_type = content_type_from_file_path(&file_path);
        let mut h = None;
        match content_type {
            None => {
                h = Some(HttpResponseHeaders::bad_request_headers());
                let  headers = h.as_mut().unwrap();
                headers.set_header_key_value("Content-Type","application/octet-stream");
                headers.set_header_key_value("Accept-Ranges","bytes");
                headers.set_header_key_value("Content-Disposition",
                                             format!("attachment; filename={:?}"
                                                     ,file_path.file_name().unwrap_or(
                                                     &OsStr::new(
                                                         &format!("file_downloaded.{:?}",
                                                                  file_path.extension().unwrap_or(
                                                                      &OsString::new()
                                                                  )
                                                         )
                                                     )
                                                 ))

                );
            }
            Some(content_type) => {
                h = Some(HttpResponseHeaders::success());
                let  headers = h.as_mut().unwrap();
                headers.set_header_key_value("Content-Type",content_type);
            }
        }

        let  file = tokio::fs::File::open(path).await;
        if let Ok(mut file) = file {
            let metadata = file.metadata().await;
            let mut file_size = 0_u64;

            // checking over file size
            if let Ok(metadata) = metadata {
                file_size = metadata.len();
            }
            else { return  Err("could not read total file size from file metadata".to_string()) ; }


            if let Some(headers) = h.as_mut() {
                let income_range = self.get_from_header_as::<String>("Range");
                return match income_range {
                    None => {
                        headers.set_header_key_value("Content-Length", file_size);
                        self.send_headers(h.unwrap()).await?;
                        let mut buffer = [0; 2000];
                        while let Ok(size) = file.read(&mut buffer).await {
                            if size == 0 {
                                break;
                            }
                            if size < buffer.len() {
                                return self.send_data(&buffer[..size], true).await;
                            }
                            self.send_data(&buffer[..size], false).await?;
                        }
                        Err("encounter error while sending file ".to_string())
                    }
                    Some(range) => {
                        let mut ranges = range.split(",").next().unwrap_or("")
                            .split("=").last().unwrap_or("").split("-");
                        let start = ranges.next().unwrap_or("").parse::<u64>().unwrap_or(0);
                        let end = ranges.next().unwrap_or("").parse::<u64>().unwrap_or_else(
                            |_| {
                                let factor  = start + SIZE_OF_FILES_WRITE_CHUNK;
                                if file_size >= factor {
                                    factor
                                } else {
                                    file_size
                                }
                            }
                        );

                        if start == end || start > end || end > file_size {
                            return Err("Ranges Not Satisfiable".to_string());
                        }
                        headers.change_first_line_to_partial_content();
                        let content_length = (end - start)  + 1  ;
                        headers.set_header_key_value("Content-Length", content_length );
                        headers.set_header_key_value("Access-Control-Allow-Origin","*");
                        headers.set_header_key_value("Content-Range",
                                                     format!("bytes {}-{}/{}", start, end, file_size - 1 )
                        );
                        if let Some(content_type) = content_type {
                            headers.set_header_key_value("Content-Type",content_type);
                        }
                        headers.set_header_key_value("Accept-Ranges","bytes");
                        if let Err(_) = file.seek(SeekFrom::Start(start)).await {
                            return Err("Could not Seek to this start range".to_string());
                        }
                        self.send_headers(h.unwrap()).await?;
                        let mut remaining = content_length as usize;
                        while remaining > 0 {

                            let mut buffer = Vec::with_capacity(SIZE_OF_FILES_WRITE_CHUNK as usize);
                            if let Ok(size) = file.read_buf(&mut buffer).await {
                                if size < 1 {
                                    break;
                                }
                                let to_send = size.min(remaining);
                                let _e = self.send_data(&buffer[..to_send],
                                               to_send >= remaining
                                ).await;
                                remaining -= to_send;
                                if remaining < 1 { return Ok(()); }
                            } else{
                                return  Err("can not send this file range".to_string());
                            }
                        }
                        Ok(())
                    }
                }
            }
        }
        
        Err("Can not Send this file".to_string())
    }
}

pub enum HttpIncomeBody {
    MultiPartFormat(Vec<(HttpMultiPartFormDataField,Vec<u8>)>),
    XWWWForm(XWWWFormUrlEncoded),
    Unit8Vec(Vec<u8>)
}
#[allow(unused)]
async fn build_headers(_peer:&mut TcpStream,mut req_headers:Option<Vec<&str>>)->Result<(Request,Vec<u8>), String> {
    if let Some(req_headers) = &mut req_headers {
        req_headers.extend(IMPORTANT_HEADERS);
    }
    let mut header_string = String::new();
    let mut buf = BytesMut::with_capacity(SIZE_OF_READ_WRITE_CHUNK);
    let mut v:Vec<u8> = vec![];
    loop {
        if let Ok(_s) = &_peer.read_buf(&mut buf).await {
            if _s == &0 {
                break;
            }
            if buf.ends_with(b"\r\n\r\n") {
                let string_result = String::from_utf8_lossy(&buf[..*_s]);
                header_string.push_str(&string_result);
                break;
            }

            let mut counter:u8 = 0 ;
            let mut _i:Option<usize> = None;

            for (index,byte) in buf.iter().enumerate() {
                match counter {
                    0 =>{
                       if *byte == 13 {
                        counter+=1;
                       } else {
                        counter = 0;
                       }
                    },
                    1 =>{
                        if *byte == 10 {
                            counter+=1;
                           } else {
                            counter = 0;
                           }
                    },
                    2 =>{
                        if *byte == 13{
                            counter+=1;
                           } else {
                            counter = 0;
                           }
                    },
                    3 =>{
                        if *byte == 10{
                            counter+=1;
                            _i = Some(index+1);
                            break;
                           } else {
                            counter = 0;
                           }
                    },
                    _ =>{}
                }
            }
          
            if let Some( index ) = _i {
                header_string.push_str(&String::from_utf8_lossy(&buf[..index]));
                v.extend(&buf[index..]);
                break;
            } else {
               header_string.push_str(&String::from_utf8_lossy(&buf[..*_s])); 
            }
            if _s < &buf.capacity()  {
                break;
            }
           
        }else {
            break;
        }
    }
    let request = Request::build_request(
                                         header_string,
                                         req_headers);
        if let Err(_s) = request  {
            return Err(_s);
        }
        return Ok((request.unwrap(),v));
}
pub use crate::configurations::HTTPFrameworkConfigs;

pub mod server_runner {
    use std::net::IpAddr;
    use std::net::SocketAddr;
    use  crate::framework_http::*;
    use h2::server;
    use  tokio::net::TcpListener;
    use tokio::net::TcpStream;
    use tokio::task::JoinHandle;
    use crate::structure::{HttpContextRController,context_route_function_finder};

    pub async fn start_server<DataHolderGeneric>(
        configurations:HTTPFrameworkConfigs,
        controllers:fn () -> &'static mut Vec<HttpContextRController<DataHolderGeneric>>
    )

    where DataHolderGeneric : Send{
        for controller in controllers() {
            controller.____insure_binding();
        }

        let mut workers = Vec::<JoinHandle<()>>::new();
        for address in &configurations.addresses {
            let mut addr = address.replace("[","").replace("]", "");
            if ! addr.contains(":") {
                addr = format!("{}",addr);
            }
            workers.push(tokio::spawn(async move {
                println!("listening on {}",addr);
                tcp_connections_threads_generator::<DataHolderGeneric>(&addr,controllers()).await;
            }));
        }

        for worker in workers {
            let _ = worker.await;
        }
    }

    async fn tcp_connections_threads_generator<DataHolderGeneric>(address:&str,
    controllers:&'static Vec<HttpContextRController<DataHolderGeneric>>
    )
    where DataHolderGeneric : Send{
        let listener = TcpListener::bind(address).await;
        match listener {
            Ok(listener) => {
                while let Ok( (stream,_address)) = listener.accept().await {
                    tokio::spawn(async move{
                        _build_context_from_stream::<DataHolderGeneric>(stream,_address,
                         controllers
                        ).await;
                    });
                }
            }
            Err(e) => {
                panic!("");
            }
        }
    }
    async fn _build_context_from_stream<DataHolderGeneric:Send>
    (mut stream:TcpStream,_address:SocketAddr,
     controllers:&'static Vec<HttpContextRController<DataHolderGeneric>>
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
                    (stream,_address),
                    Some(vec![])).await;
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
    controllers:&'static Vec<HttpContextRController<DataHolderGeneric>>
    ){
        while let Ok(_) = context_framework_handler(&mut _context,controllers).await {
            _context.wait_for_another_request().await;
        }
        // clean_cached_connection(&ip_string);
    }

    async fn context_framework_handler<DataHolderGeneric:Send>(context: &mut HttpContext<DataHolderGeneric>,
    controllers:&'static Vec<HttpContextRController<DataHolderGeneric>>
    )->Result<(),String>{
        let _res = context_route_function_finder::find_function_from_controllers_and_execute(
            context,
            controllers
        ).await;
        match _res {
            Ok(_res) => {
                return Ok(());
            }
            Err(_err) => {
                context.send_string_data(&_err,true).await?;
            }
        }
        Err("".to_string())
    }


}














// use std::sync::Arc;
// use tokio::sync::Mutex;
// type ContextWrapper<DataHolderGeneric> = Arc<Mutex<HttpContext<DataHolderGeneric>>>;
// static  mut CONNECTIONS: Option<HashMap<String,(IpAddr,ContextWrapper)>> = None;


// fn _cache_connection(ip:IpAddr,context:HttpContext)->Arc<Mutex<HttpContext>>{
//     unsafe {
//         let x =  CONNECTIONS.as_mut().unwrap();
//         let data = Arc::new(Mutex::new(context));
//         let cloned = data.clone();
//         x.insert(ip.to_string(),(ip,data));
//         return cloned;
//     }
// }


// fn _cache_get(k:&str)->Option<&'static (IpAddr, ContextWrapper)>{
//     unsafe {
//         let res: Option<&(IpAddr, ContextWrapper)> =   CONNECTIONS.as_mut().unwrap().get(k);
//         res
//     }
// }
// fn clean_cached_connection(_ip:&str){
//     unsafe {
//         CONNECTIONS.as_mut().unwrap().remove(_ip);
//     }
// }