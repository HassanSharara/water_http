
mod body;
mod header;
mod getter;
pub use getter::*;

use std::borrow::Cow;
use std::fmt::{Display,  Formatter};
pub use body::*;
use crate::http::request::header::{KeyValueList};



 const CONTENT_LENGTH_PATTERNS:[&[u8];4] = [b"Content-Length",
     b"Content-length",
     b"content-length",
     b"content-length"];



/// a structr for holding references to
/// buffer bytes with zero copy cost
pub  struct IncomingRequest<'a,const HEADERS_COUNT:usize,const PATH_QUERY_COUNT:usize> {
    http_version:&'a [u8],
    method:&'a [u8],
    path:&'a [u8],
    path_query:KeyValueList<'a,PATH_QUERY_COUNT>,
    content_length:Option<usize>,
    pub (crate) headers:KeyValueList<'a,HEADERS_COUNT>,
    pub(crate) total_headers_bytes:usize,
}

impl<'a,const HEADERS_COUNT:usize,const PATH_QUERY_COUNT:usize> Display for  IncomingRequest<'a,HEADERS_COUNT,PATH_QUERY_COUNT> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut output = String::new();
        for i in self.headers.all_pairs() {
            output.push_str(&format!("\r\n{i}"));
        }
        std::fmt::Display::fmt(
            &format!(
                "http_version : {} \r\n method {} \r\n path {} {output}"
                ,self.http_version(),
                self.method(),
                self.path()
            ),
            f
        )
    }
}

/// used for infra structure macros
 #[macro_export]
#[doc(hidden)]
 macro_rules! inc_start_pointer {
    ($start:ident,$index:ident,$payload_length:ident) => {
         $start = $index+1;
         if $start >= $payload_length {
             $start-=1;
         }
    };
}
pub (crate) use inc_start_pointer;
impl <'a,const HEADERS_COUNT:usize,const PATH_QUERY_COUNT:usize>
IncomingRequest<'a, HEADERS_COUNT,PATH_QUERY_COUNT>

{


    ///
    /// for creating new request from incoming bytes
    /// and returning back where the headers of this request
    /// is end or at what position
    /// # return [FormingRequestResult]
    pub  fn new(payload:&'a[u8])->FormingRequestResult<'a,HEADERS_COUNT,PATH_QUERY_COUNT>{
        let pk = payload.iter().enumerate();
        // for determining the end of each line and the end of the incoming request headers
        let mut end_founder_counter:u8 = 0;

        // for indicating start bytes of custom data and the end of it
        let mut start:usize = 0;

        let mut method = None;
        let mut path = None;
        let mut version = None;
        let payload_length = payload.len();
        let mut first_line_read = false;
        let mut queries = KeyValueList::empty();
        let mut queries_pop_start = false;
        let mut query_key = None;
        let mut header_key:Option<&'a [u8]> = None;
        let mut headers  = KeyValueList::empty();
        let mut headers_fully_filled = false;
        let mut success_end_of_request = None;
        let mut content_length :Option<usize> = None;
        for (index ,byte) in pk {
            match *byte {
                b'\r'  => {
                    end_founder_counter+=1;
                    if !first_line_read &&  version.is_none() {
                        version = Some(&payload[start..index]);
                        first_line_read = true;
                    }
                    if !headers_fully_filled {
                        if let Some(key) = header_key {
                            let data = &payload[start..index];

                            let this_is_content_length_header =
                                CONTENT_LENGTH_PATTERNS.contains(&key);
                            if this_is_content_length_header {
                                let cl = String::from_utf8_lossy(data).parse::<usize>();
                                match cl {
                                    Ok(cl) => { content_length = Some(cl);}
                                    Err(_) => {
                                        return FormingRequestResult::Err;
                                    }
                                }
                            } else
                            {
                               if let Err(_) =  headers.push(key,data) {
                                   headers_fully_filled = true;
                               }
                            }
                            header_key = None;
                            inc_start_pointer!(start,index,payload_length);
                        }
                    }
                    continue;}
                b'\n'=> {
                    end_founder_counter+=1;
                    inc_start_pointer!(start,index,payload_length);
                    if end_founder_counter == 4 {
                        success_end_of_request = Some(index);
                        break;
                    }
                    continue;

                }
                b' '=> {
                    if !first_line_read {
                        // checking if method is being read
                        match method {
                            Some(_)=>{
                                // checking of path is being read
                                match path {
                                    Some(_)=>{
                                        if queries_pop_start && query_key.is_some() {
                                            _=queries.push(query_key.unwrap(),&payload[start..index]);
                                            queries_pop_start = false;
                                            query_key = None;
                                            inc_start_pointer!(start,index,payload_length);
                                        }
                                    }
                                    _ => {
                                        path = Some(&payload[start..index]);
                                        inc_start_pointer!(start,index,payload_length);
                                    } }
                            }
                            _ => {
                                method = Some(&payload[start..index]);
                                start =  index +1 ;
                                if start >= payload_length {
                                    start -=1;
                                }
                            }
                        }
                    }
                    if header_key.is_some() && start == index {
                        inc_start_pointer!(start,index,payload_length);
                    }
                    continue
                }

                // formating incoming request
                _ =>{

                    end_founder_counter = 0;
                    if !first_line_read {
                        if index > 100000 {
                            return FormingRequestResult::Err
                        }
                        match *byte {
                            b'?' =>{
                                path = Some(&payload[start..index]);
                                queries_pop_start = true;
                                inc_start_pointer!(start,index,payload_length);
                            }
                            b'=' => {
                                query_key = Some(&payload[start..index]);
                                inc_start_pointer!(start,index,payload_length);
                            }
                            b'&' => {
                                if let Some(key) = query_key {
                                    _=queries.push(key,&payload[start..index]);
                                    query_key = None;
                                }
                            }
                            _ => {continue}
                        }
                        continue;
                    }
                    match *byte {
                        b':'=>{
                            header_key = Some(&payload[start..index]);
                            inc_start_pointer!(start,index,payload_length);
                        }
                        _ => {}
                    }
                }

            }


        }

        if let Some(success_end_of_request) = success_end_of_request {
            if version.is_none() || method.is_none() || path.is_none() {
                return FormingRequestResult::Err
            }
            return FormingRequestResult::Success(
                    IncomingRequest {
                        http_version:version.unwrap(),
                        headers,
                        method:method.unwrap(),
                        path_query:queries,
                        path:path.unwrap(),
                        content_length,
                        total_headers_bytes:success_end_of_request+1,
                    }
            )
        }
        FormingRequestResult::ReadMore
    }

    /// for getting the path of the incoming request
    ///
    /// # return [`Cow<str>`] which 'a stands for the life of incoming request
    pub fn path(&self)->Cow<str> {
        String::from_utf8_lossy(self.path)
    }

    /// parsing incoming http method
    pub fn method(&self)->Cow<str> {
        String::from_utf8_lossy(self.method)
    }

    /// parsing incoming http version
    pub fn http_version(&self)->Cow<str> {
        String::from_utf8_lossy(self.http_version)
    }

    /// getting content length in headers
    pub fn content_length(&self)->Option<&usize> {
        self.content_length.as_ref()
    }


    /// getting data from path query
    /// for examples
    /// - http:://examples.com/posts?id=1 the key is `id` and the value is `1`
    /// - http:://examples.com/posts?year=2024 the key is `year` and the value is `2024`
    pub fn get_from_path_query(&self,key:&str)->Option<Cow<str>>{
        self.path_query.get_as_str(key)
    }

}

/// for creating context about handling forming http1 request by given bytes
pub enum FormingRequestResult<'a,const HEADERS_COUNT:usize,const PATH_QUERY_COUNT:usize> {
    /// if the request was formatted successfully
    Success(IncomingRequest<'a,HEADERS_COUNT,PATH_QUERY_COUNT>),
    /// if we need to read more from tcp connection stream to get fully request
    ReadMore,
    /// if the request has formatting error
    Err,
}

impl <'a,const HEADERS_COUNT:usize,const PATH_QUERY_COUNT:usize> FormingRequestResult<'a,HEADERS_COUNT,PATH_QUERY_COUNT> {
    // pub (crate) fn is_ok(&self)->bool{
    //     if let FormingRequestResult::Success(_) = self { return  true}
    //     true
    // }
}



#[cfg(test)]
mod test {
    use bytes::{ BytesMut};
    use crate::http::request::{FormingRequestResult, IncomingRequest};
    #[allow(unused_variables)]
    #[test]
    fn test_building_requests_time(){

        // generating http request bytes
        let request_bytes = b"Post /path?id=2 HTTP/1.1\r\n\
        Connection: Keep-Alive\r\n\
        Accept-Encoding: br,deflate\r\n\
        Content-Type: Content-Type: multipart/form-data; boundary=----WebKitFormBoundary7MA4YWxkTrZu0gW\r\n\
        Content-Length: 238\r\n\
        \r\n\
        ----WebKitFormBoundary7MA4YWxkTrZu0gW\r\n\
        Content-Disposition: form-data; name=hellobeltopp\r\n\
        \r\n\
        yes\r\n\
        ----WebKitFormBoundary7MA4YWxkTrZu0gW\r\n\
        Content-Disposition: form-data; name=ellon\r\n\
        \r\n\
        mask\r\n\
        ------WebKitFormBoundary7MA4YWxkTrZu0gW--
        GET /test?id=1 HTTP/1.1\r\n\
        Connection: Keep-Alive\r\n\
        Accept-Encoding: br,deflate\r\n\
        \r\n";


        let inv1 = std::time::SystemTime::now();
        // check http parse crate
        // let mut headers = [httparse::EMPTY_HEADER; 16];
        // let mut req = httparse::Request::new(&mut headers);
        // let res = req.parse(request_bytes);
        // if let Ok(s) = res {
        //     // println!("{:?}",s);
        // }
        //

            let request = IncomingRequest::<'_,16,16>::new(request_bytes);
        match request {
            FormingRequestResult::Success(mut request ) => {
                let data = &request_bytes[request.total_headers_bytes..];
            }
            _ =>  {}
        }



        let inv2 = std::time::SystemTime::now();
        let dif = inv2.duration_since(inv1).unwrap();
        println!("done with {:?}",dif);
        // assert!(request.is_some())
    }


    #[test]

    fn test_reading_from_exact_buffer (){
        let t1 = std::time::SystemTime::now();
        let buf = BytesMut::with_capacity(4082);
        let t2 = std::time::SystemTime::now();
        let dif = t1.duration_since(t2);
        println!("allocating buffer with 4082 capacity takes {:?}",dif);
        let t1 = std::time::SystemTime::now();
        drop(buf);
        let t2 = std::time::SystemTime::now();
        let dif = t1.duration_since(t2);
        println!("deallocating the same buffer takes {:?}",dif);
    }
}


