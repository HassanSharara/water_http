



use std::collections::HashMap;
use std::path::Path;


/// struct for building custom headers as response
pub struct ResponseHeadersBuilder {
    /// [FirstLine] which specify the status of the response and the status code and the version of request
    pub first_line:FirstLine,
    /// for building response headers key and value pairs
    pub headers:HashMap<String,String>,
}

impl ResponseHeadersBuilder {

    /// for building custom [ResponseHeadersBuilder] from custom [FirstLine]
    pub fn custom(first_line:FirstLine)->Self{
        ResponseHeadersBuilder{
            first_line,
            headers:HashMap::new(),
        }
    }

    /// creating headers with switch protocols message
    pub fn switching_protocols_headers()->Self{
         ResponseHeadersBuilder {
            first_line: FirstLine { 
                http_version: HttpVersion::Http1, 
                status: HttpStatus {
                    code: 101,
                    value: "Switching Protocols".to_string()
                }
            },
            headers: HashMap::new()}
    }

    /// creating headers that said using http2 is required
    pub fn required_h2_protocol_headers()->Self{
         ResponseHeadersBuilder {
            first_line: FirstLine {
                http_version: HttpVersion::Http1,
                status: HttpStatus {
                    code: 426,
                    value: "Upgrade Required".to_string()
                }
            },
            headers: HashMap::new()}
    }

    /// creating temporary redirect header with status code 307 (Internal Redirect)
    pub fn temporary_redirect_header(url:&str)->Self{
       let mut headers =  ResponseHeadersBuilder {
            first_line: FirstLine {
                http_version: HttpVersion::Http1,
                status: HttpStatus {
                    code: 307,
                    value: "Internal Redirect".to_string()
                }
            },
            headers: HashMap::new()};
        headers.set_header_key_value("Location",url);
        headers
    }

    /// Creating permanent Redirect header with status code 301 (Permanently Redirect)
    pub fn permanent_redirect_header(url:&str)->Self{
       let mut headers =  ResponseHeadersBuilder {
            first_line: FirstLine {
                http_version: HttpVersion::Http1,
                status: HttpStatus {
                    code: 301,
                    value: "Permanently Redirect".to_string()
                }
            },
            headers: HashMap::new()};
        headers.set_header_key_value("Location",url);
        headers
    }


    /// creating found redirect header with status code 302 (Found Redirect)
    pub fn found_redirect_header(url:&str)->Self{
       let mut headers =  ResponseHeadersBuilder {
            first_line: FirstLine {
                http_version: HttpVersion::Http1,
                status: HttpStatus {
                    code: 302,
                    value: "Found Redirect".to_string()
                }
            },
            headers: HashMap::new()};
        headers.set_header_key_value("Location",url);
        headers
    }

    /// Creating [ResponseHeadersBuilder] with requiring http2 to be the used protocol
    pub fn required_h2()->Self {
        let mut headers = Self::required_h2_protocol_headers();
        headers.set_header_key_value("Connection","Upgrade");
        headers.set_header_key_value("Upgrade","h2c");
        headers
    }

   /// Creating headers that tells the client to switch to http2 protocol
    pub fn switch_to_h2c_headers()->Self {
        let mut headers = Self::switching_protocols_headers();
        headers.set_header_key_value("Connection","Upgrade");
        headers.set_header_key_value("Upgrade","h2c");
        headers.set_header_key_value("Content-Length","0");
        headers
    }
    /// creating headers with bad request status code is 400
    pub fn bad_request_headers()->Self{
        ResponseHeadersBuilder{
            first_line:FirstLine{
                http_version:HttpVersion::Http1_1,
                status:HttpStatus { code: 400 , value: "Bad Request".to_owned() },
            },
            headers:HashMap::new()
        }
    }


    /// creating headers with not found response and the status code is 404
   pub fn not_found_headers()->Self{
        ResponseHeadersBuilder{
            first_line:FirstLine{
                http_version:HttpVersion::Http1_1,
                status:HttpStatus { code: 404 , value: "Not Found".to_owned() },
            },
            headers:HashMap::new()
        }
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
        ResponseHeadersBuilder{
            first_line:FirstLine{
                http_version:HttpVersion::Http1_1,
                status:HttpStatus { code: 206 , value: "Partial".to_owned() },
            },
            headers:HashMap::new()
        }
    }


    /// for returning success response headers with 200 status code
   pub fn success()->Self{
        ResponseHeadersBuilder{
            first_line:FirstLine{
                http_version:HttpVersion::Http1_1,
                status:HttpStatus { code: 200 , value: "OK".to_owned() },
            },
            headers:HashMap::new()
        }
    }

    /// specify content length with success header
    /// notice that specifying content length is very important approach
    pub fn success_with_content_length(content_length:usize)->Self{
        let mut headers = ResponseHeadersBuilder{
            headers:HashMap::new(),
            ..Self::success()
        };
        headers.set_header_key_value("Content-Length",&content_length.to_string() );
        headers
    }
    fn build_headers(&self,bytes:&mut Vec<u8>){
        if  !self.headers.is_empty()
            {
            for (k,v) in self.headers.iter() {
                bytes.extend(k.as_bytes());
                bytes.extend(b": ");
                bytes.extend(v.as_bytes());
                bytes.extend(b"\r\n");
            }
        }
    }

    /// generating the headers with custom body
    pub fn with_body(&self, bytes:&[u8])->Vec<u8>{
        let mut _bytes = self.to_bytes();
        _bytes.extend(bytes);
        _bytes
    }

    /// for setting header key value pair to be sent to the users
    pub fn set_header_key_value<>(&mut self,k: impl ToString,v: impl ToString)->Option<String> {
        self.headers.insert(k.to_string(), v.to_string())
    }

    /// for setting cookie key and value to the client
    pub fn set_cookie(&mut self,cookie:HttpRequestCookie){
        self.set_header_key_value("Set-Cookie",cookie.value());
    }

    /// to set multiple cookies instead of one
    pub fn set_cookies(&mut self,cookies:Vec<HttpRequestCookie>){
        for cookie in cookies {
            self.set_cookie(cookie);
        }
    }
}


/// generating cookie struct to form the data that should be sent to the client
pub struct HttpRequestCookie<'a> {
    key:&'a str,
    value:&'a str,
    children:Vec<&'a str>,
}


impl<'a> HttpRequestCookie<'a> {

    /// adding the default cookies properties
    /// which it`s [
    //             "Max-Age=7200",
    //             "path=/",
    //             "httponly",
    //             "samesite=lax"
    //         ]
    pub fn default_cookies_properties()->Vec<&'a str>{
        vec! [
            "Max-Age=7200",
            "path=/",
            "httponly",
            "samesite=lax"
        ]
    }
    /// creating cookie from key value
    pub fn from_key_value(key:&'a str,value:&'a str)->Self{
        HttpRequestCookie {
            key,
            value,
            children: Self::default_cookies_properties()
        }
    }

    /// getting cookie value from cookie struct
    pub fn value(&self)-> String {
       let mut result = format!("{}={}",self.key,self.value);
        result.extend(self.children.iter().map(|r| format!("; {}",r)));
        result
    }
}


impl  WaterToBytesTrait for ResponseHeadersBuilder {
     fn to_bytes(&self)->Vec<u8>{
        let mut bytes : Vec<u8> = Vec::new();
        // Building The First Line in Http Request
        bytes.extend(
            format!("HTTP/1.1 {} {}\r\n",
                    self.first_line.status.code,
                    self.first_line.status.value
            )
                .as_bytes());
        self.build_headers(&mut bytes);
        bytes.extend(b"\r\n");
        bytes
    }
}


pub (crate) trait  WaterToBytesTrait{
    fn to_bytes(&self) -> Vec<u8>;
}

/// to provide writeable http version to be responded
pub enum HttpVersion {
    Http1,
    Http1_1,
    Http2,
    Http3
}
/// for provide status code and status label for [ResponseHeadersBuilder]
pub struct HttpStatus {
    pub code:u16,
    pub value:String
}

/// wrapper struct for [HttpVersion] and [HttpStatus]
pub struct FirstLine {
    pub http_version:HttpVersion,
    pub status:HttpStatus
}
pub (crate) fn content_type_from_file_path(path: &&Path) -> Option<&'static str> {
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("");

    match extension.to_lowercase().as_str() {
        "ico"=>Some("image/x-icon"),
        "txt" => Some("text/plain"),
        "html" | "htm" => Some("text/html"),
        "css" => Some("text/css"),
        "js" => Some("application/javascript"),
        "xml" => Some("application/xml"),
        "jpeg" | "jpg" => Some("image/jpeg"),
        "png" => Some("image/png"),
        "gif" => Some("image/gif"),
        "bmp" => Some("image/bmp"),
        "webp" => Some("image/webp"),
        "svg" => Some("image/svg+xml"),
        "mp3" => Some("audio/mpeg"),
        "wav" => Some("audio/wav"),
        "ogg" => Some("audio/ogg"),
        "aac" => Some("audio/aac"),
        "midi" => Some("audio/midi"),
        "mp4" => Some("video/mp4"),
        "webm" => Some("video/webm"),
        "avi" => Some("video/x-msvideo"),
        "mpeg" => Some("video/mpeg"),
        "json" => Some("application/json"),
        "pdf" => Some("application/pdf"),
        "zip" => Some("application/zip"),
        "gz" | "gzip" => Some("application/gzip"),
        "bin" => Some("application/octet-stream"),
        "docx" => Some("application/vnd.openxmlformats-officedocument.wordprocessingml.document"),
        "xlsx" => Some("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"),
        "pptx" => Some("application/vnd.openxmlformats-officedocument.presentationml.presentation"),
        "rtf" => Some("application/rtf"),
        "ttf" => Some("font/ttf"),
        "otf" => Some("font/otf"),
        "woff" => Some("font/woff"),
        "woff2" => Some("font/woff2"),
        "tar" => Some("application/x-tar"),
        "rar" => Some("application/vnd.rar"),
        _ => None,
    }
}

