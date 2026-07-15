#![allow(async_fn_in_trait)]

use std::ffi::OsStr;
use std::fmt::Display;
use std::future::Future;
#[cfg(not(feature = "use_io_uring"))]
use std::io:: SeekFrom;

#[cfg(not(any(feature = "use_only_http1", feature = "use_io_uring")))]
use bytes::Bytes;
#[cfg(not(any(feature = "use_only_http1", feature = "use_io_uring")))]
use h2::SendStream;
#[cfg(not(any(feature = "use_only_http1", feature = "use_io_uring")))]
use http::{HeaderName, HeaderValue, Response as H2Response, response::Builder as H2ResponseBuilder};
use integer_to_bytes::HumanInt;
use serde::de::Error;
use serde::Serialize;
#[cfg(not(feature = "use_io_uring"))]
use tokio::io::{AsyncSeekExt,AsyncWriteExt,AsyncReadExt};
use water_buffer::WaterBuffer;
use crate::http::{FileRSender, ResponseData};
use crate::http::status_code::{HttpStatusCode as StatusCode, HttpStatusCode};
use crate::server::connection::handle_responding;
use crate::server::errors::{ServerError, WaterErrors};
use crate::server::{ContextAccessories, Http1Context, WRITING_FILES_BUF_LEN};

#[cfg(feature = "auto_encode_response")]
use crate::server::get_server_config;


#[cfg(feature = "use_io_uring")]
use crate::server::HttpStream;
#[cfg(not(any(feature = "use_only_http1", feature = "use_io_uring")))]
use crate::server::Http2Context;




/// for providing easy access and use for sends methods
/// and providing infra structure for http protocols handling
pub  trait HttpSenderTrait {
   /// for setting http status code for response
   fn send_status_code(&mut self,http_status: StatusCode);

    /// to send partial response
   fn send_data_partial(&mut self,data:ResponseData);

    /// send final response or full response
   fn send_data_as_final_response(&mut self,data:ResponseData)->impl Future<Output=Result<(),()>>;

    /// for setting header key value to response holder

   fn set_header<K:Display, V:Display>(&mut self,key:K,value:V);
    /// for setting header with the highest efficiency
   fn set_header_ef<'h,K:Into<WaterBytes<'h>>, V:Into<WaterBytes<'h>>>(&mut self,key:K,value:V);

    fn send_json<JSON:Serialize>(&mut self,value:&JSON)->
    impl Future<Output=serde_json::Result<()>>;

    /// to send [&str] as response to client
   fn send_str(&mut self,data:&'static str)
    -> impl Future<Output=Result<(),()>>;


    #[cfg(feature = "use_io_uring")]
   /// to send files as response to client ,
   ///and it takes [FileRSender]
   fn send_file(&mut self,path:FileRSender<'_>)->
                                                     impl Future<
                                                         Output = SendingFileResults> ;
    #[cfg(not(feature = "use_io_uring"))]
    /// to send files as response to client ,
    ///and it takes [FileRSender]
    fn send_file(&mut self,path:FileRSender<'_>)->
    impl Future<
        Output = SendingFileResults > ;


    /// for flushing response stream into route connection
    async fn flush(&mut self)->Result<(),()>;

    #[cfg(not(feature = "use_tokio_send"))]
    /// for writing custom bytes to the stream
    fn write_custom_bytes(&mut self,bytes:&[u8])->
     impl Future<
     Output = Result<(),WaterErrors<'_>>> ;
    #[cfg(feature = "use_tokio_send")]
    /// for writing custom bytes to the stream
    fn write_custom_bytes(&mut self,bytes:&[u8])->
     impl Future<
     Output = Result<(),WaterErrors<'_>>> + Send;

    fn extend_write_buffer(&mut self,bytes:&[u8]);
}



#[cfg(not(any(feature = "use_only_http1", feature = "use_io_uring")))]
/// Http2 Sender for providing [HttpSenderTrait] implementations for http context that using http 2 protocol to serve connections
#[doc(hidden)]
pub struct  Http2Sender<'a,'b,const H:usize,const Q:usize> {
    context:&'a mut Http2Context<'b>,
    send_stream: Option<SendStream<Bytes>>,
    response_builder:Option<H2ResponseBuilder>,
    context_accessories: &'a mut ContextAccessories<H,Q>
}
#[cfg(not(any(feature = "use_only_http1", feature = "use_io_uring")))]

impl <'a,'b,const H:usize,const Q:usize>  Http2Sender<'a,'b,H,Q>{
    pub (crate) fn new(
        context:&'a mut Http2Context<'b>,
        context_accessories: &'a mut ContextAccessories<H,Q>
    )->Http2Sender<'a,'b,H,Q> {
        Http2Sender {
            context,
            send_stream: None,
            response_builder:None,
            context_accessories
        }
    }

    fn handle_content_type_while_sending_file(&mut self,file_content_type:&Option<&str>,file_name:&OsStr,content_disposition:Option<&String>){
        match file_content_type {
            None => {
                self.set_header_ef("Content-Type","Application/octet-stream");
                self.set_header_ef("Content-Disposition",format!("attachment; filename=\"{}\"",file_name.to_str().unwrap_or("")));
            }
            Some(content_type) => {
                self.set_header("Content-Type",content_type);
                if let Some(cd) = content_disposition {
                    self.set_header_ef("Content-Disposition",format!("{cd}; filename=\"{}\"",file_name.to_str().unwrap_or("")));
                }
            }
        }
    }


    async fn write_headers_and_get_ready(&mut self) -> Result<(), ()> {
        if self.send_stream.is_none() {
            if let Some(response_builder) = self.response_builder.take() {
                if let Ok( bb ) = response_builder.body(()) {
                    let   sender = &mut self.context.request_batch.1;
                    if let Ok(stream) = sender.send_response(bb,false) {
                        self.send_stream = Some(stream);
                        return Ok(())
                    }
                }
            }
        }else {return  Ok(())}
         Err(())
    }
}
#[cfg(not(any(feature = "use_only_http1", feature = "use_io_uring")))]
impl<'a,'b,const H:usize,const Q:usize>  HttpSenderTrait for Http2Sender<'a,'b,H,Q> {
    fn send_status_code(&mut self, http_status: StatusCode) {
        if self.response_builder.is_some() {return;}
        let   response = H2Response::
            builder().
            status(http_status.status.get());
        self.response_builder = Some(response);
    }

    fn extend_write_buffer(&mut self,bytes:&[u8]){
        if let Some(send_stream) = &mut self.send_stream {
            _= send_stream.send_data(Bytes::copy_from_slice(bytes),false);
        }
    }
    fn send_data_partial(&mut self, data: ResponseData) {
        let data = data.as_bytes().to_vec();
        if let Some(ref mut stream) = self.send_stream {
            _=stream.send_data(Bytes::from(data),false);
            return;
        } else if let Some( response_builder) = self.response_builder.take() {
            let   sender = &mut self.context.request_batch.1;
            if let Ok(mut stream) = sender.send_response(response_builder.body(()).unwrap(),false) {
                _=stream.send_data(Bytes::from(data),false);
            }
        }
    }

    async fn send_data_as_final_response(&mut self, data: ResponseData<'_>)->Result<(),()> {

         if let Some(ref mut stream) = self.send_stream {
            let data = data.as_bytes().to_vec();
            _=stream.send_data(Bytes::from(data),true);
             Ok(())
        } else if let Some( response_builder) = self.response_builder.take() {
            let data = data.as_bytes().to_vec();
            let   sender = &mut self.context.request_batch.1;
            if let Ok( bb ) = response_builder.body(()) {
                if let Ok(mut stream) = sender.send_response(bb,false) {
                    _=stream.send_data(Bytes::from(data),true);
                    return Ok(())
                }
            }

             Err(())
        } else {
            self.send_status_code(StatusCode::OK);
            if let Some( response_builder) = self.response_builder.take() {
                let data = data.as_bytes().to_vec();
                let   sender = &mut self.context.request_batch.1;
                if let Ok( bb ) = response_builder.body(()) {
                    if let Ok(mut stream) = sender.send_response(bb,false) {
                        _=stream.send_data(Bytes::from(data),true);
                        return Ok(())
                    }
                }
            }
             Err(())
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

    fn set_header_ef<'h,K: Into<WaterBytes<'h>>, V: Into<WaterBytes<'h>>>(&mut self, key: K, value: V) {
        let is_status_written = self.response_builder.is_some();
        if !is_status_written {return;}
        let res = self.response_builder.as_mut();
        if let Some(res) = res {

            let mut ito = itoa::Buffer::new();


            let key =key.into();
            let value =value.into();
            if let Some(headers ) = res.headers_mut() {
                headers.insert(HeaderName::from_bytes(
                   if let WaterBytes::Usize(n) = key {
                       ito.format(n).as_bytes()
                    } else { key.as_bytes() }
                ).unwrap(),HeaderValue::from_bytes(
                    if let WaterBytes::Usize(n) = value {
                        ito.format(n).as_bytes()
                    } else { value.as_bytes() }
                ).unwrap());
            }
        } else {
            self.send_status_code(StatusCode::OK);
            self.set_header_ef(key,value);
        }
    }

    async fn send_json<JSON: Serialize>(&mut self, value: &JSON)->serde_json::Result<()>{
        self.set_header_ef("content-type","application/json");
         match serde_json::to_vec(&value) {
            Ok(data) => {
                _=self.send_data_as_final_response(ResponseData::Slice(
                    data.as_ref()
                )).await;
                Ok(())
            }
            Err(e) => {  Err(e)}
        }

    }

    async fn send_str(&mut self, data: &'static str)->Result<(),()> {
        self.send_data_as_final_response(ResponseData::Str(data)).await
    }

    #[cfg(not(feature = "use_io_uring"))]
    async fn send_file(&mut self,mut pc: FileRSender<'_>)-> SendingFileResults {


        #[cfg(not(feature = "use_io_uring"))]
        // preparing file
        let mut file = match tokio::fs::File::open(pc.path).await {
            Ok(f) => {f}
            Err(_) => {return SendingFileResults::ErrorWhileOpeningTheFile}
        };

        #[cfg(feature = "use_io_uring")]
            let mut file = match tokio_uring::fs::File::open(pc.path).await {
            Ok(f) => {f}
            Err(_) => {return SendingFileResults::ErrorWhileOpeningTheFile}
        };
        let meta = match file.metadata().await {
            Ok(m) => {m}
            Err(_) => { return SendingFileResults::ErrorWhileOpeningTheFile}
        };
        let  file_size  = meta.len() as usize;
        let file_name = match pc.path.file_name() {
            None => { return SendingFileResults::FileNotFound}
            Some(f) => {f}
        };
        let file_content_type = crate::util::content_type_from_file_path(&pc.path);
        let mut start = 0_usize;
        let mut end = file_size;

        // check if we need to send the whole file or just range of it
        match pc.range {
            None => {
                self.send_status_code(HttpStatusCode::OK);
            }
            Some(ranges) => {
                start = ranges.0.unwrap_or(0);
                end = ranges.1.unwrap_or({
                    (start + pc.buffer_size_for_reading_from_file_and_writing_to_stream)
                        .min(file_size)
                });
                if (end - start)  == file_size {
                    self.send_status_code(HttpStatusCode::OK);
                } else {
                    self.send_status_code(HttpStatusCode::PARTIAL_CONTENT);
                }
            }
        }

        self.handle_content_type_while_sending_file(&file_content_type,file_name,pc.content_disposition.as_ref());
        if end >= file_size { end = file_size;}
        if end < start { end = (start + pc.buffer_size_for_reading_from_file_and_writing_to_stream).min(file_size)}
        if  start == end || start > end || end > file_size {
            return SendingFileResults::RangesNotSatisfied
        }
        let mut to_send = end - start  ;
        if to_send != file_size {
            self.set_header("Content-Range",format!("bytes {start}-{}/{}",{end-1},file_size));
        }
        self.set_header("Content-Length",to_send);
        if let Some(d) = pc.content_disposition.as_ref() {
            self.set_header_ef("Content-Disposition",d);
        }
        if file.seek(SeekFrom::Start(start as u64)).await.is_err() {return SendingFileResults::RangesNotSatisfied}
        let mut buffer = Vec::with_capacity(
            WRITING_FILES_BUF_LEN.min(
                to_send
            )
        );

        if self.write_headers_and_get_ready().await.is_err() {
            return SendingFileResults::ErrorWhileSendingBytesToClient;
        }
        while to_send > 0 {
            buffer.clear();
            match file.read_buf(&mut buffer).await {
                Ok(size) => {
                    let index = to_send.min(size);
                    if let Some(ref mut callback) = pc.edit_each_chunk {
                        callback(&mut buffer[..index]);
                    }
                    if self.write_custom_bytes(&buffer[..index]).await.is_err() {
                        return SendingFileResults::ErrorWhileSendingBytesToClient
                    }
                    to_send -= index;
                    continue;
                }
                Err(_) => {
                    return  SendingFileResults::ReadingFileBytesError
                }
            }

        }
         SendingFileResults::Success
    }


    // #[cfg(feature = "use_io_uring")]
    // async fn send_file(&mut self, mut pc: FileRSender<'_>) -> SendingFileResults {
    //     // Open the file using tokio_uring's specialized File type
    //     let file = match tokio_uring::fs::File::open(pc.path).await {
    //         Ok(f) => f,
    //         Err(_) => return SendingFileResults::ErrorWhileOpeningTheFile,
    //     };
    //
    //     // Since metadata() on tokio_uring::fs::File might be different or unavailable depending on versions,
    //     // ensure your setup handles standard metadata conversion or use standard fs for stateless size checks:
    //     let meta = match std::fs::metadata(pc.path) {
    //         Ok(m) => m,
    //         Err(_) => return SendingFileResults::ErrorWhileOpeningTheFile,
    //     };
    //
    //     let file_size = meta.len() as usize;
    //     let file_name = match pc.path.file_name() {
    //         None => return SendingFileResults::FileNotFound,
    //         Some(f) => f,
    //     };
    //
    //     let file_content_type = crate::util::content_type_from_file_path(&pc.path);
    //     let mut start = 0_usize;
    //     let mut end = file_size;
    //
    //     // Check ranges
    //     match pc.range {
    //         None => {
    //             self.send_status_code(HttpStatusCode::OK);
    //         }
    //         Some(ranges) => {
    //             start = ranges.0.unwrap_or(0);
    //             end = ranges.1.unwrap_or({
    //                 (start + pc.buffer_size_for_reading_from_file_and_writing_to_stream)
    //                     .min(file_size)
    //             });
    //             if (end - start) == file_size {
    //                 self.send_status_code(HttpStatusCode::OK);
    //             } else {
    //                 self.send_status_code(HttpStatusCode::PARTIAL_CONTENT);
    //             }
    //         }
    //     }
    //
    //     self.handle_content_type_while_sending_file(&file_content_type, file_name, pc.content_disposition.as_ref());
    //
    //     if end >= file_size { end = file_size; }
    //     if end < start { end = (start + pc.buffer_size_for_reading_from_file_and_writing_to_stream).min(file_size) }
    //     if start == end || start > end || end > file_size {
    //         return SendingFileResults::RangesNotSatisfied;
    //     }
    //
    //     let mut to_send = end - start;
    //     if to_send != file_size {
    //         self.set_header("Content-Range", format!("bytes {start}-{}/{}", {end - 1}, file_size));
    //     }
    //     self.set_header("Content-Length", to_send);
    //
    //     if self.write_headers_and_get_ready().await.is_err() {
    //         return SendingFileResults::ErrorWhileSendingBytesToClient;
    //     }
    //
    //     // Initialize our tracking file offset instead of using .seek()
    //     let mut current_offset = start as u64;
    //
    //     // Allocate the chunk buffer vector explicitly for tokio_uring
    //     let buf_capacity = WRITING_FILES_BUF_LEN.min(to_send);
    //     let mut buffer = vec![0u8; buf_capacity];
    //
    //     while to_send > 0 {
    //         let chunk_size = to_send.min(buffer.capacity());
    //
    //         // Resize buffer to represent the chunk slice target size safely
    //         if buffer.len() != chunk_size {
    //             buffer.resize(chunk_size, 0);
    //         }
    //
    //         // tokio_uring uses positional ownership reads.
    //         // .read_at() consumes the buffer vector and returns it in the tuple.
    //         let (res, returned_buf) = file.read_at(buffer, current_offset).await;
    //         buffer = returned_buf; // Re-claim the buffer ownership
    //
    //         match res {
    //             Ok(0) => break, // EOF reached prematurely
    //             Ok(size) => {
    //                 let index = to_send.min(size);
    //
    //                 if let Some(ref mut callback) = pc.edit_each_chunk {
    //                     callback(&mut buffer[..index]);
    //                 }
    //
    //                 if self.write_custom_bytes(&buffer[..index]).await.is_err() {
    //                     return SendingFileResults::ErrorWhileSendingBytesToClient;
    //                 }
    //
    //                 current_offset += index as u64;
    //                 to_send -= index;
    //             }
    //             Err(_) => {
    //                 return SendingFileResults::ReadingFileBytesError;
    //             }
    //         }
    //     }
    //
    //     SendingFileResults::Success
    // }



    async fn flush(&mut self) -> Result<(), ()> {
        if self.send_stream.is_none() {
            if let Some(response_builder) = self.response_builder.take() {
                if let Ok( bb ) = response_builder.body(()) {
                    let   sender = &mut self.context.request_batch.1;
                    if let Ok(stream) = sender.send_response(bb,true) {
                        self.send_stream = Some(stream);
                        return Ok(())
                    }
                }
            }
        }
         Err(())
    }

    async fn write_custom_bytes(&mut self, bytes: &[u8]) -> Result<(), WaterErrors<'_>> {
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
pub  enum HttpSender<'a,'context,const HEADERS_COUNT:usize,const QUERY_COUNT:usize> {
    H1(Http1Sender<'a,'context,HEADERS_COUNT,QUERY_COUNT>),
    #[cfg(not(any(feature = "use_only_http1", feature = "use_io_uring")))]
    H2(Http2Sender<'a,'context,HEADERS_COUNT,QUERY_COUNT>),
}

 impl<'a,'context,const HEADERS_COUNT:usize,const QUERY_COUNT:usize> HttpSenderTrait
 for HttpSender<'a,'context,HEADERS_COUNT,QUERY_COUNT>
 {
    #[inline(always)]
    fn send_status_code(&mut self, http_status: StatusCode) {
        match self {
            HttpSender::H1(h1) => {
                if h1.is_status_written {return;}
                if let Some((checker,f)) = h1.context_accessories.initial_interceptor.take() {
                    if checker.can_apply(&http_status) {
                        h1.send_status_code(http_status);
                        f(self);
                        return;
                    }
                }
                h1.send_status_code(http_status);

            }
            #[cfg(not(any(feature = "use_only_http1", feature = "use_io_uring")))]
            HttpSender::H2(h2) => {

                if let Some((checker,f)) = h2.context_accessories.initial_interceptor.take() {
                    if checker.can_apply(&http_status) {
                        h2.send_status_code(http_status);
                        f(self);
                        return;
                    }
                }
                h2.send_status_code(http_status);
            }
        }
    }
     #[inline(always)]
    fn send_data_partial(&mut self, data: ResponseData) {
        match self {
            HttpSender::H1(h1) => {
                h1.send_data_partial(data)
            }
            #[cfg(not(any(feature = "use_only_http1", feature = "use_io_uring")))]
            HttpSender::H2(h2) => {
                h2.send_data_partial(data)
            }
        }
    }
     #[inline(always)]
   async fn send_data_as_final_response(&mut self, data: ResponseData<'_>)->Result<(),()> {
        match self {
            HttpSender::H1(h1) => {
                h1.send_data_as_final_response(data).await
            }
            #[cfg(not(any(feature = "use_only_http1", feature = "use_io_uring")))]
            HttpSender::H2(h2) => {
                h2.send_data_as_final_response(data).await
            }
        }
    }
     #[inline(always)]
    fn set_header<K: Display, V: Display>(&mut self, key: K, value: V) {
        match self {
            HttpSender::H1(h1) => {
                h1.set_header(key,value)
            }
            #[cfg(not(any(feature = "use_only_http1", feature = "use_io_uring")))]
            HttpSender::H2(h2) => {
                h2.set_header(key,value)
            }
        }
    }
     #[inline(always)]
     fn set_header_ef<'h,K: Into<WaterBytes<'h>>, V: Into<WaterBytes<'h>>>(&mut self, key: K, value: V) {
            match self {
                HttpSender::H1(h1) => {
                    h1.set_header_ef(key,value)
                }
                #[cfg(not(any(feature = "use_only_http1", feature = "use_io_uring")))]
                HttpSender::H2(h2) => {
                    h2.set_header_ef(key,value)
                }
            }
     }
     #[inline(always)]
     async fn send_json<JSON: Serialize>(&mut self, value: &JSON)->serde_json::Result<()>{
         match self {
             HttpSender::H1(h1) => {h1.send_json(value).await}
             #[cfg(not(any(feature = "use_only_http1", feature = "use_io_uring")))]
             HttpSender::H2(h2) => {h2.send_json(value).await}
         }
     }

     #[inline(always)]
     async fn send_str(&mut self, data: &'static str)->Result<(),()> {
        match self {
            HttpSender::H1(h1) => {
                h1.send_str(data).await
            }
            #[cfg(not(any(feature = "use_only_http1", feature = "use_io_uring")))]
            HttpSender::H2(h2) => {
                h2.send_str(data).await
            }
        }
    }


    async fn send_file(&mut self, pc: FileRSender<'_>) ->SendingFileResults {
        match self {
            HttpSender::H1(h1) => {h1.send_file(pc).await}
            #[cfg(not(any(feature = "use_only_http1", feature = "use_io_uring")))]
            HttpSender::H2(h2) => {h2.send_file(pc).await}
        }
    }
     #[inline(always)]
     async fn flush(&mut self) -> Result<(), ()> {
         match self {
             HttpSender::H1(h1) => {h1.flush().await}
             #[cfg(not(any(feature = "use_only_http1", feature = "use_io_uring")))]
             HttpSender::H2(h2) => {h2.flush().await}
         }
     }
     #[inline(always)]
     async fn write_custom_bytes(&mut self, bytes: &[u8]) -> Result<(), WaterErrors<'_>> {
         match self {
             HttpSender::H1(h1) => {h1.write_custom_bytes(bytes).await}
             #[cfg(not(any(feature = "use_only_http1", feature = "use_io_uring")))]
             HttpSender::H2(h2) => {h2.write_custom_bytes(bytes).await}
         }
     }
     #[inline(always)]
     fn extend_write_buffer(&mut self, bytes: &[u8]) {
         match self {
             HttpSender::H1(h1) => {h1.extend_write_buffer(bytes)}
             #[cfg(not(any(feature = "use_only_http1", feature = "use_io_uring")))]
             HttpSender::H2(h2) => {h2.extend_write_buffer(bytes)}
         }
     }
 }


/// Http2 Sender for providing [HttpSenderTrait] implementations for http context that using http 1 protocol to serve connections

#[doc(hidden)]
pub  struct Http1Sender<'a,'context,const HEADERS_COUNT:usize,const QUERY_COUNT:usize
> {
    pub context:&'a mut Http1Context<'context,HEADERS_COUNT,QUERY_COUNT>,
    is_status_written:bool,
    pub (crate) context_accessories:&'a mut ContextAccessories<HEADERS_COUNT,QUERY_COUNT>
}

impl <'a,'context,const HEADERS_COUNT:usize,const QUERY_COUNT:usize> Http1Sender<'a,'context,
  HEADERS_COUNT,QUERY_COUNT
> {
    pub (crate) fn new(
       context: &'a mut Http1Context<'context,HEADERS_COUNT,QUERY_COUNT>,
       context_accessories:&'a mut ContextAccessories<HEADERS_COUNT,QUERY_COUNT>
    )->Http1Sender<'a,'context,HEADERS_COUNT,QUERY_COUNT>{
        Http1Sender {
            context,
            is_status_written:false,
            context_accessories
        }
    }


    pub (crate) async fn write_bytes(&mut self,bytes:&[u8])->Result<(),()>{
        #[cfg(feature = "use_io_uring")]
        {
            match self.context.stream {
                #[cfg(all(feature = "support_tls",not(feature="use_io_uring")))]
                HttpStream::AsyncSecure(_) => {
                    todo!()
                }
                HttpStream::Async(s) => {
                    let bs = water_buffer::helper::BytesSliceWrapper::new(bytes);
                    let (r,_) = s.write_all(bs).await;
                    if r.is_err() {return  Err(())}
                    return  Ok(())
                }
            }
        }
        #[cfg(not(feature = "use_io_uring"))]
        {
            match self.context.stream.write_all(bytes).await {
                Ok(_) => {Ok(())}
                Err(_) => {Err(())}
            }
        }
    }
    #[inline(always)]
    fn handle_content_type_while_sending_file(&mut self,file_content_type:&Option<&str>,file_name:&OsStr,content_disposition:Option<&String>){
        match content_disposition {
            None => {
                match file_content_type {
                    None => {
                        self.set_header_ef("Content-Type","Application/octet-stream");
                        self.set_header_ef("Content-Disposition",format!("attachment; filename=\"{}\"",file_name.to_str().unwrap_or("")));
                    }
                    Some(content_type) => {
                        self.set_header("Content-Type",content_type);
                        self.set_header_ef("Content-Disposition",format!("attachment; filename=\"{}\"",file_name.to_str().unwrap_or("")));
                    }
                }
            }
            Some(d) => {
                self.set_header_ef("Content-Disposition",d);

            }
        }
    }
}
impl<'a,'context,const HEADERS_COUNT:usize,const QUERY_COUNT:usize> HttpSenderTrait for
Http1Sender <'a,'context,HEADERS_COUNT,QUERY_COUNT>  {

    #[inline(always)]
    fn extend_write_buffer(&mut self,bytes:&[u8]){
        self.context.response_buffer.extend_from_slice(bytes);
    }
    #[inline(always)]
    fn send_status_code(&mut self, http_status: StatusCode) {
        if self.is_status_written {return}
        let buf = &mut self.context.response_buffer;
        buf.extend_from_slice(b"HTTP/1.1 ");
        // let mut iota_buffer = itoa::Buffer::new();
        // let num_str = iota_buffer.format(http_status.status.get());
        let v = http_status.status.get();
        #[cfg(not(feature = "use_io_uring"))]
        {
            v.put_into(*buf);
        }
        #[cfg(feature = "use_io_uring")]
        {
            v.put_into(buf) ;
        }
        buf.extend_from_slice(b" ");
        buf.extend_from_slice(http_status.label.as_bytes());
        buf.extend_from_slice(b"\r\n");
        self.is_status_written = true;
    }

    #[inline]
    fn send_data_partial(&mut self, data: ResponseData) {
        let bytes = data.as_bytes();
        self.context.response_buffer.extend_from_slice(b"\r\n");
        self.context.response_buffer.extend_from_slice(bytes);
    }

    #[inline]
    async fn send_data_as_final_response(&mut self, data: ResponseData<'_>) -> Result<(),()> {
        #[cfg(feature = "auto_encode_response")]
        let ref en_configurations = get_server_config().responding_encoding_configurations;

        let data = data.as_bytes();
        let len = data.len();

        #[cfg(feature = "auto_encode_response")]
        if en_configurations.is_not_none() && data.len() >= en_configurations.threshold_for_encoding_response  {
            let accept_encoding = self.context
                .request.headers().get_as_str("Accept-Encoding");
            if let Some(accept_encoding ) = accept_encoding {
                let encoder = en_configurations.encode(
                    accept_encoding.into(),
                    data
                ).await;
                if let Some(encoder )  = encoder {
                    self.set_header_ef("Content-Encoding",encoder.logic);
                    let data = encoder.data;
                    self.context.response_buffer.extend_from_slice(format!("Content-Length: {}\r\n\r\n",data.len()).as_bytes());
                    self.context.response_buffer.extend_from_slice(data.as_ref());
                    return Ok(())
                }
            }

        }
        self.context.response_buffer.extend_from_slice(b"Content-Length: ");
        #[cfg(feature = "use_io_uring")]
        {
            let  buf = &mut self.context.response_buffer;
            len.put_into(buf);
        }
        #[cfg(not(feature = "use_io_uring"))]
        {
            len.put_into(self.context.response_buffer);
        }
        self.context.response_buffer.extend_from_slice(b"\r\n\r\n");
        self.context.response_buffer.extend_from_slice(data);
        Ok(())
    }
    #[inline(always)]
    fn set_header<K:Display, V:Display>(&mut self, key: K, value: V) {
        if !self.is_status_written { self.send_status_code(StatusCode::OK);}
        self.context.response_buffer.extend_from_slice(format!("{key}: {value}\r\n").as_bytes());
    }
    #[inline(always)]
    fn set_header_ef<'h,K: Into<WaterBytes<'h>>, V: Into<WaterBytes<'h>>>(&mut self, key: K, value: V) {
        if !self.is_status_written { self.send_status_code(StatusCode::OK);}
        let key_bytes = key.into();
        let value_bytes = value.into();

        if let WaterBytes::Usize(u) = key_bytes {
            #[cfg(not(feature = "use_io_uring"))]
            u.put_into(self.context.response_buffer);
            #[cfg(feature = "use_io_uring")]
            {
                u.put_into(&mut self.context.response_buffer);
            }
        } else {
            self.context.response_buffer.extend_from_slice(key_bytes.as_bytes());
        }
        self.context.response_buffer.extend_from_slice(b": ");
        if let WaterBytes::Usize(v) = value_bytes {
            #[cfg(not(feature = "use_io_uring"))]
            v.put_into(self.context.response_buffer);
            #[cfg(feature = "use_io_uring")]
            v.put_into(&mut self.context.response_buffer);
        } else {
            self.context.response_buffer.extend_from_slice(value_bytes.as_bytes());
        }
        self.context.response_buffer.extend_from_slice(b"\r\n");
    }
    #[inline(always)]
    async fn send_json<JSON: Serialize>(&mut self, value: &JSON)->serde_json::Result<()> {
        self.set_header_ef("content-type","application/json");
        match serde_json::to_vec(value) {
            Ok(data) => {
                if self.send_data_as_final_response(ResponseData::Slice(data.as_ref())).await.is_ok() {
                    return Ok(())
                }
                Err(serde_json::Error::custom("fail"))
            }
            Err(e) => {return Err(e)}
        }

    }
    #[inline(always)]
    async fn send_str(&mut self,data: &'static str) -> Result<(),()> {
        self.send_status_code(StatusCode::OK);
        self.send_data_as_final_response(ResponseData::Str(data)).await
    }

    #[inline(always)]
    #[cfg(not(feature = "use_io_uring"))]
    async fn send_file(&mut self,mut pc: FileRSender<'_>)-> SendingFileResults {


        #[cfg(not(feature = "use_io_uring"))]
            // preparing file
            let mut file = match tokio::fs::File::open(pc.path).await {
            Ok(f) => {f}
            Err(_) => {return SendingFileResults::ErrorWhileOpeningTheFile}
        };

        #[cfg(feature = "use_io_uring")]
            let mut file = match tokio_uring::fs::File::open(pc.path).await {
            Ok(f) => {f}
            Err(_) => {return SendingFileResults::ErrorWhileOpeningTheFile}
        };
        let meta = match file.metadata().await {
            Ok(m) => {m}
            Err(_) => { return SendingFileResults::ErrorWhileOpeningTheFile}
        };
        let  file_size   = meta.len() as usize;

        let file_name = match pc.path.file_name() {
            None => { return SendingFileResults::FileNotFound}
            Some(f) => {f}
        };
        let file_content_type = crate::util::content_type_from_file_path(&pc.path);
        let mut start = 0_usize;
        let mut end = file_size;

        // check if we need to send the whole file or just range of it
        match pc.range {
            None => {
                self.send_status_code(HttpStatusCode::OK);
            }
            Some(ranges) => {
                start = ranges.0.unwrap_or(0);
                end = ranges.1.unwrap_or({
                    (start + pc.buffer_size_for_reading_from_file_and_writing_to_stream)
                        .min(file_size)
                });
                if (end - start)  == file_size {
                    self.send_status_code(HttpStatusCode::OK);
                } else {
                    self.send_status_code(HttpStatusCode::PARTIAL_CONTENT);
                }
            }
        }

        self.handle_content_type_while_sending_file(&file_content_type,file_name,pc.content_disposition.as_ref());


        if end >= file_size { end = file_size;}
        if end < start { end = (start + pc.buffer_size_for_reading_from_file_and_writing_to_stream).min(file_size)}

        if  start == end || start > end || end > file_size {
            return SendingFileResults::RangesNotSatisfied
        }
        let mut to_send = end - start  ;

        if to_send != file_size {
            self.set_header("Content-Range",format!("bytes {start}-{}/{}",{end-1},file_size));
        }

        self.set_header("Content-Length",to_send);

        if file.seek(SeekFrom::Start(start as u64)).await.is_err() {return SendingFileResults::RangesNotSatisfied}
        let mut buffer = Vec::with_capacity(
            WRITING_FILES_BUF_LEN.min(
                to_send
            )
        );
        if self.flush().await.is_err() || self.write_bytes(b"\r\n").await.is_err() { return  SendingFileResults::ErrorWhileSendingBytesToClient}

        while to_send > 0 {
            buffer.clear();
           match file.read_buf(&mut buffer).await {
               Ok(size) => {
                   let index = to_send.min(size);
                   if let Some(ref mut callback) = pc.edit_each_chunk {
                       callback(&mut buffer[..index]);
                   }
                   if self.write_bytes(&buffer[..index]).await.is_err() {
                       return SendingFileResults::ErrorWhileSendingBytesToClient
                   }
                   to_send -= index;
                   continue;
               }
               Err(_) => {
                   return  SendingFileResults::ReadingFileBytesError
               }
           }

        }
         SendingFileResults::Success
    }


    #[inline(always)]
    #[cfg(feature = "use_io_uring")]
    async fn send_file(&mut self, mut pc: FileRSender<'_>) -> SendingFileResults {
        // Open the file using tokio_uring's specialized stateless File type
        let file = match tokio_uring::fs::File::open(pc.path).await {
            Ok(f) => f,
            Err(_) => return SendingFileResults::ErrorWhileOpeningTheFile,
        };

        // Leverage standard fs blocking metadata check safely before entering the hot loop
        let meta = match std::fs::metadata(pc.path) {
            Ok(m) => m,
            Err(_) => return SendingFileResults::ErrorWhileOpeningTheFile,
        };

        let file_size = meta.len() as usize;
        let file_name = match pc.path.file_name() {
            None => return SendingFileResults::FileNotFound,
            Some(f) => f,
        };

        let file_content_type = crate::util::content_type_from_file_path(&pc.path);
        let mut start = 0_usize;
        let mut end = file_size;

        // Evaluate if range processing or a full response is expected
        match pc.range {
            None => {
                self.send_status_code(HttpStatusCode::OK);
            }
            Some(ranges) => {
                start = ranges.0.unwrap_or(0);
                end = ranges.1.unwrap_or({
                    (start + pc.buffer_size_for_reading_from_file_and_writing_to_stream)
                        .min(file_size)
                });
                if (end - start) == file_size {
                    self.send_status_code(HttpStatusCode::OK);
                } else {
                    self.send_status_code(HttpStatusCode::PARTIAL_CONTENT);
                }
            }
        }

        self.handle_content_type_while_sending_file(&file_content_type, file_name, pc.content_disposition.as_ref());

        if end >= file_size { end = file_size; }
        if end < start { end = (start + pc.buffer_size_for_reading_from_file_and_writing_to_stream).min(file_size) }

        if start == end || start > end || end > file_size {
            return SendingFileResults::RangesNotSatisfied;
        }

        let mut to_send = end - start;

        if to_send != file_size {
            self.set_header("Content-Range", format!("bytes {start}-{}/{}", {end - 1}, file_size));
        }

        self.set_header("Content-Length", to_send);

        // Flush HTTP headers to the connection buffer and write the protocol delimiter
        if self.flush().await.is_err() || self.write_bytes(b"\r\n").await.is_err() {
            return SendingFileResults::ErrorWhileSendingBytesToClient;
        }

        // Initialize positional tracker offset since io_uring is stateless
        let mut current_offset = start as u64;

        // Allocate block capacity for ownership transferring reads
        let buf_capacity = WRITING_FILES_BUF_LEN.min(to_send);
        let mut buffer = vec![0u8; buf_capacity];

        while to_send > 0 {
            let chunk_size = to_send.min(buffer.capacity());

            if buffer.len() != chunk_size {
                buffer.resize(chunk_size, 0);
            }

            // tokio_uring handles structural completion memory allocation via tuple ownership
            let (res, returned_buf) = file.read_at(buffer, current_offset).await;
            buffer = returned_buf; // Regain vector ownership safely

            match res {
                Ok(0) => break, // Premature End Of File encountered
                Ok(size) => {
                    let index = to_send.min(size);

                    if let Some(ref mut callback) = pc.edit_each_chunk {
                        callback(&mut buffer[..index]);
                    }

                    // Push chunk out through Http1Sender's low-level stream handler
                    if self.write_bytes(&buffer[..index]).await.is_err() {
                        return SendingFileResults::ErrorWhileSendingBytesToClient;
                    }

                    current_offset += index as u64;
                    to_send -= index;
                }
                Err(_) => {
                    return SendingFileResults::ReadingFileBytesError;
                }
            }
        }

        SendingFileResults::Success
    }
    #[cfg(feature = "use_io_uring")]
    #[inline(always)]
    async fn flush(&mut self) -> Result<(),()>{
        if handle_responding(unsafe{self.context.response_buffer.unsafe_clone()},self.context.stream).await.is_err() {
            return Err(())
        }
        Ok(())
    }

    #[inline(always)]
    #[cfg(not(feature = "use_io_uring"))]
     async fn flush(&mut self) -> Result<(),()>{
         if handle_responding(self.context.response_buffer,self.context.stream).await.is_err() {
             return Err(())
         }
         Ok(())
    }

    #[inline(always)]
    #[cfg(feature = "use_io_uring")]
    async fn write_custom_bytes(&mut self, bytes: &[u8]) -> Result<(), WaterErrors<'_>> {
        self.context.response_buffer.extend_from_slice(bytes);
        if handle_responding(unsafe{self.context.response_buffer.unsafe_clone()},self.context.stream).await.is_err() {
            return Err(WaterErrors::Server(ServerError::WRITING_TO_STREAM_ERROR))
        }
        Ok(())
    }
    #[cfg(not(feature = "use_io_uring"))]
    #[inline(always)]
    async fn write_custom_bytes(&mut self, bytes: &[u8]) -> Result<(), WaterErrors<'_>> {
        self.context.response_buffer.extend_from_slice(bytes);
        if handle_responding(self.context.response_buffer,self.context.stream).await.is_err() {
            return Err(WaterErrors::Server(ServerError::WRITING_TO_STREAM_ERROR))
        }
        Ok(())
    }
}



/// defining sending file behavior
#[derive(Debug)]
pub enum SendingFileResults {
    /// if file path is not real file
    FileNotFound,
    /// error while reading file data
    ReadingFileBytesError,
    /// error while opening an existing file
    ErrorWhileOpeningTheFile,
    /// error while sending file data bytes to the client
    ErrorWhileSendingBytesToClient,
    /// when incoming request is not correct
    RangesNotSatisfied,
    /// when sending file successfully to the client
    Success
}

impl SendingFileResults {

    /// when sending file completed successfully
    pub fn is_success(&self)->bool{
        if let SendingFileResults::Success = self { return true}
        false
    }
}

/// for constructing efficient bytes injector
pub enum WaterBytes<'a> {
    Str(&'static str),
    StringSlice(&'a str),
    Slice(&'a [u8]),
    String(String),
    Usize(usize),
    Vec(Vec<u8>),
    None
}

impl<'a> WaterBytes<'a> {
    /// convert any type of supported data to &[u8]
    pub fn as_bytes(&self)->&'_ [u8] {

        match self {
            WaterBytes::Str(s) => {s.as_bytes()}
            WaterBytes::Slice(s) => {s}
            WaterBytes::String(s) => {(*s).as_bytes()}

            WaterBytes::StringSlice(a) => {(*a).as_bytes()}
            WaterBytes::Vec(r) => {r.as_slice()}
            _=>panic!("can not convert to bytes")
        }
    }


    /// for pushing to buffer
    #[inline(always)]
    pub fn push_to(&self,buf:&mut WaterBuffer<u8>){
        if let WaterBytes::Usize(u) = self {
            u.put_into(buf);
            return
        }
       let bytes =  match self {
            WaterBytes::Str(s) => {s.as_bytes()}
            WaterBytes::Slice(s) => {s}
            WaterBytes::String(s) => {(*s).as_bytes()}
            WaterBytes::StringSlice(a) => {(*a).as_bytes()}
            WaterBytes::Vec(r) => {r.as_slice()}
            _=>panic!("can not convert to bytes")
        };
        buf.extend_from_slice(bytes);
    }
}

impl<'a> Into<WaterBytes<'a>> for &'a [u8] {
    fn into(self) -> WaterBytes<'a> {
        WaterBytes::Slice(self)
    }
}



impl <'a> Into<WaterBytes<'a>> for () {
    fn into(self) -> WaterBytes<'a> {
        WaterBytes::None
    }
}

impl <'a> Into<WaterBytes<'a>> for usize {
    fn into(self) -> WaterBytes<'a> {
        WaterBytes::Usize(self)
    }
}
impl  Into<WaterBytes<'_>> for Vec<u8> {
    fn into(self) -> WaterBytes<'static> {
        WaterBytes::Vec(self)
    }
}


impl<'a> Into<WaterBytes<'a>> for &'a String {
    fn into(self) -> WaterBytes<'a> {
        WaterBytes::StringSlice(self)
    }
}
impl<'a> Into<WaterBytes<'a>> for  String {
    fn into(self) -> WaterBytes<'a> {
        WaterBytes::String(self)
    }
}


impl Into<WaterBytes<'_>> for &'static str {
    fn into(self) -> WaterBytes<'static> {
        WaterBytes::Str(self)
    }

}






