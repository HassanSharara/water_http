use std::fmt::{Display};
use bytes::{BytesMut};
use crate::http::status_code::HttpStatusCode as StatusCode;



/// for writing http response format bytes to given buffer
pub struct HttpResponseBufferWriter<'a> {
    /// writable given buffer
    write_able_buffer:&'a mut BytesMut
}




 impl<'a> HttpResponseBufferWriter<'a> {
     /// for creating ok response with 200 status code
     #[inline]
     pub (crate) fn success(buffer:&'a mut BytesMut)->HttpResponseBufferWriter<'a>{
         Self::with_version_and_status(buffer,StatusCode::OK)
     }

     pub fn new(buffer:&'a mut BytesMut)->HttpResponseBufferWriter<'a>{
         HttpResponseBufferWriter{
             write_able_buffer:buffer
         }
     }

     /// for creating ok response with 200 status code
     #[inline]
     pub fn with_version_and_status(buffer:&'a mut BytesMut,status:StatusCode)->HttpResponseBufferWriter<'a>{
         buffer.extend_from_slice(format!("HTTP/1.1 {} {}\r\n",
          status.status,
          status.label
         ).as_bytes());
         HttpResponseBufferWriter {
             write_able_buffer:buffer
         }
     }
     
     /// for setting header key and value pair to response
     #[inline]
     pub fn set_header_pair(&mut self,key:impl Display,value:impl Display){
         self.write_able_buffer.extend_from_slice(
             format!("{key}: {value}\r\n")
                 .as_bytes()
         );
     }
     
     /// setting data to the response
     #[inline]
     pub fn send_data_with_content_length(&mut self,data:ResponseData<'a>){
         let data = data.as_bytes();
         self.write_able_buffer.extend_from_slice(format!("Content-Length: {}\r\n\r\n",data.len()).as_bytes());
         self.write_able_buffer.extend_from_slice(data);
     }

     /// setting custom data to the response buffer without counting content-length
     /// so that means that you need to count content-length and write it to headers manually
     #[inline]
     pub fn send_data(&mut self,data:ResponseData<'a>){
         let data = data.as_bytes();
         self.write_able_buffer.extend_from_slice(data);
     }
 }

 /// for specifying response data
 pub enum ResponseData<'a> {
     Str(&'a str),
     Slice(&'a [u8]),
     String(String),
 }

impl<'a> ResponseData<'a> {

    pub  fn as_bytes(&'a self)->&'a[u8]{
        match self {
            ResponseData::Str(s) => {s.as_bytes()}
            ResponseData::Slice(s) => {*s}
            ResponseData::String(s) => {s.as_bytes()}
        }
    }
}
#[cfg(test)]
mod test_buffer {
    use bytes::BytesMut;
    use crate::http::status_code::HttpStatusCode as StatusCode;
    use crate::http::HttpVersion;

    #[test]
    pub fn buffer_writing_speed(){
        let status = StatusCode::OK;
        let version = HttpVersion::Http1_1.to_str();


        let v1 = std::time::SystemTime::now();
        let mut buffer =BytesMut::with_capacity(4048);
        buffer.extend_from_slice(version.as_bytes());
        buffer.extend_from_slice(b" ");
        let (code,la) = status.to_bytes();
        buffer.extend_from_slice(format!("{}",code).as_bytes());
        buffer.extend_from_slice(b" ");
        buffer.extend_from_slice(la);
        buffer.extend_from_slice(b"\r\n");
        let v2 = std::time::SystemTime::now();
        let dif = v1.duration_since(v2);
        println!("difference with each writing mod {:?}",dif);

        let v1 = std::time::SystemTime::now();
        let mut buffer =BytesMut::with_capacity(4048);
        buffer.extend_from_slice(
            format!("{} {} {}\r\n",version,status.status,status.label).as_bytes()
        );
        let v2 = std::time::SystemTime::now();
        let dif = v1.duration_since(v2);
        println!("difference with each format mod {:?}",dif);
    }
}

