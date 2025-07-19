use bytes::{Buf, Bytes};
#[cfg(feature = "debugging")]
use tracing::{field::debug,debug, error, info};
use crate::http::request::{FieldCallBackResult, H1StreamHolder, MultipartStreamHolder};
use crate::server::connection::BodyReadingBuffer;
use crate::util::hex_bytes_to_usize;



/// check if reading bytes is going well or not
pub type BodyChunksReadingResult = Result<Bytes,()>;

/// incoming body chunked bytes

#[derive(Debug)]
/// for handling multipart from data in both protocols http1 and http2
pub struct BodyChunkedReader<'a> {
    stream_holder:MultipartStreamHolder<'a>,
    reading_buffer:&'a mut BodyReadingBuffer,
    chunk_indexes_count:usize,
    // remaining:usize,
}


#[derive(Debug)]
pub struct  Chunk {
    /// refer to the index of chunk between incoming chunks
    pub index:usize,
    /// incoming chunk length
    pub chunk_size:usize
}





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
impl<'a> BodyChunkedReader<'a> {



    /// for creating new [BodyChunkedReader]
     pub (crate) fn new(
         stream_holder:MultipartStreamHolder<'a>,
         reading_buffer:&'a mut BodyReadingBuffer,

     )->BodyChunkedReader<'a>{
         BodyChunkedReader {
             stream_holder,
             reading_buffer,
             chunk_indexes_count:0
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
                             match find_new_line(holder.left_bytes,16) {
                                 Ok(index_option) => {
                                     #[cfg(feature = "debugging")]
                                     {
                                         debug!("trying to find chunk on {:?}",index_option)
                                     }
                                     return match index_option {
                                         None => {
                                             #[cfg(feature = "debugging")]
                                             {
                                                 debug!("the left data is {}",String::from_utf8_lossy(holder.left_bytes));
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
                                                     debug!("chunk size is {}",chunk_size)
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
                                                     debug!("data after advanced {}",
                                                       String::from_utf8_lossy(holder.left_bytes)
                                                     )
                                                 }
                                                 continue;
                                             }
                                             Err(())
                                         }
                                     }
                                 }
                                 Err(_) => {}
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
                                                 if i == 0 { holder.left_bytes = &holder.left_bytes[2..] }
                                                 #[cfg(feature = "debugging")]{
                                                     info!("after chunked payload proceed {:?}",
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
                                                 debug!("left bytes after advanced {:?}",
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
                                         error!("there is no new line when it should be {:?}",
                                           String::from_utf8_lossy(holder.left_bytes )
                                         )
                                     }
                                     return Err(())}
                             }

                         }
                     }
                 }
             }
             MultipartStreamHolder::H2(_) => {
                 todo!()
             }
         }
     }



 }



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
                                reader.advance(index+2);
                                #[cfg(feature = "debugging")]
                                {
                                    debug!("bytes after advanced {}",String::from_utf8_lossy(reader.chunk()))
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

                match find_new_line(data,chunk_oop.chunk_size) {
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
                                        if data.len() <= i +2 { return Err(()) }
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

