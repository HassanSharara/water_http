#[allow(unused,unused_assignments)]
 mod request;
mod server_runner;
pub use server_runner::start_server;
#[macro_use]
 mod response;
pub use response::*;
pub use request::*;

pub use crate::configurations::WaterServerConfigurations;
mod chose_encoding_algorithm;
mod tls;

/// for simple utils when you are working with
pub mod util;
use util::*;
use h2::{server::SendResponse,SendStream, RecvStream};
use serde::Serialize;
use std::{collections::HashMap, net::SocketAddr, str::FromStr, vec};
use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io::{Read, SeekFrom, Write};
use std::path::Path;
use std::string::ToString;
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::TcpStream};
use tokio::io::{AsyncSeekExt, AsyncWrite};
use nom::AsBytes;
use tokio_util::bytes::{Bytes, BytesMut};
use crate::framework_http::multipart_form::{HttpMultiPartFormDataField,
                                            parse_body_to_list_of_multipart_fields,
                                            parse_body_to_multipart_field_chunks
};
use crate::framework_http::x_www_form_urlencoded::XWWWFormUrlEncoded;
use crate::framework_http::chose_encoding_algorithm::HttpEncodingAlgorithms;





pub(crate) enum WaterTcpStream {
    Tls(tokio_rustls::server::TlsStream<TcpStream>),
    Stream(TcpStream),
}
type IncomeBody =Option<Option<HttpIncomeBody>>;
static mut ___SERVER_CONFIGURATIONS : Option<WaterServerConfigurations> = None;
const SIZE_OF_READ_WRITE_CHUNK:usize = 100000;
const SIZE_OF_FILES_WRITE_CHUNK:u64 = 100000;

/// struct for handling http requests
/// and responsible for caching connection and send response back to the client
/// with many easy and helpful functions to serve the client for  best performance
/// and provide stability and clear code for developer
///
pub struct HttpContext<DataHolderGeneric:Send> {
    pub(crate) protocol:Protocol,
    bytes_sent:usize,
    /// holding the data while passing through children capsules and middlewares
    pub data_holder:Option<DataHolderGeneric>,
    /// if the request was containing queries then it would be serialized to [HashMap]
    /// and saved inside path_params_map
    pub path_params_map:HashMap<String,String>,

    body:IncomeBody,
}

unsafe impl<T:Send>  Send for HttpContext<T> {
}
pub (crate) struct Http1 {
    _peer:(WaterTcpStream,SocketAddr),
    /// incoming http 1 request struct
    pub request:Request,
    _extra_bytes_from_headers:Vec<u8>,
    _header_sent:bool,
}

type  Http2Request = http::Request<RecvStream>;
type  Http2ResponseSender = SendResponse<Bytes>;
pub (crate) struct Http2 {
    pub cached_headers:HttpHeadersMap,
    pub request:Http2Request,
    pub send_response:Http2ResponseSender,
    pub body_sender:Option<SendStream<Bytes>>,
    pub path_query:HashMap<String,String>
}
pub(crate) enum Protocol {
    Http1(Http1),
    Http2(Http2)
}

/// # parsing the incoming body data to many objects
/// to fit all server requirements , need to provide all the data types
/// of incoming body so that why [HttpIncomeBody] is existed
pub enum HttpIncomeBody {
    /// to handle multipart-form data
    MultiPartFormat(Vec<(HttpMultiPartFormDataField,Vec<u8>)>),
    /// to handle x-www-form incoming data
    XWWWForm(XWWWFormUrlEncoded),
    /// to handle application/json data
    Json(Vec<u8>),
    /// to handle binary data
    Unit8Vec(Vec<u8>)
}



impl<DataHolderGeneric:Send> HttpContext<DataHolderGeneric> {


    fn from_http2_connection(
        request:Http2Request,
        send_response:Http2ResponseSender)->Result<Self,String>{
        let mut path_query = HashMap::new();
        let q = request.uri().query();
        if let Some(query) = q {
            path_query = Request::parse_to_query_map(query);
        }
        Ok(HttpContext{
            protocol:Protocol::Http2(Http2 {
                cached_headers:HashMap::new(),
                request,
                send_response,
                body_sender:None,
                path_query
            }),
            bytes_sent:0,
            data_holder:None,
            body:None,
            path_params_map:HashMap::new()
        })
    }

    async fn wait_for_another_request(&mut self){
        self.refresh_stream().await;
    }

    /// # for checking if we are serving this client by http1 protocol
    /// if you want to specify some features just for protocol h2 or h1 this is very useful
    pub fn is_http1(&self)->bool{
        if let Protocol::Http1(_) = self.protocol {
            return  true;
        }
        false
    }
    pub(crate) async fn refresh_stream(&mut self){
        self.bytes_sent = 0;
        if let Protocol::Http1(protocol) = &mut self.protocol {
            protocol._extra_bytes_from_headers.clear();
            protocol._header_sent = false;

            if let Ok((request,extra_body_bytes)) =
                build_headers(&mut protocol._peer.0).await {
                protocol.request = request;
                protocol._extra_bytes_from_headers = extra_body_bytes;
            }

        }
    }

    async  fn from_http1_connection(
        mut _peer:(WaterTcpStream,SocketAddr),
    )->Result<Self,String>{
        let (request,extra_body_bytes) =
            build_headers(&mut _peer.0).await?;
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
    fn from_h2_protocol_to_valid_headers_map<'a>(key:&str,_p2:&'a mut Http2)->Option<&'a Vec<Vec<String>>>{
        if let Some(_h) = _p2.request.headers().get(key) {
            let s = String::from_utf8_lossy(_h.as_bytes());
            let res = convert_string_value_to_vec_of_strings(&s);
            _p2.cached_headers.insert(key.to_owned(), res);
            return  _p2.cached_headers.get(key);
        }
        None
    }

    /// # for getting the current request path
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

    /// # for getting the current request method
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

    /// # getting all the headers requested by client
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

    /// # get single header value
    /// some headers has single value and some not
    /// that why it`s returning &['``Vec<Vec<String>>``']
    ///
    /// # return `Option<&Vec<Vec<String>>`
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

    ///  if the headers key was expected to return single value
    /// then you could use this fn
    /// # return [Option<&String>]
    pub fn get_from_headers_as_string(&mut self,key:&str)->Option<&String>{
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

    /// if you are expecting [u8] value or [usize] value from the header
    /// or any other data that could be parsed from string
    /// you can use this generic function
    pub fn get_from_header_as<T:FromStr>(&mut self,key:&str)->Option<T>{
        if let Some(value) = self.get_from_headers_as_string(key) {
            if let Ok ( v ) = value.parse::<T>() {
                return Some(v);
            }
        }

        None
    }



    /// # for saving memory from overload
    /// when you read the data from memory and then parse it to another type
    /// it`s will take multi steps to get the final results and all of these steps
    /// are exhausting the resources specially if the body data was large
    /// ,so you need to overhead these steps with single one and use these chunks
    /// to take advantage of using data buffer from the beginning
    /// ( notice ) that this function used by the same crate to save data synchronously
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
                            match &mut p._peer.0 {
                                WaterTcpStream::Tls(stream) => {
                                    'label: while let Ok(_c) = stream.read_buf(&mut buf).await {
                                        total_bytes_received += _c;
                                        bytes_chunk(&buf[.._c]);
                                        if _c == 0 || total_bytes_received >= length  {
                                            break 'label;
                                        }
                                        buf.clear();
                                    }
                                }
                                WaterTcpStream::Stream(stream) => {
                                    while let Ok(_c) = stream.read_buf(&mut buf).await {
                                        total_bytes_received += _c;
                                        bytes_chunk(&buf[.._c]);
                                        if _c == 0 || total_bytes_received >= length  {
                                            break;
                                        }
                                        buf.clear();
                                    }
                                }

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

    /// # converting all the incoming body to String
    /// some cases we need to consider using body as string data,
    /// so you could use this function for that purpose
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

    /// some cases we need to get all the incoming bytes to handle them later,
    /// so we could consider using whole_body_as_bytes function
    /// # return [`Result<Option<Vec<u8>>,String>`]
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

                match &mut p1._peer.0 {
                    WaterTcpStream::Tls(stream) => {
                        loop {
                            match stream.read_buf(&mut buf).await {
                                Ok(_s) => {
                                    if _s == 0 {
                                        return Ok(Some(bytes));
                                    }
                                    bytes.extend(&buf);

                                    if _s < buf.capacity() {
                                        return Ok(Some(bytes));
                                    }
                                    buf.clear();
                                }
                                Err(_) => {
                                    return Err("There An Error Happen While Reading Bytes From Tls\
                                     Stream".to_string());
                                }
                            }
                        }
                    }
                    WaterTcpStream::Stream(stream) => {
                        loop {
                            if let Ok(_s) = stream.read_buf(&mut buf).await {
                                if  _s <= 0 {
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

    /// # Recommended
    /// when we have GET request some time it come with query like
    /// /send?id=1&age=23
    /// so to get these values based on keys we could use this function
    /// # return [`Option<std::borrow::Cow<str>>`]
    pub async fn get_from_path_query_as_cow(&mut self,key:&str)->Option<std::borrow::Cow<str>>{
        let res = self.get_from_path_query(key);
        if let Some(res) = res {
            return Some(String::from_utf8_lossy(res));
        }
        None
    }

    /// when we have GET request some time it come with query like
    /// /send?id=1&age=23
    /// so to get these values based on keys we could use this function
    /// # return [`Option<String>`]
    pub async fn get_from_path_query_as_string(&mut self,key:&str)->Option<String>{
        let res = self.get_from_path_query_as_cow(key).await;
        if let Some(res) = res {
            return  Some(res.to_string());
        }
        None
    }
    /// when we have GET request some time it come with query like
    /// /send?id=1&age=23
    /// so to get these values bytes which it`s list of u8 based on keys we could use this function
    /// # return [Option<&[u8]>]
    pub fn      get_from_path_query(&self,key:&str)->Option<&[u8]> {
        return  match self.protocol {
            Protocol::Http1(ref h1) => {
                let res = h1.request.headers_query.get(key);
                if let Some(res) = res {
                    return Some(res.as_bytes());
                }
                None
            }
            Protocol::Http2(ref h2) => {
                if let Some(data) = h2.path_query.get(key){
                    return Some(data.as_bytes());
                }
                None
            }
        };
    }

    /// # Recommended
    /// when you want to get body value from request as [std::borrow::Cow]
    /// you could use this function
    pub async fn get_from_body_as_cow(&mut self,key:&str)->Option<std::borrow::Cow<str>>{
        if let Some(res) = self.get_from_body(key).await {
            return Some(String::from_utf8_lossy(res));
        }
        None
    }

    /// when you want to get body value from request as [String]
    /// you could use this function
    pub async fn get_from_body_as_string(&mut self,key:&str)->Option<String>{
        if let Some(res) = self.get_from_body_as_cow(key).await {
            return Some(res.to_string());
        }
        None
    }

    /// you could use this method to retrieve data from GET OR POST method the data has been passed to the
    /// server
    pub async fn get_from_all_params<'a>(&'a mut self,key:&str)->Option<&'a [u8]>{
        if &self.get_method().to_lowercase() == "get" {
            return self.get_from_path_query(key);
        }
        self.get_from_body(key).await
    }

    /// # when you want to get specific bytes from body by using key
    /// example when provide some encoded or special bytes inside incoming request
    /// you could  retrieve these bytes using this function
    pub async fn get_from_body(&mut self,key:&str)->Option<&[u8]>{
        let body = self.get_body().await;
        return  match body {
            None => {
                None
            }
            Some(body) => {
                match body {
                    HttpIncomeBody::MultiPartFormat(body) => {
                        for (body,bytes) in body {
                            if body.get_name_key() == key {
                                return Some(bytes);
                            }
                        }
                        None
                    }
                    HttpIncomeBody::XWWWForm(body) => {
                        let body =  body.data.get(key);
                        if let Some(body) = body {
                            return  Some(body.as_bytes());
                        }
                        None
                    }

                    _ => {
                        None
                    }
                }
            }
        }
    }
    ///
    /// # return Option;
    pub async fn get_body<'a>(&'a mut self )-> &'a Option<HttpIncomeBody>{
        match self.body {
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
        if let Some(mut content_type) = content_type {
            content_type = content_type.to_lowercase();
            if content_type == "multipart/form-data" {
                let fields = parse_body_to_list_of_multipart_fields(self).await;
                return  HttpIncomeBody::MultiPartFormat(fields).into();
            }
            else  if content_type == "application/json" {
                let body = self.whole_body_as_bytes().await;
                if let Ok(Some(body)) = body {
                    return  Some(HttpIncomeBody::Json(body));
                }
                return  None;
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

    /// for redirecting request to another path by using
    /// routers names
    ///
    /// if you need to know how to set name for each router you could use this
    /// GET_make_order => / => ....
    ///
    /// in this request route name is ["make_order"] which came after GET_ or POST_
    ///
    /// and also you can use this function [get_route_by_name] to get your route
    ///
    pub async fn redirect_by_route_name(&mut self,key:&str,options:Option<&[(&str,&str)]>)->Result<(),String>{
        let path = get_route_by_name(key,options);
        if let Some(_path) = path {
            self.redirect_to_by_url(&_path).await?;
        }
        Err(format!("could not found this route name: {key}"))
    }

    /// for redirecting request to another path
    pub async fn redirect_to_by_url(&mut self,url:&str)->Result<(),String>{
        let mut headers = ResponseHeadersBuilder::found_redirect_header(url);
        let data = b" ";
        headers.set_header_key_value("Content-Length",data.len());
        self.send_headers(headers).await?;
        self.send_data(data,true).await?;
        Ok(())
    }

    pub async fn send_string_data(&mut self,slice:String,end_of_stream:bool)->Result<(),String>{
        self.send_data(slice.as_bytes(), end_of_stream).await?;
        return Ok(());
    }



    /// Sends a string slice [&str] as a response.
    /// Note that the second parameter, `end_of_stream`, indicates whether this
    /// is the last response packet for the request.
    /// If it is, the system will flush all the data, clear any necessary
    /// data from RAM, and close unnecessary connections if needed.
    pub async fn send_str_data(&mut self,slice:&str,end_of_stream:bool)->Result<(),String>{
        self.send_data(slice.as_bytes(), end_of_stream).await?;
        return Ok(());
    }


    pub (crate)async fn shutdown_connection(&mut self){
        match &mut self.protocol {
            Protocol::Http1(p1) => {
                match &mut p1._peer.0 {
                    WaterTcpStream::Tls(stream) => {
                        let _ = stream.shutdown().await;
                    }
                    WaterTcpStream::Stream(stream) => {
                        let _ = stream.shutdown().await;
                    }
                }
            }
            _ => {

            }
        }
    }

    /// for sending json response that depends on [serde]
    /// for example each struct that derive [#[derive(Serialize,Deserialize)]]
    pub async fn send_json_data<T>(&mut self,value:&T,end_of_stream:bool)->Result<(),String> where T: ?Sized + Serialize {
        let v = serde_json::to_string(value);
        if let Ok(_v) = v {
            return self.send_data(_v.as_bytes(), end_of_stream).await;
        }
        Err("Can not convert json data to string".to_owned())
    }
    /// let`s say that you want to return file as response and this file stored
    /// in public path,
    /// so you do not need to provide the path of the whole file
    /// you just need to provide the path since of public directory path
    /// for example
    ///
    /// if your file have this path ["public/images/customer1_profile.jpg"]
    /// then you could use this function [send_file_from_public_resources("images/customer1_profile.jpg")]
    pub async fn send_file_from_public_resources(&mut self,path:&str)->Result<(),String>{
        let public_path = unsafe {&___SERVER_CONFIGURATIONS.as_ref().unwrap().public_files_path};
        let path = format!("{public_path}/{path}").replace("//","/");
        self.send_file_as_response(&path).await
    }


    #[allow(unused_assignments)]
    /// for sending any file in any directory of the system as response
    ///
    pub async fn send_file_as_response(&mut self,path:&str)
                                       ->Result<(),String>{
        let file_path = Path::new(path);
        if !file_path .exists() {
            let mut headers = ResponseHeadersBuilder::not_found_headers();
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
        let mut h: Option<ResponseHeadersBuilder> = None;
        match content_type {
            None => {
                h = Some(ResponseHeadersBuilder::bad_request_headers());
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
                h = Some(ResponseHeadersBuilder::success());
                let  headers = h.as_mut().unwrap();
                headers.set_header_key_value("Content-Type",content_type);
            }
        }

        let  file = tokio::fs::File::open(path).await;
        if let Ok(mut file) = file {
            let metadata = file.metadata().await;
            let  file_size = if let Ok(metadata) = metadata {
                metadata.len()
            }
            else { 0 };

            if file_size == 0 {
               return Err("could not read total file size from file metadata".to_string());
            }





            if let Some(headers) = h.as_mut() {
                let income_range = self.get_from_header_as::<String>("Range");
                return match income_range {
                    None => {
                        headers.set_header_key_value("Content-Length", file_size);
                        self.send_headers(h.unwrap()).await?;
                        let mut buffer = [0; 4000];
                        while let Ok(size) = file.read(&mut buffer).await {
                            if size == 0 {
                                return  Ok(());
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
    /// in order to render html your request should have Content-Length and Content-Type
    /// with specific data so that the browsers would understand what type of response
    /// they are receiving
    pub async fn render_html(&mut self,data:&str,with_headers:bool)->Result<(),String>{
        let data = data.as_bytes();
        if with_headers {
            let mut headers = ResponseHeadersBuilder::success();
            headers.set_header_key_value("Content-Type","text/html; charset=UTF-8");
            headers.set_header_key_value("Content-Length",data.len());
            self.send_headers(headers).await?;
        }
        self.send_data(data,true).await?;
        Ok(())
    }

    async fn write_bytes(&mut self,bytes:&[u8],end_of_stream:bool)->Result<(),String>{
        match &mut self.protocol {
            Protocol::Http1(p) => {
                match &mut p._peer.0 {
                    WaterTcpStream::Tls(  stream) => {
                        if let Ok(_) = stream.write_all(&bytes).await {
                            if  end_of_stream {
                                if let Err(_) = stream.flush().await {
                                    return Err("can not Flushing All The Data in The Stream".to_string());
                                }

                            }
                            return Ok(());
                        } else {

                        }
                    }
                    WaterTcpStream::Stream(  _stream) => {
                        if let Ok(_) = _stream.write_all(&bytes).await {
                            if  end_of_stream {
                                if let Err(_) = _stream.flush().await {
                                    return Err("can not Flushing All The Data in The Stream".to_string());
                                }
                            }
                            return Ok(());
                        }
                    }
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


    /// for sending a most likely headers and custom headers
    /// you could check this headers by using [ResponseHeadersBuilder] struct
    /// if you are returning success content with 200 status code you could use
    /// this factory [ResponseHeadersBuilder::success()]
    pub async fn send_headers(
        &mut self,
        headers:ResponseHeadersBuilder,
    )->Result<(),String>{
        self._send_headers(headers,false).await
    }

    async fn _send_headers(
        &mut self,
         headers:ResponseHeadersBuilder,
        end_of_stream:bool
    )->Result<(),String>{

        match  &mut self.protocol {
            Protocol::Http1(h1) => {
                if h1._header_sent {
                    return Ok(());
                }
                h1._header_sent = true;

                let bytes =  headers.to_bytes();
                self.write_bytes(&bytes,end_of_stream).await?;
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
                        let key =http::HeaderName::from_str(k);
                        let value = http::HeaderValue::from_str(v);
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

    /// for sending bytes data [[u8]]
    /// if you need to use lower level and use custom data to send back
    /// you could use this function and also notice that end_of_stream
    /// indicating if this the last response for your current request or not
    #[async_recursion::async_recursion]
    pub async fn send_data(&mut self,bytes:&[u8],
                           end_of_stream:bool)->Result<(),String>{
        let bytes_length = bytes.len();
        let mut encoded_data : Option<Vec<u8>> = None;

        match &mut self.protocol {
            Protocol::Http1(h1) => {

                let mut  headers = ResponseHeadersBuilder::success();
                if !h1._header_sent {
                    let threshold = unsafe {___SERVER_CONFIGURATIONS.as_ref().unwrap().threshold_for_encoding_response};
                    if bytes_length >= threshold as usize {
                        if let Some(encoded_message_from_headers) = self.get_from_headers("Accept-Encoding") {
                            let encoding_algorithm =
                                chose_encoding_algorithm::detect_encoding_algorithm(encoded_message_from_headers);
                            if let Some(encoding_algorithm)  = encoding_algorithm {
                                match encoding_algorithm {
                                    HttpEncodingAlgorithms::ZStd => {
                                        let mut data = Vec::new();
                                        if let Ok(_) = chose_encoding_algorithm::encode_data_with_z_std(
                                            bytes,&mut data
                                        ) {
                                            headers.set_header_key_value("Content-Encoding","zstd");
                                            encoded_data = Some(data);
                                        }

                                    }
                                    HttpEncodingAlgorithms::Brotli => {
                                        let mut data = Vec::new();
                                        if let Ok(_) = chose_encoding_algorithm::encode_data_with_brotli(
                                            bytes,&mut data
                                        ) {
                                            headers.set_header_key_value("Content-Encoding","br");
                                            encoded_data = Some(data);
                                        }
                                    }
                                    HttpEncodingAlgorithms::Gzip => {
                                        let mut data = Vec::new();
                                        if let Ok(_) = chose_encoding_algorithm::encode_data_with_gzip(
                                            bytes,&mut data
                                        ) {
                                            headers.set_header_key_value("Content-Encoding","gzip");
                                            encoded_data = Some(data);
                                        }
                                    }
                                    HttpEncodingAlgorithms::Deflate => {
                                        let mut data = Vec::new();
                                        if let Ok(_) = chose_encoding_algorithm::encode_data_with_deflate(
                                            bytes,&mut data
                                        ) {
                                            headers.set_header_key_value("Content-Encoding","deflate");
                                            encoded_data = Some(data);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    match encoded_data {
                        None => {
                            if end_of_stream {
                                headers.set_header_key_value("Content-Length",bytes.len());
                            }
                            headers.set_header_key_value("Date",__current_date_rfc2822());

                            self.send_headers(headers).await?;
                            if let Ok(_) = self.write_bytes(bytes, end_of_stream).await {

                                self.bytes_sent += bytes_length;
                                return  Ok(());
                            }
                        }
                        Some(ref bytes) => {
                            if end_of_stream {
                                headers.set_header_key_value("Content-Length",bytes.len());
                            }
                            headers.set_header_key_value("Date",__current_date_rfc2822());
                            let _ = self.send_headers(headers).await?;
                            if let Ok(_) = self.write_bytes(bytes, end_of_stream).await {
                                self.bytes_sent += bytes_length;
                                return  Ok(());
                            }
                        }
                    }
                } else {
                    if let Ok(_) = self.write_bytes(bytes, end_of_stream).await {
                        println!("we are already sent {}",String::from_utf8_lossy(bytes));
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
                        let mut headers = ResponseHeadersBuilder::success();
                        let threshold = unsafe {___SERVER_CONFIGURATIONS.as_ref().unwrap().threshold_for_encoding_response};
                        if bytes_length >= threshold as usize {
                            if let Some(encoded_message_from_headers) = self.get_from_headers("Accept-Encoding") {
                                let encoding_algorithm =
                                    chose_encoding_algorithm::detect_encoding_algorithm(encoded_message_from_headers);
                                if let Some(encoding_algorithm)  = encoding_algorithm {
                                    match encoding_algorithm {
                                        HttpEncodingAlgorithms::ZStd => {
                                            let mut data = Vec::new();
                                            if let Ok(_) = chose_encoding_algorithm::encode_data_with_z_std(
                                                bytes,&mut data
                                            ) {
                                                headers.set_header_key_value("Content-Encoding","zstd");
                                                encoded_data = Some(data);
                                            }

                                        }
                                        HttpEncodingAlgorithms::Brotli => {
                                            let mut data = Vec::new();
                                            if let Ok(_) = chose_encoding_algorithm::encode_data_with_brotli(
                                                bytes,&mut data
                                            ) {
                                                headers.set_header_key_value("Content-Encoding","br");
                                                encoded_data = Some(data);
                                            }
                                        }
                                        HttpEncodingAlgorithms::Gzip => {
                                            let mut data = Vec::new();
                                            if let Ok(_) = chose_encoding_algorithm::encode_data_with_gzip(
                                                bytes,&mut data
                                            ) {
                                                headers.set_header_key_value("Content-Encoding","gzip");
                                                encoded_data = Some(data);
                                            }
                                        }
                                        HttpEncodingAlgorithms::Deflate => {
                                            let mut data = Vec::new();
                                            if let Ok(_) = chose_encoding_algorithm::encode_data_with_deflate(
                                                bytes,&mut data
                                            ) {
                                                headers.set_header_key_value("Content-Encoding","deflate");
                                                encoded_data = Some(data);
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        match encoded_data {
                            Some(data) => {
                                if end_of_stream {
                                    headers.set_header_key_value(
                                        "Content-Length",
                                        data.len(),
                                    );
                                }
                            }
                            None => {
                                if end_of_stream {
                                    headers.set_header_key_value(
                                        "Content-Length",
                                        bytes_length,
                                    );
                                }
                            }
                        }
                        headers.set_header_key_value(
                            "Date",__current_date_rfc2822()
                        );
                        self.send_headers(headers).await?;
                        self.send_data(bytes,end_of_stream).await?;
                        return  Ok(());
                    },
                }

            },
        }
        Err("Cant Write Data to Stream".to_owned())
    }


    /// notice that this function work with single read of tcp stream
    /// its mean if had already got the body of request
    /// ,or you had already used this function in scope before its will not work
    /// cause it`s designed for high performance
    /// so make sure to call this function one time at scope per context
    /// if you expect for users to upload multi files or single file
    /// its will work successfully with efficient speed
    pub async fn save_files_from_request_and_get_the_rest_of_fields<'a>(
        &mut self,
        rules_for_saving:&[SaveMetadataForMultipart<'a>],
    )
    ->Result<SaveForMultipartResults,String>{

        let mut rest_fields :Vec<(HttpMultiPartFormDataField,Vec<u8>)> = vec![];
        let mut saved_files :Vec<(HttpMultiPartFormDataField,Result<File,String>)> = vec![];
        let mut working_on_field:Option<HttpMultiPartFormDataField> = None;
        let mut  for_saving:Option<(&SaveMetadataForMultipart,Result<File,String>)> = None;
        let mut single_field_data = Vec::<u8>::new();
        let _ = parse_body_to_multipart_field_chunks(
            self,
         |field,data|   {
             let _  =  _save_from_request_multipart_sync_and_get_the_rest_of_fields_function_helper(
                 &mut single_field_data,
                 &mut rest_fields,
                 &mut saved_files,
                 &mut working_on_field,
                 &mut for_saving,
                 field,
                 data,
                 rules_for_saving);
             ()
         }
        ).await;
        if let Some(for_saving) = for_saving {
            if let Some(field) = working_on_field {
                saved_files.push(
                   (
                       field,
                       for_saving.1
                   )
                );
            }
        }else {
            if  !single_field_data.is_empty() {
                rest_fields.push(
                    (
                        working_on_field.unwrap(),
                        single_field_data
                    )
                );
            }
        }


       Ok(
           SaveForMultipartResults {
               saved_as_files_fields:saved_files,
               rest_fields
           }
       )
    }
}
  fn _save_from_request_multipart_sync_and_get_the_rest_of_fields_function_helper<'a>(
                single_field_data:&mut Vec<u8>,
                rest_fields:&mut Vec<(HttpMultiPartFormDataField,Vec<u8>)>,
                saved_files:&mut Vec<(HttpMultiPartFormDataField,Result<File,String>)>,
                working_on_field:&mut Option<HttpMultiPartFormDataField>,
                for_saving:&mut Option<(&'a SaveMetadataForMultipart<'a>,Result<File,String>)>,
                field:&HttpMultiPartFormDataField,
                data:&[u8],
                rules_for_saving:&'a [SaveMetadataForMultipart<'a>]
) ->Result<(),String>{
    // now we had a field
    if let Some(wf) = working_on_field {
        if wf.get_file_name() != field.get_file_name() {
            if let Some((_,file)) = for_saving {
                match file {
                    Ok(file) => {
                        if let Ok(file) = file.try_clone() {
                            saved_files.push(
                                ( wf.clone(),
                                  Ok(file)
                                )
                            );
                        }
                    }
                    Err(err) => {
                        saved_files.push(
                            ( wf.clone(),
                              Err(err.to_string())
                            )
                        );
                    }
                }
            } else{
                rest_fields.push(
                    (
                        wf.clone(),
                        single_field_data.clone()
                    )
                );
            }
            single_field_data.clear();
            *working_on_field = None;
           let _ =  _save_from_request_multipart_sync_and_get_the_rest_of_fields_function_helper(single_field_data,
                                                                                         rest_fields,
                                                                                         saved_files,
                                                                                         working_on_field,for_saving,field,data,rules_for_saving);
        }
        if let Some((_save_rule,file)) = for_saving {
            if let Ok(file) = file {
                let _ = file.write(data);
            } else {
                return Ok(());
            }
        } else{
            single_field_data.extend(data);
        }
        Ok(())

    }
    // if we are signing new field
    else {
        *working_on_field = Some(field.clone());
        *for_saving = None;
        for save_rule in rules_for_saving {
            if save_rule.field_name != field.get_name_key() { continue; }
            let mut file_path = save_rule.saving_path.to_string();
            let last = file_path.split("/").last();
            if let Some(last) = last{
                if !last.contains(".") {
                    if !last.ends_with("/") {file_path.push_str("/");}
                    if let Some(name) = field.get_file_name() {
                        file_path.push_str(name);
                    }
                }
            }
            let file  = File::create(&file_path);
            if let Ok(file) = file {
             *for_saving = Some((save_rule,Ok(file)));
            }
            else{
             *for_saving = Some((save_rule,Err("could not initiate file with this path".to_string())));
            }
            break;
        }
        _save_from_request_multipart_sync_and_get_the_rest_of_fields_function_helper
            (single_field_data,rest_fields,
             saved_files,
             working_on_field,for_saving,field,data,rules_for_saving)
    }
}



/// # for getting route by using his name
/// also you need to provide an options of parameters if there were
/// else you could parse None if there is not
pub fn get_route_by_name<'a>(
    name:&str,
    options:Option<&[(&str,&str)]>)->Option<String>{
    unsafe {
        let results = crate::___ROUTERS.as_ref();
        if let Some(map) = results {
            let  path = map.get(name);
            if let Some(_path) = path {
                let mut path = _path.to_string();
                if let Some(options) = options {
                    if !options.is_empty() {
                        for (k,v) in options.iter() {
                            let replace_pattern = format!("{{{}}}",k);
                            if !_path.contains(&replace_pattern){
                                return None;
                            }
                            path = path.replace(&replace_pattern,v);
                        }
                    }
                }
                if !path.is_empty() {
                    return Some(path);
                }
            }
        }
    }
    None
}


/// - when you are doing post request to upload file,and you want to use save_files_from_request_and_get_the_rest_of_fields function
/// then you probably need to provide these expected files as a parameter so
/// [SaveMetadataForMultipart] struct is for that purpose
#[derive(Debug)]
pub struct SaveMetadataForMultipart <'a>{
    /// # providing the incoming key or field name that holds a file
    pub field_name:&'a str,
    /// # providing the path for saving this file in
    /// you could provide this path in two ways
    /// - providing the path without file name ex:"./public/uploads/" or "./public/uploads"
    /// then the file would be saving by his default name
    /// - providing the path with a file name ex:"./public/uploads/image1.jpg"
    /// then the file would be saving as image1.jpg
    pub saving_path:&'a str,
}

impl<'a> SaveMetadataForMultipart<'a> {
    pub fn new(name:&'a str,saving_path:&'a str)->Self {
        Self {
            field_name:name,
            saving_path
        }
    }
}

/// when you call fn save_files_from_request_and_get_the_rest_of_fields
/// the results would be [SaveForMultipartResults]
/// which a struct holding the body fields of the request and if these fields are
/// having files field that you provide before then it would be saved inside
/// ['saved_as_files_fields'] and rest of fields would be saved at ['rest_fields']
#[derive(Debug)]
pub struct SaveForMultipartResults {
    /// # if request was containing files data like in multipart form is
    /// and these files are saved successfully then it would be used at this variable
    pub saved_as_files_fields:Vec<(HttpMultiPartFormDataField,Result<File,String>)>,


    /// # if the requested field was not a file or it`s not be provided by you as file expected
    /// then it would be saved at reset_fields variable
    pub rest_fields:Vec<(HttpMultiPartFormDataField,Vec<u8>)>,
}

pub (crate) fn __current_date_rfc2822()->String{
    let datetime = chrono::Utc::now().format("%a, %d %b %Y %H:%M:%S GMT").to_string();
    datetime
}












