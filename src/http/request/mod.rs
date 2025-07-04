mod body;
mod header;
mod getter;
pub use getter::*;
use std::borrow::Cow;
use std::fmt::{Display,  Formatter};
pub use body::*;
pub use crate::http::request::header::{KeyValueList};
use water_http_utils::request::{CreatingRequestErrors, HttpRequest,headers::HttpHeaders};
pub use water_http_utils::request::HttpFirstLine;





/// a structr for holding references to
/// buffer bytes with zero copy cost
pub  struct IncomingRequest<'a,const HEADERS_COUNT:usize,const PATH_QUERY_COUNT:usize> {
    http_request:HttpRequest<'a,HEADERS_COUNT>,
}


impl<'a,const HEADERS_COUNT:usize,const PATH_QUERY_COUNT:usize> Display for  IncomingRequest<'a,HEADERS_COUNT,PATH_QUERY_COUNT> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut output = String::new();
        for i in self.http_request.headers().lines() {
            output.push_str(&format!("\r\n{}: {}",i.key,i.value.to_str()));
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
impl <'a,const HEADERS_COUNT:usize,const PATH_QUERY_COUNT:usize>
IncomingRequest<'a, HEADERS_COUNT,PATH_QUERY_COUNT>

{

    /// getting total headers length
    pub fn get_total_headers_length(&self)->usize{
        self.http_request.headers().headers_length
        +
            self.http_request.first_line().first_line_length
    }

    /// first line
    pub fn first_line(&self)->&HttpFirstLine<'a>{
        self.http_request.first_line()
    }

    /// get headers
    pub fn headers(&self) -> &HttpHeaders<'a,HEADERS_COUNT>{
        self.http_request.headers()
    }

    ///
    /// for creating new request from incoming bytes
    /// and returning back where the headers of this request
    /// is end or at what position
    /// # return [FormingRequestResult]
     pub  fn new(payload:&'a[u8])->FormingRequestResult<'a,HEADERS_COUNT,PATH_QUERY_COUNT> {
        let request = HttpRequest::<HEADERS_COUNT>::from_incoming_bytes(payload);
        return match request {
            Ok(request) => {
                FormingRequestResult::Success(
                    IncomingRequest {
                        http_request:request,
                    }
                )
            }
            Err(e) => {
               return  match e {
                    CreatingRequestErrors::InsufficientDataSoReadMore => { return FormingRequestResult::ReadMore}
                    _ => {
                         FormingRequestResult::Err(e)
                    }
                }
            }
        }
    }


    /// for getting the path of the incoming request
    ///
    /// # return [`&str`] which 'a stands for the life of incoming request
    pub fn path(&'a self)->&'a str {
        self.http_request.path().to_str()
    }

    /// parsing incoming http method
    pub fn method(&self)->& str {
        self.http_request.method()
    }

    /// parsing incoming http version
    pub fn http_version(&self)->&str {
        self.http_request.version()
    }

    /// getting content length in headers
    pub fn content_length(&self)->Option<usize> {
        if let Some(c) = self.http_request.headers().get("Content-Length") {
            if let Ok(v) = c.to_str().parse::<usize>() {
                return Some(v);
            }
        }
        None
    }


    /// getting data from path query
    /// for examples
    /// - http:://examples.com/posts?id=1 the key is `id` and the value is `1`
    /// - http:://examples.com/posts?year=2024 the key is `year` and the value is `2024`
    pub fn get_from_path_query(&self,key:&str)->Option<Cow<str>>{
      let (_,query) =    self.http_request.path().split_to_path_and_query();
        if let Some(v) = query.get(key) {
            return Some(Cow::Owned(v.into()));
        }
        None
    }

}

/// for creating context about handling forming http1 request by given bytes
pub enum FormingRequestResult<'a,const HEADERS_COUNT:usize,const PATH_QUERY_COUNT:usize> {
    /// if the request was formatted successfully
    Success(IncomingRequest<'a,HEADERS_COUNT,PATH_QUERY_COUNT>),
    /// if we need to read more from tcp connection stream to get fully request
    ReadMore,
    /// if the request has formatting error
    Err(CreatingRequestErrors),
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
            FormingRequestResult::Success( request ) => {
                let data = &request_bytes[request.http_request.headers().headers_length+1..];
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


