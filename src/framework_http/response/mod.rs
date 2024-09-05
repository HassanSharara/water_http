
#[macro_use]
pub (crate) mod senders;

#[macro_use]
pub(crate) mod getters;


use std::collections::HashMap;
use std::path::Path;

pub struct HttpResponseHeaders {
    pub first_line:FirstLine,
    pub headers:HashMap<String,String>,
}

impl HttpResponseHeaders {
    pub fn custom(first_line:FirstLine)->Self{
        HttpResponseHeaders{
            first_line,
            headers:HashMap::new(),
        }
    }

    pub fn switching_protocols_headers()->Self{
         HttpResponseHeaders {
            first_line: FirstLine { 
                http_version: HttpVersion::Http1, 
                status: HttpStatus {
                    code: 101,
                    value: "Switching Protocols".to_string()
                }
            },
            headers: HashMap::new()}
    }
    pub fn required_h2_protocol_headers()->Self{
         HttpResponseHeaders {
            first_line: FirstLine {
                http_version: HttpVersion::Http1,
                status: HttpStatus {
                    code: 426,
                    value: "Upgrade Required".to_string()
                }
            },
            headers: HashMap::new()}
    }

    pub fn temporary_redirect_header(url:&str)->Self{
       let mut headers =  HttpResponseHeaders {
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
    pub fn permanent_redirect_header(url:&str)->Self{
       let mut headers =  HttpResponseHeaders {
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
    pub fn found_redirect_header(url:&str)->Self{
       let mut headers =  HttpResponseHeaders {
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
    pub fn required_h2()->Self {
        let mut headers = Self::required_h2_protocol_headers();
        headers.set_header_key_value("Connection","Upgrade");
        headers.set_header_key_value("Upgrade","h2c");
        headers
    }
    pub fn switch_to_h2c_headers()->Self {
        let mut headers = Self::switching_protocols_headers();
        headers.set_header_key_value("Connection","Upgrade");
        headers.set_header_key_value("Upgrade","h2c");
        headers.set_header_key_value("Content-Length","0");
        headers
    }
    pub fn bad_request_headers()->Self{
        HttpResponseHeaders{
            first_line:FirstLine{
                http_version:HttpVersion::Http1_1,
                status:HttpStatus { code: 400 , value: "Bad Request".to_owned() },
            },
            headers:HashMap::new()
        }
    }
   pub fn not_found_headers()->Self{
        HttpResponseHeaders{
            first_line:FirstLine{
                http_version:HttpVersion::Http1_1,
                status:HttpStatus { code: 404 , value: "Not Found".to_owned() },
            },
            headers:HashMap::new()
        }
    }
   pub fn change_first_line(&mut self,first_line: FirstLine){
        self.first_line  = first_line;
    }
   pub fn change_first_line_to_partial_content(&mut self){
        self.change_first_line(FirstLine{
            http_version:HttpVersion::Http1_1,
            status:HttpStatus { code: 206 , value: "Partial".to_owned() },
        });
    }
   pub fn success_partial_content()->Self{
        HttpResponseHeaders{
            first_line:FirstLine{
                http_version:HttpVersion::Http1_1,
                status:HttpStatus { code: 206 , value: "Partial".to_owned() },
            },
            headers:HashMap::new()
        }
    }


    /// for returning success response headers with 200 status code
   pub fn success()->Self{
        HttpResponseHeaders{
            first_line:FirstLine{
                http_version:HttpVersion::Http1_1,
                status:HttpStatus { code: 200 , value: "OK".to_owned() },
            },
            headers:HashMap::new()
        }
    }
    pub fn success_with_content_length(content_length:usize)->Self{
        let mut headers = HttpResponseHeaders{
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
    pub fn with_body(&self,mut bytes:Vec<u8>)->Vec<u8>{
        let mut _bytes = self.to_bytes();
        _bytes.append(&mut bytes);
        _bytes
    }
    pub fn set_header_key_value<>(&mut self,k: impl ToString,v: impl ToString)->Option<String> {
        self.headers.insert(k.to_string(), v.to_string())
    }
    pub fn set_cookie(&mut self,cookie:HttpRequestCookie){
        self.set_header_key_value("Set-Cookie",cookie.value());
    }
    pub fn set_cookies(&mut self,cookies:Vec<HttpRequestCookie>){
        for cookie in cookies {
            self.set_cookie(cookie);
        }
    }
}

pub struct HttpRequestCookie<'a> {
    key:&'a str,
    value:&'a str,
    children:Vec<&'a str>,
}


impl<'a> HttpRequestCookie<'a> {
    pub fn default_cookies_properties()->Vec<&'a str>{
        vec! [
            "Max-Age=7200",
            "path=/",
            "httponly",
            "samesite=lax"
        ]
    }
    pub fn from_key_value(key:&'a str,value:&'a str)->Self{
        HttpRequestCookie {
            key,
            value,
            children: Self::default_cookies_properties()
        }
    }
    pub fn value(&self)-> String {
       let mut result = format!("{}={}",self.key,self.value);
        result.extend(self.children.iter().map(|r| format!("; {}",r)));
        result
    }
}


impl  HttpResponseTrait for HttpResponseHeaders {
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

pub trait  HttpResponseTrait{
    fn to_bytes(&self) -> Vec<u8>;
}

pub enum HttpVersion {
    Http1,
    Http1_1,
    Http2,
    Http3
}
pub struct HttpStatus {
    pub code:u16,
    pub value:String
}
pub struct FirstLine {
    pub http_version:HttpVersion,
    pub status:HttpStatus
}
pub fn content_type_from_file_path(path: &&Path) -> Option<&'static str> {
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

