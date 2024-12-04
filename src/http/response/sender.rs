use std::fmt::Display;
use std::io::SeekFrom;
use std::os::windows::fs::MetadataExt;
use bytes::{Bytes, BytesMut};
use h2::{RecvStream, SendStream};
use h2::server::SendResponse;
use http::{HeaderName, HeaderValue, Request, Response as H2Response, response::Builder as H2ResponseBuilder};
use tokio::io::{ AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use crate::http::{FileRSender, HeaderResponseBuilder, ResponseData};
use crate::http::status_code::{HttpStatusCode as StatusCode};
use crate::server::connection::handle_responding;
use crate::server::errors::{ServerError, WaterErrors};
use crate::server::HttpStream;


pub  trait HttpSenderTrait {
   /// for setting http status code for response
   fn send_status_code(&mut self,http_status: StatusCode);

    /// to send partial response
   fn send_data_partial(&mut self,data:ResponseData);

    /// send final response or full response
   fn send_data_as_final_response(&mut self,data:ResponseData);

    /// for setting header key value to response holder

   fn set_header<K:Display, V:Display>(&mut self,key:K,value:V);

    /// to send [&str] as response to client
   fn send_str(&mut self,data:&str);

   /// to send files as response to client ,
   ///and it takes [FileRSender]
   fn send_file(&mut self,path:FileRSender<'_>)->
                                                     impl std::future::Future<
                                                         Output = Result<(),&str>> + Send;

    /// for writing custom bytes to the stream
    fn write_custom_bytes(&mut self,bytes:&[u8])->
     impl std::future::Future<
     Output = Result<(),WaterErrors>> + Send;
}



pub (crate) struct  Http2Sender<'a> {
    batch:&'a mut (Request<RecvStream>,SendResponse<Bytes>),
    send_stream: Option<SendStream<Bytes>>,
    response_builder:Option<H2ResponseBuilder>
}

impl <'a> Http2Sender<'a>{
    pub (crate) fn new(
        batch:&'a mut (Request<RecvStream>,SendResponse<Bytes>),
    )->Http2Sender<'a> {
        Http2Sender {
            batch,
            send_stream: None,
            response_builder:None
        }
    }
}

impl<'a> HttpSenderTrait for Http2Sender<'a> {
    fn send_status_code(&mut self, http_status: StatusCode) {
        if self.response_builder.is_some() {return;}
        let   response = H2Response::
            builder().
            status(http_status.status.get());
        self.response_builder = Some(response);
    }

    fn send_data_partial(&mut self, data: ResponseData) {
        let data = data.as_bytes().to_vec();
        if let Some(ref mut stream) = self.send_stream {
            _=stream.send_data(Bytes::from(data),false);
            return;
        } else if let Some( response_builder) = self.response_builder.take() {
            let   sender = &mut self.batch.1;
            if let Ok(mut stream) = sender.send_response(response_builder.body(()).unwrap(),false) {
                _=stream.send_data(Bytes::from(data),false);
            }
        }
    }

    fn send_data_as_final_response(&mut self, data: ResponseData) {

        if let Some(ref mut stream) = self.send_stream {
            let data = data.as_bytes().to_vec();
            _=stream.send_data(Bytes::from(data),true);
            return;
        } else if let Some( response_builder) = self.response_builder.take() {
            let data = data.as_bytes().to_vec();
            let   sender = &mut self.batch.1;
            if let Ok(mut stream) = sender.send_response(response_builder.body(()).unwrap(),false) {
                _=stream.send_data(Bytes::from(data),true);
            }
        } else {
            self.send_status_code(StatusCode::OK);
            self.send_data_as_final_response(data);
        }
    }

    fn set_header<K: Display, V: Display>(&mut self, key: K, value: V) {
        let is_status_written = self.response_builder.is_some();
        if !is_status_written {return;}
        let res = self.response_builder.as_mut();
        if let Some(res) = res {
            if let Some(headers ) = res.headers_mut() {
                headers.insert(HeaderName::from_bytes(
                    format!("{key}").as_bytes()
                ).unwrap(),HeaderValue::from_bytes(
                    format!("{value}").as_bytes()
                ).unwrap());
            }
        } else {
            self.send_status_code(StatusCode::OK);
            self.set_header(key,value);
        }
    }

    fn send_str(&mut self, data: &str) {
        self.send_data_as_final_response(ResponseData::Str(data));
    }

    async fn send_file(&mut self, pc: FileRSender<'_>) -> Result<(),&str> {
        let ref path = pc.path;
        let file_name = match path.file_name() {
            Some(f_n) => f_n,
            None => return Err("the path is not valid")
        };
        let file = tokio::fs::File::open(path).await;
        if let Ok( mut file ) = file {
            let meta_data = match file.metadata().await {
                Ok(m) => { m}
                Err(_) => {
                    return Err("an error with metadata of the given file")
                }
            };
            let file_size = meta_data.file_size() as usize;
            let file_content_type = crate::util::content_type_from_file_path(path);
            let mut h2_res_builder = H2Response::builder()
                .status(200);
            // check the content type from file name
            match file_content_type {
                None => {
                    // start sending file as octet-stream content
                    h2_res_builder = h2_res_builder.header("Content-Type","Application/octet-stream");
                    h2_res_builder = h2_res_builder.header("Content-Disposition",format!("attachment; filename={}",file_name.to_str().unwrap_or("")));
                }
                Some(content_type) => {
                    h2_res_builder = h2_res_builder.header("Content-Type",content_type);
                }
            }
            h2_res_builder = h2_res_builder.header("Accept-Ranges","bytes");

            let last_index = if file_size > pc.buffer_size_for_reading_from_file_and_writing_to_stream {
                pc.buffer_size_for_reading_from_file_and_writing_to_stream
            }  else {
                file_size
            };
            let mut start:usize = 0;
            let mut end:usize =
                start
                    + last_index;
            let mut content_length:usize = (end - start)   +1  ;
            // start handle ranges
            if let Some(range) = &pc.range {
                start = range.0.unwrap_or(start);
                if let Some(e) = range.1 {
                    if e <= file_size {
                        end = e;
                    }
                }
                if end < start {
                    end = start + last_index;
                }
                if end >= file_size {
                    end = file_size;
                }
                content_length = (end - start)   +1 ;

                h2_res_builder = h2_res_builder.header("Content-Range",format!("bytes {start}-{end}/{}",file_size-1));
            }


            h2_res_builder = h2_res_builder.status(206);


            // checking range validity
            if start == end || start > end || end > file_size  {
                return Err("Ranges Not Satisfiable");
            }

            // initiating buffer with the appropriate buffer size
            let buffer_size = if content_length >
                pc.buffer_size_for_reading_from_file_and_writing_to_stream {
                pc.buffer_size_for_reading_from_file_and_writing_to_stream
            } else {content_length };


            // specifying content-length
            h2_res_builder = h2_res_builder.header("Content-Length",content_length);
            h2_res_builder = h2_res_builder.header("Access-Control-Allow-Origin","*");

            // sending headers firstly to make sure that connection stream is a live
            // and also for acknowledge the client that we ready to sent back what`s he need



            let   sender = &mut self.batch.1;
            let mut sender =
            match sender.send_response(h2_res_builder.body(()).unwrap(),false) {
                Ok(sender) => {sender}
                Err(_) => { return Err("can not send response")}
            };


            // allocate memory for reading file from disk
            let mut buffer= Vec::<u8>::with_capacity(buffer_size);

            // specifying the remaining or not sent data
            let mut remaining = content_length;

            // seeking to the start of range bytes
            if let Err(_) = file.seek(SeekFrom::Start(start as u64)).await {
                return Err("can not seek to the given bytes")
            }

            // sending back all the wanted data
            while remaining > 0 {
                if let Ok( size) = file.read_buf(&mut buffer).await  {
                    let to_send = size.min(remaining);
                    match sender.send_data(Bytes::copy_from_slice(&buffer[..to_send]), false) {
                        Ok(_) => {
                            remaining-=to_send;
                            if remaining < 1  {
                                break;
                            }
                            buffer.clear();
                        }
                        Err(_) => {
                            return Err("can not write bytes to stream")

                        }
                    }

                } else {
                    return Err("can not read file successfully")
                }
            }
        }
        return Ok(())

    }

    async fn write_custom_bytes(&mut self, bytes: &[u8]) -> Result<(), WaterErrors> {
        if let Some(send_stream) = &mut self.send_stream {
            if let Ok(_) = send_stream.send_data(Bytes::copy_from_slice(bytes),false) {
                return  Ok(())
            }
        }

        Err(
            WaterErrors::Server(
                ServerError::WRITING_TO_STREAM_ERROR
            )
        )
    }
}



/// for sending http response to all supported protocols by the crate
pub  enum HttpSender<'a> {
    H1(Http1Sender<'a>),
    H2(Http2Sender<'a>),
}

 impl<'a> HttpSenderTrait for HttpSender<'a> {
    fn send_status_code(&mut self, http_status: StatusCode) {
        match self {
            HttpSender::H1(h1) => {
                h1.send_status_code(http_status)
            }
            HttpSender::H2(h2) => {
                h2.send_status_code(http_status)
            }
        }
    }

    fn send_data_partial(&mut self, data: ResponseData) {
        match self {
            HttpSender::H1(h1) => {
                h1.send_data_partial(data)
            }
            HttpSender::H2(h2) => {
                h2.send_data_partial(data)
            }
        }
    }

    fn send_data_as_final_response(&mut self, data: ResponseData) {
        match self {
            HttpSender::H1(h1) => {
                h1.send_data_as_final_response(data)
            }
            HttpSender::H2(h2) => {
                h2.send_data_as_final_response(data)
            }
        }
    }

    fn set_header<K: Display, V: Display>(&mut self, key: K, value: V) {
        match self {
            HttpSender::H1(h1) => {
                h1.set_header(key,value)
            }
            HttpSender::H2(h2) => {
                h2.set_header(key,value)
            }
        }
    }

    fn send_str(&mut self, data: &str) {
        match self {
            HttpSender::H1(h1) => {
                h1.send_str(data)
            }
            HttpSender::H2(h2) => {
                h2.send_str(data)
            }
        }
    }

    async fn send_file(&mut self, pc: FileRSender<'_>) -> Result<(), &str> {
        match self {
            HttpSender::H1(h1) => {h1.send_file(pc).await}
            HttpSender::H2(h2) => {h2.send_file(pc).await}
        }
    }

     async fn write_custom_bytes(&mut self, bytes: &[u8]) -> Result<(), WaterErrors> {
         match self {
             HttpSender::H1(h1) => {h1.write_custom_bytes(bytes).await}
             HttpSender::H2(h2) => {h2.write_custom_bytes(bytes).await}
         }
     }
 }



pub (crate) struct Http1Sender<'a,
> {
    pub buf:&'a mut BytesMut,
    stream:&'a mut HttpStream,
    is_status_written:bool,
}

impl <'a> Http1Sender<'a> {
    pub (crate) fn new(
       buf:&'a mut BytesMut,
       stream:&'a mut HttpStream
    )->Http1Sender<'a>{
        Http1Sender {
            buf,
            is_status_written:false,
            stream
        }
    }


    pub (crate) async fn write_bytes(&mut self,bytes:&[u8])->Result<(),()>{
        match self.stream.write_all(bytes).await {
            Ok(_) => {Ok(())}
            Err(_) => {Err(())}
        }
    }
}
impl<'a> HttpSenderTrait for Http1Sender <'a>  {
    fn send_status_code(&mut self, http_status: StatusCode) {
        self.buf.extend_from_slice(format!("HTTP/1.1 {} {}\r\n",
         http_status.status,
         http_status.label
        ).as_bytes());
        self.is_status_written = true;
    }

    #[inline]
    fn send_data_partial(&mut self, data: ResponseData) {
        let bytes = data.as_bytes();
        self.buf.extend_from_slice(b"\r\n");
        self.buf.extend_from_slice(bytes);
    }

    #[inline]
    fn send_data_as_final_response(&mut self, data: ResponseData) {
        let data = data.as_bytes();
        self.buf.extend_from_slice(format!("Content-Length: {}\r\n\r\n",data.len()).as_bytes());
        self.buf.extend_from_slice(data);
    }

    fn set_header<K:Display, V:Display>(&mut self, key: K, value: V) {
        if !self.is_status_written { self.send_status_code(StatusCode::OK);}
        self.buf.extend_from_slice(format!("{key}: {value}\r\n").as_bytes());
    }

    fn send_str(&mut self,data: &str)  {
        self.send_status_code(StatusCode::OK);
        self.send_data_as_final_response(ResponseData::Str(data));
    }

    async fn send_file(&mut self,pc: FileRSender<'_>)->Result<(),&str>{
        let ref path = pc.path;
        let file_name = match path.file_name() {
            Some(f_n) => f_n,
            None => return Err("the path is not valid")
        };
        let file = tokio::fs::File::open(path).await;
        if let Ok( mut file ) = file {
            let meta_data = match file.metadata().await {
                Ok(m) => { m}
                Err(_) => {
                    return Err("an error with metadata of the given file")
                }
            };
            let file_size = meta_data.file_size() as usize;
            let file_content_type = crate::util::content_type_from_file_path(path);
            let mut headers_builder = HeaderResponseBuilder::success();

            // check the content type from file name
            match file_content_type {
                None => {
                    // start sending file as octet-stream content
                    headers_builder.set_header_key_value("Content-Type","Application/octet-stream");
                    headers_builder.set_header_key_value("Content-Disposition",format!("attachment; filename={}",file_name.to_str().unwrap_or("")));
                }
                Some(content_type) => {
                    headers_builder.set_header_key_value("Content-Type",content_type);
                }
            }
            headers_builder.set_header_key_value("Accept-Ranges","bytes");

            let last_index = if file_size > pc.buffer_size_for_reading_from_file_and_writing_to_stream {
                pc.buffer_size_for_reading_from_file_and_writing_to_stream
            }  else {
                file_size
            };
            let mut start:usize = 0;
            let mut end:usize =
             start
            + last_index;
            let mut content_length:usize = (end - start)   +1  ;
            // start handle ranges
            if let Some(range) = &pc.range {
                start = range.0.unwrap_or(start);
                if let Some(e) = range.1 {
                    if e <= file_size {
                        end = e;
                    }
                }
                if end < start {
                    end = start + last_index;
                }
                if end >= file_size {
                    end = file_size;
                }
                content_length = (end - start)   +1 ;

                headers_builder.set_header_key_value("Content-Range",format!("bytes {start}-{end}/{}",file_size-1));
            }



            headers_builder.change_first_line_to_partial_content();

            // checking range validity
            if start == end || start > end || end > file_size  {
                return Err("Ranges Not Satisfiable");
            }

            // initiating buffer with the appropriate buffer size
            let buffer_size = if content_length> pc.buffer_size_for_reading_from_file_and_writing_to_stream {
                pc.buffer_size_for_reading_from_file_and_writing_to_stream
            } else {content_length };


            // specifying content-length
            headers_builder.set_header_key_value("Content-Length",content_length);
            headers_builder.set_header_key_value("Access-Control-Allow-Origin","*");

            // sending headers firstly to make sure that connection stream is a live
            // and also for acknowledge the client that we ready to sent back what`s he need
            if let Err(_) = self.write_bytes(&headers_builder.to_bytes()).await {
                return Err("can not send write headers to connection peer")
            }

            // allocate memory for reading file from disk
            let mut buffer= Vec::<u8>::with_capacity(buffer_size);

            // specifying the remaining or not sent data
            let mut remaining = content_length;

            // seeking to the start of range bytes
            if let Err(_) = file.seek(SeekFrom::Start(start as u64)).await {
                return Err("can not seek to the given bytes")
            }

            // sending back all the wanted data
            while remaining > 0 {
                if let Ok( size) = file.read_buf(&mut buffer).await  {
                    let to_send = size.min(remaining);
                    match self.write_bytes(&buffer[..to_send]).await {
                        Ok(_) => {
                            remaining-=to_send;
                            if remaining < 1  {
                                break;
                            }
                            buffer.clear();
                        }
                        Err(_) => {
                            return Err("can not write bytes to stream")
                        }
                    }
                } else {
                    return Err("can not read file successfully")
                }
            }

            return Ok(())

        }
        Err("file dose not exist")
    }

    async fn write_custom_bytes(&mut self, bytes: &[u8]) -> Result<(), WaterErrors> {
        self.buf.extend_from_slice(bytes);
        if handle_responding(self.buf,self.stream).await.is_err() {
            return Err(WaterErrors::Server(ServerError::WRITING_TO_STREAM_ERROR))
        }
        Ok(())
    }
}




