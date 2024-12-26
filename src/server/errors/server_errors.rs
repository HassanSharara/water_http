#![allow(unused)]
use crate::http::status_code::HttpStatusCode;


/// for defining framework errors to enhance debugging and profiling operations
#[derive(Debug,Clone)]
pub enum WaterErrors <'a>{
    Server(ServerError<'a>),
    Http(HttpStatusCode<'a>)
}


impl<'a> From<WaterErrors<'a>> for Result<(), WaterErrors<'a>> {
    fn from(error: WaterErrors<'a>) -> Self {
        Err(error)
    }
}
/// specifying all server errors
#[derive(Debug,Clone)]
pub struct  ServerError<'a> {
     code:u16,
     msg:&'a str
}

impl <'a> ServerError<'a> {

    pub const fn new(code:u16,msg:&'a str)->ServerError<'a>{
        ServerError {
            code,
            msg
        }
    }
}



macro_rules! form_server_errors {
    {
        $($(#[$docs:meta])* $name:ident = $code:expr => $msg:expr;)*
    } => {
       impl <'a> ServerError<'a> {
           $(
            $(#[$docs])*
            pub const $name:ServerError<'static> = ServerError::new($code,$msg);
           )*
       }
    };
}


form_server_errors!{
    /// this error when the server trying to make sure that all written data had been flushed
    /// to the incoming stream and fail during that
    FLUSH_DATA_TOSTREAM_ERROR = 39 => "could not flush data to stream";
    /// when the server trying to parse incoming request body
    /// and fail to the malicious bytes or under attack or bad request
    HANDLING_INCOMING_BODY_ERROR = 40 => "could not handle incoming body";
    /// could not find boundary
    MULTIPARTFORMDATA_ERROR = 41 => "could not parse multipart form data";
    /// error while writing to the stream
    WRITING_TO_STREAM_ERROR = 42 => "encounter error while writing to the stream";
}

