mod writer;
mod sender;
mod file_response;
pub use file_response::*;
pub use writer::*;
pub use sender::*;

#[doc(hidden)]
pub struct HeaderResponseBuilder {
      first_line: FirstLine,
      data:Vec<u8>,
}
impl HeaderResponseBuilder {


    pub fn custom(
        first_line: FirstLine)->Self
    {
        let  data = Vec::with_capacity(1024);
        Self {
            first_line,
            data
        }
    }


    pub fn to_bytes(&self)->Vec<u8>{
        let mut f = self.first_line.to_bytes();
        f.extend_from_slice(&self.data);
        f.extend_from_slice(b"\r\n");
        f
    }
    pub fn set_header_key_value(&mut self,key:impl std::fmt::Display,value:impl std::fmt::Display){
        self.data
            .extend_from_slice(
                format!("{key}: {value}\r\n").as_bytes()
            );
    }
    /// creating headers with switch protocols message
    pub fn switching_protocols_headers()->Self{
        Self::custom(
            FirstLine {
                http_version: HttpVersion::Http1,
                status: HttpStatus {
                    code: 101,
                    value: "Switching Protocols".to_string()
                }
            }
        )
    }

    /// creating headers that said using http2 is required
    pub fn required_h2_protocol_headers()->Self{
        Self::custom(
            FirstLine {
                http_version: HttpVersion::Http1,
                status: HttpStatus {
                    code: 426,
                    value: "Upgrade Required".to_string()
                }
            }
        )
    }


    /// creating temporary redirect header with status code 307 (Internal Redirect)
    pub fn temporary_redirect_header(url:&str)->Self{
        let mut headers =  Self::custom(
            FirstLine {
                http_version: HttpVersion::Http1,
                status: HttpStatus {
                    code: 307,
                    value: "Internal Redirect".to_string()
                }
            }
        );
        headers.set_header_key_value("Location",url);
        headers
    }

    /// Creating permanent Redirect header with status code 301 (Permanently Redirect)
    pub fn permanent_redirect_header(url:&str)->Self{
        let mut headers =  Self::custom(
            FirstLine {
                http_version: HttpVersion::Http1,
                status: HttpStatus {
                    code: 301,
                    value: "Permanently Redirect".to_string()
                }
            }
        );
        headers.set_header_key_value("Location",url);
        headers
    }


    /// creating found redirect header with status code 302 (Found Redirect)
    pub fn found_redirect_header(url:&str)->Self{
        let mut headers =  Self::custom(
            FirstLine {
                http_version: HttpVersion::Http1,
                status: HttpStatus {
                    code: 302,
                    value: "Found Redirect".to_string()
                }
            }
        );
        headers.set_header_key_value("Location",url);
        headers
    }

    /// Creating `ResponseHeadersBuilder` with requiring http2 to be the used protocol
    pub fn required_h2()->Self {
        let mut headers = Self::required_h2_protocol_headers();
        headers.set_header_key_value("connection","Upgrade");
        headers.set_header_key_value("Upgrade","h2c");
        headers
    }

    /// Creating headers that tells the client to switch to http2 protocol
    pub fn switch_to_h2c_headers()->Self {
        let mut headers = Self::switching_protocols_headers();
        headers.set_header_key_value("connection","Upgrade");
        headers.set_header_key_value("Upgrade","h2c");
        headers.set_header_key_value("Content-Length","0");
        headers
    }
    /// creating headers with bad request status code is 400
    pub fn bad_request_headers()->Self{
        Self::custom(
            FirstLine{
                http_version:HttpVersion::Http1_1,
                status:HttpStatus { code: 400 , value: "Bad Request".to_owned() },
            }
        )
    }


    /// creating headers with not found response and the status code is 404
    pub fn not_found_headers()->Self{
        Self::custom(
            FirstLine{
                http_version:HttpVersion::Http1_1,
                status:HttpStatus { code: 404 , value: "Not Found".to_owned() },
            }
        )
    }

    /// for changing the first line of the current header
    pub fn change_first_line(&mut self,first_line: FirstLine){
        self.first_line  = first_line;
    }

    /// making the header have partial content with status code 206
    /// it`s meaning that the response would be sent is not the full response from the server
    pub fn change_first_line_to_partial_content(&mut self){
        self.change_first_line(FirstLine{
            http_version:HttpVersion::Http1_1,
            status:HttpStatus { code: 206 , value: "Partial".to_owned() },
        });
    }


    /// creating headers with partial  content and the status code is 206
    pub fn success_partial_content()->Self{
        Self::custom(
            FirstLine{
                http_version:HttpVersion::Http1_1,
                status:HttpStatus { code: 206 , value: "Partial".to_owned() },
            }
        )
    }


    /// for returning success response headers with 200 status code
    pub fn success()->Self{
        Self::custom(
            FirstLine{
                http_version:HttpVersion::Http1_1,
                status:HttpStatus { code: 200 , value: "OK".to_owned() },
            }
        )
    }

}
/// to provide writeable http version to be responded
pub enum HttpVersion {
    Http1,
    Http1_1,
    Http2,
    Http3
}

impl HttpVersion {

    /// converting http version to bytes
    /// #[&[u8]]
    pub const fn to_bytes(&self)->&[u8]{
            self.to_str().as_bytes()
    }


    /// converting http version to string slice
    /// #[&str]
    pub const fn to_str(&self)->&str{
        match self {
            HttpVersion::Http1 => {"HTTP/1.0"}
            HttpVersion::Http1_1 => {"HTTP/1.1"}
            HttpVersion::Http2 => {"HTTP/2"}
            HttpVersion::Http3 => {"HTTP/3"}
        }
    }
}
/// for provide status code and status label for `ResponseHeadersBuilder`
pub struct HttpStatus {
    pub code:u16,
    pub value:String
}




/// wrapper struct for [HttpVersion] and [HttpStatus]
pub struct FirstLine {
    pub http_version:HttpVersion,
    pub status:HttpStatus
}
impl FirstLine {
    pub fn to_bytes(&self)->Vec<u8>{
        let mut bytes = Vec::with_capacity(1024);
        let version =
        match self.http_version {
            HttpVersion::Http1 => { "HTTP/1.0" }
            HttpVersion::Http1_1 => { "HTTP/1.1" }
            HttpVersion::Http2 => { "HTTP/2" }
            HttpVersion::Http3 => { "HTTP/3" }
        }.as_bytes();
        bytes.extend_from_slice(version);
        bytes.extend_from_slice(format!(" {} {}\r\n",self.status.code,self.status.value).as_bytes());
        bytes
    }
}
#[doc(hidden)]
pub struct BodyResponseBuilder<'a> {
    mechanism:HandlingResponseMechanism<'a>,
}
impl <'a> BodyResponseBuilder<'a> {
    pub fn new(mechanism:HandlingResponseMechanism<'a>)->Self{
        Self{
            mechanism
        }
    }
}

#[doc(hidden)]
pub enum HandlingResponseMechanism<'a> {
    File(&'a[&'a str]),
    NormalResponse(&'a[u8]),
    None
}

/// for creating custom response
#[doc(hidden)]
pub struct ResponseBuilder<'a>{

    pub headers:HeaderResponseBuilder,
    body:BodyResponseBuilder<'a>
}


// for building custom response for clients within http request
impl <'a> ResponseBuilder<'a> {


    /// return empty body response with
    pub fn empty()->ResponseBuilder<'a>{
        Self{
            headers:HeaderResponseBuilder::success(),
            body:BodyResponseBuilder::new(HandlingResponseMechanism::NormalResponse(
                b""
            ))
        }
    }
    /// for sending custom bytes as server response
    pub fn from_bytes_response(bytes:&'a [u8])->ResponseBuilder<'a>{
        let mut headers = HeaderResponseBuilder::success();
        headers.set_header_key_value("Content-Length",bytes.len());
        let  body =
        BodyResponseBuilder::new(
            HandlingResponseMechanism::NormalResponse(
                bytes
            )
        );
        Self {
            headers,
            body
        }
    }


    /// for sending ['&str'] data type as response
    pub fn from_str(str:&'a str)->ResponseBuilder<'a>{
        let  res = Self::from_bytes_response(str.as_bytes());
        res
    }

    /// sending ref [String] type to clients as response
    pub fn from_string_ref(data:&'a String)->ResponseBuilder<'a>{ Self::from_bytes_response(data.as_bytes())}


    pub  fn to_bytes(&self)->Option<Vec<u8>>{
        let this = self;
        let mut res = this.headers.first_line.to_bytes();
        res.extend_from_slice(this.headers.data.as_slice());
        match &self.body.mechanism {
            HandlingResponseMechanism::NormalResponse(bytes)=>{
                res.extend_from_slice(b"\r\n");
                res.extend_from_slice(*bytes);
                Some(res)
            }
            HandlingResponseMechanism::File(_files)=>{
                None
            }
            _ => {
                None
            }
        }
    }
}


