#[cfg(feature = "accept_transfer_chunked")]
use bytes::{Buf, Bytes};
#[cfg(feature = "accept_transfer_chunked")]
use crate::http::request::{FieldCallBackResult, H1StreamHolder, MultipartStreamHolder};
#[cfg(feature = "accept_transfer_chunked")]
use crate::server::connection::BodyReadingBuffer;
#[cfg(feature = "accept_transfer_chunked")]
use crate::util::hex_bytes_to_usize;


#[cfg(feature = "accept_transfer_chunked")]
/// check if reading bytes is going well or not
pub type BodyChunksReadingResult = Result<Bytes,()>;

/// incoming body chunked bytes
#[cfg(feature = "accept_transfer_chunked")]

#[derive(Debug)]
/// for handling multipart from data in both protocols http1 and http2
pub struct BodyChunkedReader<'a> {
    stream_holder:MultipartStreamHolder<'a>,
    reading_buffer:&'a mut BodyReadingBuffer,
    chunk_indexes_count:usize,
    #[cfg(feature = "accept_transfer_chunked")]
    original_left_bytes_length:usize,
    #[cfg(feature = "accept_transfer_chunked")]
    to_advance_bytes: &'a mut Option<usize>
    // remaining:usize,
}


#[derive(Debug)]
pub struct  Chunk {
    /// refer to the index of chunk between incoming chunks
    pub index:usize,
    /// incoming chunk length
    pub chunk_size:usize
}




#[cfg(feature = "accept_transfer_chunked")]

macro_rules! try_call_back {
    ($callback:expr,$chunk:expr,$data:expr) => {
        match $callback($chunk,$data) {

                                   Err(_) => {return Err(())}
                                   Ok(future) => {
                                       if let Some(future) = future {
                                           if future.await.is_err() {return Err(())}
                                       }
                                   }

                               }
    };
}
#[cfg(feature = "accept_transfer_chunked")]

impl<'a> BodyChunkedReader<'a> {



    /// for creating new [BodyChunkedReader]
     pub (crate) fn new(
         stream_holder:MultipartStreamHolder<'a>,
         reading_buffer:&'a mut BodyReadingBuffer,
         #[cfg(feature = "accept_transfer_chunked")]
         to_advance_bytes:&'a mut Option<usize>
     )->BodyChunkedReader<'a>{
         BodyChunkedReader {

             reading_buffer,
             chunk_indexes_count:0,
             #[cfg(feature = "accept_transfer_chunked")]
             original_left_bytes_length:match &stream_holder {
                 MultipartStreamHolder::H1(holder) => { holder.left_bytes.len() }
                 MultipartStreamHolder::H2(_) => {0}
             },
             #[cfg(feature = "accept_transfer_chunked")]
             to_advance_bytes,
             stream_holder,
         }
     }


     /// for polling chunks in order and efficient
     pub async fn on_chunk_detected(&mut self,
        mut callback:impl FnMut(&Chunk,&[u8])->FieldCallBackResult
     )->Result<(),()>{
         let chunk_index = &mut self.chunk_indexes_count;
         match &mut self.stream_holder {
             MultipartStreamHolder::H1(holder) => {
                 let mut chunk:Option<Chunk> = None;
                 loop {

                     match &chunk {
                         None => {
                             return  match find_new_line(holder.left_bytes,16) {
                                 Ok(index_option) => {
                                     #[cfg(feature = "debugging")]
                                     {
                                         tracing::debug!("trying to find chunk on {:?}",index_option)
                                     }
                                      match index_option {
                                         None => {
                                             #[cfg(feature = "debugging")]
                                             {
                                                 tracing::debug!("the left data is {}",String::from_utf8_lossy(holder.left_bytes));
                                             }
                                             self.reading_buffer.extend_from_slice(holder.left_bytes);
                                             holder.left_bytes = &[];
                                             h1_chunk_detecting_on_stream(
                                                 holder,
                                                 self.reading_buffer,
                                                 chunk_index,
                                                 &mut callback,
                                                 chunk
                                             ).await
                                         }
                                         Some(i) => {

                                             let chunk_size = hex_bytes_to_usize(&holder.left_bytes[..i]);
                                             if let Some(chunk_size) = chunk_size {
                                                 #[cfg(feature = "debugging")]
                                                 {
                                                     tracing::debug!("chunk size is {}",chunk_size)
                                                 }
                                                 chunk = Some(Chunk { index: *chunk_index, chunk_size });

                                                 *chunk_index += 1;
                                                 if i + 2 >= holder.left_bytes.len() {
                                                     holder.left_bytes = &[];
                                                     return h1_chunk_detecting_on_stream(
                                                         holder,
                                                         self.reading_buffer,
                                                         chunk_index,
                                                         &mut callback,
                                                         chunk
                                                     ).await;
                                                 }
                                                 holder.left_bytes = &holder.left_bytes[i+2..];
                                                 #[cfg(feature = "debugging")]
                                                 {
                                                     tracing::debug!("data after advanced {}",
                                                       String::from_utf8_lossy(holder.left_bytes)
                                                     )
                                                 }
                                                 continue;
                                             }
                                             Err(())
                                         }
                                     }
                                 }
                                 Err(_) => {  Err(())}
                             }
                         }
                         Some(chunk_op) => {
                             if chunk_op.chunk_size == 0 {
                                 #[cfg(feature = "debugging")]{
                                     println!("the last chunk payload {:?}",
                                       String::from_utf8_lossy(holder.left_bytes)
                                     )
                                 }
                                 return match find_new_line(holder.left_bytes, 4) {
                                     Ok(index_option) => {
                                         match index_option {
                                             None => {

                                                 h1_chunk_detecting_on_stream(
                                                     holder,
                                                     self.reading_buffer,
                                                     chunk_index,
                                                     &mut callback,
                                                     chunk
                                                 ).await
                                             }
                                             Some(i) => {
                                                 #[cfg(feature = "debugging")]{
                                                     println!("the last chunk was found on {i}"
                                                     )
                                                 }
                                                 if holder.left_bytes.len() < 2 { return Err(()) }
                                                 if i == 0 {
                                                     holder.left_bytes = &holder.left_bytes[2..];
                                                     #[cfg(feature = "accept_transfer_chunked")]
                                                     {
                                                            if let Some(to_advance_bytes) = &mut self.to_advance_bytes {
                                                             *to_advance_bytes = self.original_left_bytes_length - holder.left_bytes.len();
                                                            } else {
                                                                *self.to_advance_bytes = Some(self.original_left_bytes_length - holder.left_bytes.len());
                                                            }
                                                     }
                                                 }
                                                 #[cfg(feature = "debugging")]{
                                                     tracing::info!("after chunked payload proceed {:?}",
                                                       String::from_utf8_lossy(holder.left_bytes)
                                                     )
                                                 }
                                                 Ok(())
                                             }
                                         }
                                     }
                                     Err(_) => { Err(()) }
                                 }
                             }


                             match find_new_line(holder.left_bytes,chunk_op.chunk_size) {
                                 Ok(new_line) => {
                                     match new_line {
                                         None => {
                                             try_call_back!(callback,chunk_op,holder.left_bytes);
                                             holder.left_bytes = &[];
                                             return  h1_chunk_detecting_on_stream(
                                                 holder,
                                                 self.reading_buffer,
                                                 chunk_index,
                                                 callback,
                                                 chunk
                                             ).await
                                         }
                                         Some(n) => {
                                             try_call_back!(callback,chunk_op,&holder.left_bytes[..n]);
                                             if   holder.left_bytes.len() <= 2 {
                                                 chunk = None;
                                                 holder.left_bytes = &[];
                                                 return  h1_chunk_detecting_on_stream(
                                                     holder,
                                                     self.reading_buffer,
                                                     chunk_index,
                                                     callback,
                                                     chunk
                                                 ).await;
                                             }
                                             holder.left_bytes = &holder.left_bytes[n+2..];
                                             #[cfg(feature = "debugging")]
                                             {
                                                 tracing::debug!("left bytes after advanced {:?}",
                                                  String::from_utf8_lossy(holder.left_bytes
                                                  )
                                              )
                                             }
                                             chunk = None;
                                             continue;
                                         }
                                     }
                                 }
                                 Err(_) => {
                                     #[cfg(feature = "debugging")]
                                     {
                                         tracing::error!("there is no new line when it should be {:?}",
                                           String::from_utf8_lossy(holder.left_bytes )
                                         )
                                     }
                                     return Err(())}
                             }

                         }
                     }
                 }
             }



             MultipartStreamHolder::H2(h2) => {
                 let mut chunk:Option<Chunk> = None;

                 let  body_reader = h2.batch.body_mut();
                 loop {
                     #[cfg(feature = "debugging")]
                     {
                         println!("chunk is {:?}",chunk);
                     }
                     match &chunk {
                         None => {

                             let  data = self.reading_buffer.chunk();
                             return  match find_new_line(data,16) {
                                 Ok(index_option) => {
                                     #[cfg(feature = "debugging")]
                                     {
                                         tracing::debug!("trying to find chunk on {:?}",index_option)
                                     }
                                     match index_option {
                                         None => {
                                           if let Some(r) = body_reader.data().await {
                                               if let Ok(r) = r  {
                                                   self.reading_buffer.extend_from_slice(&r);
                                                   continue
                                               } else {

                                                   return Err(())
                                               }

                                           }
                                           else {
                                               return Ok(())
                                           }
                                         }
                                         Some(i) => {

                                             let chunk_size = hex_bytes_to_usize(&data[..i]);
                                             if let Some(chunk_size) = chunk_size {
                                                 #[cfg(feature = "debugging")]
                                                 {
                                                     tracing::debug!("chunk size is {}",chunk_size)
                                                 }
                                                 chunk = Some(Chunk { index: *chunk_index, chunk_size });

                                                 *chunk_index += 1;
                                                 if i + 2 >= data.len() {
                                                     self.reading_buffer.clear();
                                                     continue;
                                                 }
                                                 self.reading_buffer.advance(i+2);
                                                 #[cfg(feature = "debugging")]
                                                 {
                                                     tracing::debug!("data after advanced {}",
                                                       String::from_utf8_lossy(self.reading_buffer.chunk())
                                                     )
                                                 }
                                                 continue;
                                             }
                                             Err(())
                                         }
                                     }
                                 }
                                 Err(_) => {  Err(())}
                             }
                         }
                         Some(chunk_op) => {

                             let data = self.reading_buffer.chunk();
                             if chunk_op.chunk_size == 0 {
                                 #[cfg(feature = "debugging")]{
                                     println!("the last chunk payload {:?}",
                                              String::from_utf8_lossy(data)
                                     )
                                 }
                                 return match find_new_line(data, 4) {
                                     Ok(index_option) => {
                                         match index_option {
                                             None => {
                                                 if let Some(r) = body_reader.data().await {
                                                     if let Ok(r) = r {
                                                         self.reading_buffer.extend_from_slice(&r);
                                                         continue
                                                     } else {
                                                         return Err(());
                                                     }
                                                 } else {
                                                     return Ok(())
                                                 }
                                             }
                                             Some(i) => {
                                                 #[cfg(feature = "debugging")]{
                                                     println!("the last chunk was found on {i}"
                                                     )
                                                 }
                                                 if data.len() < 2 { return Err(()) }
                                                 if i == 0 {
                                                     self.reading_buffer.advance(2);
                                                 }
                                                 #[cfg(feature = "debugging")]{
                                                     tracing::info!("after chunked payload proceed {:?}",
                                                       String::from_utf8_lossy(self.reading_buffer.chunk())
                                                     )
                                                 }
                                                 Ok(())
                                             }
                                         }
                                     }
                                     Err(_) => { Err(()) }
                                 }
                             }

                             let data = self.reading_buffer.chunk();
                             match find_new_line(data,chunk_op.chunk_size) {
                                 Ok(new_line) => {
                                     match new_line {
                                         None => {
                                             try_call_back!(callback,chunk_op,data);
                                             self.reading_buffer.clear();
                                             if let Some(r) = body_reader.data().await {
                                                 if let Ok(r) = r {
                                                     self.reading_buffer.extend_from_slice(&r);
                                                     continue
                                                 }
                                                 return Err(())
                                             } else {
                                                 return Ok(())
                                             }
                                         }
                                         Some(n) => {
                                             let data = self.reading_buffer.chunk();
                                             try_call_back!(callback,chunk_op,&data[..n]);
                                             if   data.len() <= 2 {
                                                 chunk = None;
                                                 self.reading_buffer.clear();
                                                 if let Some(r) = body_reader.data().await {
                                                     if let Ok(r) = r {
                                                         self.reading_buffer.extend_from_slice(&r);
                                                         continue
                                                     }
                                                     return Err(())
                                                 } else {
                                                     return Ok(())
                                                 }
                                             }
                                             self.reading_buffer.advance(n+2);
                                             #[cfg(feature = "debugging")]
                                             {
                                                 tracing::debug!("left bytes after advanced {:?}",
                                                  String::from_utf8_lossy(self.reading_buffer.chunk()
                                                  )
                                              )
                                             }
                                             chunk = None;
                                             continue;
                                         }
                                     }
                                 }
                                 Err(_) => {
                                     #[cfg(feature = "debugging")]
                                     {
                                         tracing::error!("there is no new line when it should be {:?}",
                                           String::from_utf8_lossy(data)
                                         )
                                     }
                                     return Err(())}
                             }

                         }
                     }
                 }
             }
         }
     }



 }


#[cfg(feature = "accept_transfer_chunked")]

async fn h1_chunk_detecting_on_stream(
    holder:& mut H1StreamHolder<'_>,
    reader:&mut BodyReadingBuffer,
    chunk_index:&mut usize,
    mut callback:impl FnMut(&Chunk,&[u8])->FieldCallBackResult,
    mut chunk: Option<Chunk>,
)->Result<(),()>{

    loop {
        match &mut chunk {
            None => {

                if reader.is_empty()  {
                    if reader.read_buf(holder.stream).await.is_err() {return Err(())}
                }
                let data = reader.chunk();
                if data.is_empty() {return Err(())}
                match find_new_line(data,16) {
                    Ok(index_option)=>{
                        match  index_option {
                            None => { continue}
                            Some(index) => {
                                let  c = &data[..index];
                                if index + 2 >= data.len() { continue }
                                let chunk_size = match hex_bytes_to_usize(c) {
                                    None => { return Err(())}
                                    Some(r) => {r}
                                };
                                chunk  = Some(Chunk { index:*chunk_index,chunk_size});
                                *chunk_index+=1;
                                reader.advance(index+2);
                                if chunk_size == 0 {
                                    if reader.len() < 2{return  Err(()) }
                                    reader.advance(2);
                                    return Ok(())
                                }
                                #[cfg(feature = "debugging")]
                                {
                                    tracing::debug!("bytes after advanced {}",String::from_utf8_lossy(reader.chunk()))
                                }
                            }
                        }
                    }
                    Err(_)=>{ return Err(())}
                }
            }
            Some(chunk_oop) => {
                if reader.is_empty()  {
                    if reader.read_buf(holder.stream).await.is_err() {return Err(())}
                }
                let data = reader.chunk();
                if data.is_empty() {return Err(())}

                match find_new_line(data,chunk_oop.chunk_size  + 2 ) {
                    Ok(op) => {
                        match op {
                            None => {
                                match callback(chunk_oop,data) {
                                    Ok(f)=>{
                                        if let Some(f) = f {
                                            if f.await.is_err() {return Err(())}
                                        }
                                        reader.clear();

                                    }
                                    Err(_)=> { return Err(())}
                                }
                            }
                            Some(i) => {
                                match callback(chunk_oop,&data[..i]) {
                                    Ok(future) => {
                                        if let Some(future
                                        ) = future {
                                            if future.await.is_err()  { return Err(())}
                                        }
                                        if data.len() < i + 2 { return Err(()) }
                                        else {reader.advance(i+2);}
                                        chunk = None;
                                    }
                                    Err(_) => { return Err(())}
                                }
                            }
                        }
                    }
                    Err(_) => {return Err(())}
                }
            }
        }
    }
}

#[cfg(feature = "accept_transfer_chunked")]

#[inline]
fn find_new_line(data:&[u8],cap:usize)->Result<Option<usize>,()>{
    let mut co = 0_u8 ;
    for (index,byte) in data.iter().enumerate() {
        match byte {
            b'\r'=>{ co+=1;}
            b'\n'=>{ if co == 1  { return Ok(Some(index - 1))}}
            _ => {
                if index >= cap { return Err(())}
                co = 0;
            }
        }
    }
    Ok(None)
}

