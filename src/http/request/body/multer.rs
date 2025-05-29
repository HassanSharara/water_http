use std::borrow::Cow;
use std::future::Future;
use std::pin::Pin;
use bytes::{Buf};

use twoway::find_bytes;
use crate::http::request::MultiPartFormDataField;
use crate::server::connection::BodyReadingBuffer;
use crate::util::{found_boundary_in, PatternExistResult};

use super::{H1StreamHolder,H2StreamHolder};
use super::FormDataAll;



macro_rules! body_reading_checker {
    ($body_mut:ident,$reading_buffer:expr) => {
        if let Some(data) = $body_mut.data().await {
            match data {
                Ok(data) => {
                    $reading_buffer.extend_from_slice(data.as_ref());
                }
                Err(_) => {
                    return  Err(())
                }
            }
        }
    };
}

pub (crate) enum  MultipartStreamHolder<'a> {
    H1(H1StreamHolder<'a>),
    H2(H2StreamHolder<'a>)
}



/// for handling multipart from data in both protocols http1 and http2
pub struct MultipartData<'a> {
     stream_holder:MultipartStreamHolder<'a>,
     reading_buffer:&'a mut BodyReadingBuffer,
     boundary:Cow<'a,str>,
     content_length:usize,
     // remaining:usize,
 }


pub (crate) type FieldCallBackResult = Result<Option<Pin<Box< dyn Future<Output = Result<(),()>>+Send>>>,()>;
// type FieldCallBackResult = Pin<Box<dyn Future<Output=Result<(), ()>>>>;
 impl <'a>  MultipartData<'a> {

     /// for creating new Multipart parser
     pub (crate) fn new(
     stream_holder:MultipartStreamHolder<'a>,
     reading_buffer:&'a mut BodyReadingBuffer,
     boundary:Cow<'a,str>,
     content_length:usize,
     )->MultipartData<'a>{
         MultipartData { stream_holder,
             reading_buffer,boundary,
             content_length,
         }
     }



    pub async fn on_field_detected(
        &mut self,
        mut callback: impl FnMut(& MultiPartFormDataField, & [u8]) -> FieldCallBackResult
    )->Result<(),()>{
        let mut field:Option<MultiPartFormDataField<'_>> = None;

        match &mut self.stream_holder {
            MultipartStreamHolder::H1(h1) => {
                let left_bytes_len = h1.left_bytes.len();
                if left_bytes_len < self.content_length {
                    self.reading_buffer.bytes_consumed+=left_bytes_len;
                }
                let boundary = self.boundary.as_bytes();
                loop {
                    if h1.left_bytes == b"--\r\n" {
                        return Ok(())
                    }
                    // when left bytes is not empty
                    if h1.left_bytes.is_empty()
                    {

                        let res =   self.read_using_local_buffer(field,callback).await;

                        return  res;
                    }
                    else
                    // when left bytes from incoming request is not empty
                    {
                        match &field {
                            None => {
                                if let Some(f_field) = MultiPartFormDataField::new(h1.left_bytes) {
                                    h1.left_bytes=& h1. left_bytes[f_field.field_header_length..];
                                    // self.remaining -= f_field.field_header_length;

                                    field = Some(f_field);
                                } else {
                                    self.reading_buffer.extend_from_slice( h1.left_bytes);
                                    h1.left_bytes =&[];
                                    continue;
                                }
                            }
                            Some(f_field) => {
                                match found_boundary_in( h1.left_bytes,boundary) {
                                    PatternExistResult::Some(index) => {

                                        if Self::handle_callback(
                                            f_field,
                                            & h1.left_bytes[..index],
                                            &mut callback
                                        ).await.is_err() { return Err(())}

                                        let len = index + boundary.len() + 4;
                                        h1.left_bytes = & h1.left_bytes[len..];
                                        // self.remaining-=len;
                                        field = None;
                                        continue;
                                    }
                                    PatternExistResult::MaybeExistOnLastBytesFromLen(index) => {
                                        let to_send = & h1.left_bytes[..index];
                                        if Self::handle_callback(&f_field,to_send,&mut callback).await
                                            .is_err() { return  Err(())}
                                        // self.remaining-=to_send.len();
                                        self.reading_buffer.clear();
                                        self.reading_buffer.extend_from_slice(& h1.left_bytes[index..]);
                                        h1.left_bytes =&[];
                                        if  self.reading_buffer.read_buf(h1.stream).await.is_err() {
                                            return Err(())
                                        }
                                        continue;
                                    }
                                    PatternExistResult::None => {
                                        if Self::handle_callback(&f_field, h1.left_bytes,&mut callback).await
                                            .is_err() { return Err(())}
                                        // self.remaining-= h1.left_bytes.len().min(self.remaining);
                                        h1.left_bytes=&[];
                                        continue;

                                    }
                                };
                            }
                        };
                    }
                }
            }
            MultipartStreamHolder::H2(_) => {
                self.read_using_local_buffer_for_h2(
                    field,
                    callback
                ).await
            }
        }
    }

    #[inline]
    async fn handle_callback(field:&'_ MultiPartFormDataField<'_>,data:&[u8],
                                  callback:&mut impl FnMut (&'_ MultiPartFormDataField<'_> ,&'_[u8])
                                     ->FieldCallBackResult
    )->Result<(),()>{
        match callback(field,data) {
            Ok(future) => {
                if let Some(future) = future {
                    if future.await.is_err() { return Err(())}
                }
            }
            Err(_) => {
                return Err(())
            }
        };
        Ok(())
    }


     #[inline]
      async fn read_using_local_buffer(
         &mut self,
         mut field:Option<MultiPartFormDataField<'_>>,
         mut callback:impl FnMut (&'_ MultiPartFormDataField<'_> ,&'_[u8])->FieldCallBackResult
     )->Result<(),()>{
          let boundary = self.boundary.as_bytes();
          let boundary_length = boundary.len();
          let mut field_bytes = Vec::<u8>::with_capacity(2500);
          let h1 = match &mut self.stream_holder {
              MultipartStreamHolder::H1(h1) => {h1}
              MultipartStreamHolder::H2(_) => {return  Err(())}
          };
          let mut end_reading_trigger = 0_u8;
          loop { // checking if the data is already close to end

              if self.reading_buffer.bytes_consumed >= self.content_length {
                  if end_reading_trigger >= 8 { return Ok(())}
                  let chunk = self.reading_buffer.chunk();
                  if chunk.len() <= boundary_length+2  {
                      if let Some(index) = find_bytes(chunk,b"--\r\n") {
                          self.reading_buffer.advance(index + 4);
                          return Ok(())
                      }
                  }
                  end_reading_trigger+=1;
              } else if self.reading_buffer.read_buf(h1.stream).await.is_err() {
                  return Err(())
              }
              match &field {
                  None => {
                      let chunk = self.reading_buffer.chunk();
                      if let Some(r_field) = MultiPartFormDataField::new(chunk) {
                          field_bytes.clear();
                          field_bytes.extend_from_slice(&chunk[..r_field.field_header_length]);
                          field = Some(MultiPartFormDataField::new(&field_bytes).unwrap());
                          self.reading_buffer.advance(field_bytes.len());
                          continue;
                      }
                  }
                  Some(r_field) => {
                      let chunk = self.reading_buffer.chunk();

                      match found_boundary_in(chunk,boundary) {
                          PatternExistResult::Some(index) => {
                              let data =&chunk[..index];
                              if Self::handle_callback(r_field,chunk,&mut callback).await
                                  .is_err() { return  Err(())}
                              let consumed = data.len() + boundary_length;
                              // self.remaining-=consumed;
                              self.reading_buffer.advance(consumed);
                              field = None;
                              continue;
                          }
                          PatternExistResult::MaybeExistOnLastBytesFromLen(index) => {
                              let data =&chunk[..index];
                              if Self::handle_callback(r_field,chunk,&mut callback).await
                                  .is_err() { return  Err(())}
                              let consumed = data.len() ;
                              // self.remaining-=consumed;
                              self.reading_buffer.advance(consumed);
                              field = None;
                              continue;
                          }
                          PatternExistResult::None => {
                              if Self::handle_callback(r_field,chunk,&mut callback).await
                                  .is_err() { return  Err(())}
                              let consumed = chunk.len() ;
                              // self.remaining-=consumed;
                              self.reading_buffer.advance(consumed);
                              continue;
                          }
                      }

                  }
              }
          }
     }




    // if self.reading_buffer.bytes_consumed >= self.content_length {
    // if last_read_trigger >= 4 { return Ok(())}
    // let chunk = self.reading_buffer.chunk();
    // if chunk.len() <= boundary_length + 2  {
    // if chunk.ends_with(b"--"){ return Ok(())}
    // }
    // last_read_trigger+=1;
    // } else if self.reading_buffer.read_buf(h1.stream).await.is_err() {
    // return Err(())
    // }

    #[inline]
    async fn read_using_local_buffer_for_h2(
        &mut self,
        mut field:Option<MultiPartFormDataField<'_>>,
        mut callback:impl FnMut (&'_ MultiPartFormDataField<'_> ,&'_[u8])->FieldCallBackResult
    )->Result<(),()>{
        let boundary = self.boundary.as_bytes();
        let boundary_length = boundary.len();
        let mut field_bytes = Vec::<u8>::with_capacity(2500);

        let h2 = match &mut self.stream_holder {
            MultipartStreamHolder::H1(_) => {
                return Err(())}
            MultipartStreamHolder::H2(h2) => {h2}
        };

        let body_mut = h2.batch.body_mut();
        loop {


            if self.reading_buffer.bytes_consumed < boundary_length {
                if self.reading_buffer.chunk().ends_with(b"--\r\n") {
                    return Ok(())
                }
            }

            // checking if the data is already close to end
            if self.reading_buffer.bytes_consumed < self.content_length  {
                body_reading_checker!(body_mut,self.reading_buffer);
            }



            match &field {
                None => {
                    let chunk = self.reading_buffer.chunk();
                    if let Some(r_field) = MultiPartFormDataField::new(chunk) {
                        field_bytes.clear();
                        field_bytes.extend_from_slice(&chunk[..r_field.field_header_length]);
                        // self.remaining -= field_bytes.len();
                        field = Some(MultiPartFormDataField::new(&field_bytes).unwrap());
                        self.reading_buffer.advance(field_bytes.len());
                        continue;
                    }
                }
                Some(r_field) => {
                    let chunk = self.reading_buffer.chunk();

                    match found_boundary_in(chunk,boundary) {
                        PatternExistResult::Some(index) => {
                            let data =&chunk[..index];
                            if Self::handle_callback(r_field,chunk,&mut callback).await
                                .is_err() { return  Err(())}
                            let consumed = data.len() + boundary_length;
                            // self.remaining-=consumed;
                            self.reading_buffer.advance(consumed);
                            field = None;
                            continue;
                        }
                        PatternExistResult::MaybeExistOnLastBytesFromLen(index) => {
                            let data =&chunk[..index];
                            if Self::handle_callback(r_field,chunk,&mut callback).await
                                .is_err() { return  Err(())}
                            let consumed = data.len() ;
                            // self.remaining-=consumed;
                            self.reading_buffer.advance(consumed);
                            field = None;
                            continue;
                        }
                        PatternExistResult::None => {
                            if Self::handle_callback(r_field,chunk,&mut callback).await
                                .is_err() { return  Err(())}
                            let consumed = chunk.len() ;
                            // self.remaining-=consumed;
                            self.reading_buffer.advance(consumed);
                            continue;
                        }
                    }

                }
            }

        }
    }
    /// converting buffer bytes into [FormDataAll]
    /// # Note:
    /// - this action will cause heap memory allocation
    /// so if your data is small and required to be handled like this you can go with that
    /// but the better approach is to use the default struct handler [MultipartData]
    pub async fn take(mut self)->Result<FormDataAll,()>{
        let mut data = FormDataAll::new();

        if (&mut self).on_field_detected(|field,parsed_data|{
            data.push(field,parsed_data);
            return Ok(None)
        }).await.is_err() {
            return  Err(())
        }
        return Ok(data)
    }


    /// a shortcut for [self.take]
    /// # return [FormDataAll]
    pub async fn to_form_data_all(self)->Result<FormDataAll,()> { self.take().await }



 }










