/// if the request body was multipart-form-data
/// then it would handle as [HttpMultiPartFormDataField]
pub mod multipart_form;

/// a struct for handling Content or the body of request
/// when the requested body was a type of x-www-form data
/// then it would be handled using [XWWWFormUrlEncoded] struct
/// which hold data
pub mod x_www_form_urlencoded;
use std::collections::HashMap;
use bytes::BytesMut;
use tokio::io::AsyncReadExt;
use crate::framework_http::{___SERVER_CONFIGURATIONS, SIZE_OF_READ_WRITE_CHUNK, WaterTcpStream};
use crate::multipart_form::HttpMultiPartFormDataField;
use crate::x_www_form_urlencoded::XWWWFormUrlEncoded;


/// just a type of [`HashMap<String,Vec<Vec<String>>>`]
pub type HttpHeadersMap = HashMap<String,Vec<Vec<String>>>;


/// # used for very light api
const IMPORTANT_HEADERS :[&str;9] = [
    "Cookie",
    "Accept-Encoding",
    "Accept",
    "Connection",
    "Content-Length",
    "Content-Type",
    "Range",
    "Host",
    "Date",
];

/// # for building income request struct and holding the important data
#[derive(Debug)]
pub struct Request {
    /// # hold request type like (GET,POST) Requests
    pub method:String,
    /// # hold requested path
    pub path:String,
    /// # provide http version requested by clients
    pub http_version:String,
    /// # hold headers keys and values
    pub headers_map:HttpHeadersMap,
    /// # if request headers has a query then it would be serialized to headers_query
    pub headers_query:HashMap<String,String>,
}


impl Request {


    pub (crate) fn parse_to_query_map(i:&str)->HashMap<String,String>{
        let mut result = HashMap::new();
        let values = i.split("&");
        for query in values {
            if let Some(index) = query.find("=") {
                result.insert(query[..index].to_string(),query[index+1..].to_string());
            }
        }
        result
    }

    /// # for building [Request]
    /// for parsing incoming request string in http1.1 to Request struct
    /// also you have important headers
    /// - if important headers is None then all the headers will be allocated at the whole request life
    /// - if it`s  Some of vec but it`s empty then just the most important headers would be created and allocated
    /// - if it`s has one or more values then just these values would be created and allocated in memory
    pub fn build_request(
        string_header:String,
        important_headers:Option<&Vec<String>>
         )->Result<Request,String>{
        let mut lines = string_header.lines();
        let mut request = Request {
            method:"".to_string(),
            path:"".to_string(),
            http_version:"".to_string(),
            headers_map:HashMap::new(),
            headers_query:HashMap::new(),
        };
        if let Some(first_line) = lines.next(){
            let _e = Err("Request Error".to_string());
            if let Some(method) = first_line.find(" ") {
                request.method = first_line[..method].to_string();
                let rest = &first_line[method..].trim();
                if let Some(_path) = rest.find(" ") {
                    if let Some(_and_splitter) = rest.find("?") {
                        let path =  &rest[.._and_splitter];
                        request.path = path.to_string();
                        request.headers_query = Self::parse_to_query_map(&rest[_and_splitter+1.._path]);
                    } else {
                        let path =  &rest[.._path];
                        request.path = path.to_string();
                    }



                    request.http_version = rest[_path+1..rest.len()].to_string();
                } else { return _e;}
            } else { return _e;}
        }
         let never_filter_headers = important_headers.is_none();
         while let Some(_line) = lines.next() {
            let mut splitter : Vec<&str> = _line.split(": ").collect();
            if splitter.len() < 2 {
                continue;
            }
            let key = splitter.remove(0);
             if !never_filter_headers  {
                 if let Some(important_headers) = &important_headers {
                     if important_headers.is_empty() {
                         if !IMPORTANT_HEADERS.contains(&key){
                             continue;
                         }
                     }
                     if let None = important_headers.iter().find(
                     |_v| &key == _v
                     ) {
                         continue;
                     }
                 }
             }

            let  values:Vec<Vec<String>> = 
            convert_string_value_to_vec_of_strings(splitter.first().unwrap());
            let _ = &request.headers_map.insert(key.to_string(),values);    
         }
        Ok(request)
    }
}

pub (crate) fn convert_string_value_to_vec_of_strings(slice:&str)->Vec<Vec<String>> {
    let mut values:Vec<Vec<String>> = vec![];
                let internal_splitter = slice.split(";");
                for lvi in internal_splitter {
                    let mut internal_values :Vec<String>= vec![];
                    let last_interval =  lvi.split(",");
                    for v in last_interval {
                        internal_values.push(v.to_string());
                    }
                    values.push(internal_values);
                }
                values
}
pub (crate) async fn build_headers(_peer:&mut WaterTcpStream)->Result<(Request,Vec<u8>), String> {
    let  req_headers : Option<&Vec<String>> =unsafe {___SERVER_CONFIGURATIONS.as_ref().unwrap()
        .headers_for_reading.as_ref()
    };
    let mut header_string = String::new();
    let mut buf = BytesMut::with_capacity(SIZE_OF_READ_WRITE_CHUNK);
    let mut left_bytes:Vec<u8> = vec![];

    match _peer {
        WaterTcpStream::Tls(stream) => {
            'label: loop {
                if let Ok(_s) = &stream.read_buf(&mut buf).await {
                    if _s == &0 {
                        break 'label;
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
                        left_bytes.extend(&buf[index..]);
                        break 'label;
                    } else {
                        header_string.push_str(&String::from_utf8_lossy(&buf[..*_s]));
                    }
                    if _s < &buf.capacity()  {
                        break 'label;
                    }

                }else {
                    break 'label;
                }
            }
        }
        WaterTcpStream::Stream(_stream) => {
            loop {
                if let Ok(_s) = &_stream.read_buf(&mut buf).await {
                    if _s == &0 {
                        break;
                    }
                    let mut counter:u8 = 0 ;
                    let mut _i:Option<usize> = None;
                    for (index,byte) in buf.iter().enumerate() {
                        if let 0 = counter {
                            if *byte == 13 {
                                counter += 1;
                            } else {
                                counter = 0;
                            }
                        } else if let 1 = counter {
                            if *byte == 10 {
                                counter += 1;
                            } else {
                                counter = 0;
                            }
                        } else if let 2 = counter {
                            if *byte == 13 {
                                counter += 1;
                            } else {
                                counter = 0;
                            }
                        } else if let 3 = counter {
                            if *byte == 10 {
                                counter += 1;
                                _i = Some(index + 1);
                                break;
                            } else {
                                counter = 0;
                            }
                        }
                    }


                    if let Some( index ) = _i {
                        let end_string = String::from_utf8_lossy(&buf[..index]);
                        header_string.push_str(&end_string);
                        left_bytes.extend(&buf[index..]);
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
        }
    }

    let request = Request::build_request(
        header_string,
        req_headers);
    if let Err(_s) = request  {
        return Err(_s);
    }
    return Ok((request.unwrap(),left_bytes));
}
