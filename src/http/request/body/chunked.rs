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
                 let mut chunk_count = *chunk_index;
                 let body_reader = h2.batch.body_mut();

                 // 1. Process anything currently sitting in our local reading buffer
                 let local_chunk = self.reading_buffer.chunk();
                 if !local_chunk.is_empty() {
                     let current_chunk = Chunk {
                         index: chunk_count,
                         chunk_size: local_chunk.len(),
                     };
                     try_call_back!(callback, &current_chunk, local_chunk);
                     chunk_count += 1;
                     self.reading_buffer.clear();
                 }

                 // 2. Drive the active incoming stream
                 // Check if the stream has reached EOF natively
                 if body_reader.is_end_stream() {
                     *chunk_index = chunk_count;
                     return Ok(());
                 }

                 // Await the next frame fragment.
                 // If this is a half-send, this will unblock the moment frame 1 hits,
                 // execute the callback, and exit cleanly so your router handles it immediately.
                 if let Some(frame_result) = body_reader.data().await {
                     match frame_result {
                         Ok(raw_bytes) => {
                             let frame_data = raw_bytes.as_ref();
                             if !frame_data.is_empty() {
                                 let current_chunk = Chunk {
                                     index: chunk_count,
                                     chunk_size: frame_data.len(),
                                 };

                                 try_call_back!(callback, &current_chunk, frame_data);
                                 chunk_count += 1;
                             }
                         }
                         Err(_) => {
                             return Err(());
                         }
                     }
                 }

                 *chunk_index = chunk_count;
                 return Ok(());
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

    #[cfg(feature = "debugging")]
    {
        tracing::debug!("[h1_chunk_detecting_on_stream] function called");
    }


    loop {
        match &mut chunk {

            None => {
                #[cfg(feature = "debugging")]
                {
                    tracing::debug!("now chunk size is None so we need to find next chunk size");
                }
                if reader.is_empty()  {
                    #[cfg(feature = "debugging")]
                    tracing::info!("[Chunk Handler]: trying to read more data from stream ");
                    if reader.read_buf(holder.stream).await.is_err() {return Err(())}
                    #[cfg(feature = "debugging")]
                    tracing::info!("[Chunk Handler]: after reading data from stream {:?}",String::from_utf8_lossy(reader.chunk()));
                }
                let data = reader.chunk();

                #[cfg(feature = "debugging")]
                {
                    tracing::info!("while chunk is None data is {:?}",String::from_utf8_lossy(data));
                }

                if data.is_empty() {
                    #[cfg(feature = "debugging")]{
                        tracing::error!("body chunk ended with error");
                    }
                    return Err(())
                }

                #[cfg(feature = "debugging")]{
                    tracing::debug!("try to find new line");
                }
                match find_new_line(data,16) {
                    Ok(index_option)=>{
                        #[cfg(feature = "debugging")]{
                            tracing::debug!("new line  {:?}",index_option);
                        }
                        match  index_option {
                            None => {
                                #[cfg(feature = "debugging")]
                                {
                                    tracing::debug!("trying to read more data"
                                    );
                                }
                                if reader.read_buf(holder.stream).await.is_err() {return Err(())}
                                #[cfg(feature = "debugging")]
                                {
                                    tracing::debug!("[Chunk:reading within chunk index] None {:?}",
                                      String::from_utf8_lossy(reader.chunk())
                                    );
                                }
                                continue
                            }
                            Some(index) => {
                                let  c = &data[..index];
                                if index + 2 >= data.len() {
                                    #[cfg(feature = "debugging")]
                                    {
                                        tracing::debug!(
                                            "after found new line the data is not complete {:?}",
                                            String::from_utf8_lossy(c)
                                        );
                                    }

                                    if !c.is_empty() {
                                        if reader.read_buf(holder.stream).await.is_err() {
                                            return  Err(())
                                        }
                                    }
                                    continue
                                }
                                #[cfg(feature = "debugging")]{
                                    tracing::debug!("chunk size as hex {:?}",String::from_utf8_lossy(c));
                                }
                                let chunk_size = match hex_bytes_to_usize(c) {
                                    None => { return Err(())}
                                    Some(r) => {r}
                                };
                                #[cfg(feature = "debugging")]{
                                    tracing::debug!("chunk size is {}",chunk_size);
                                }
                                chunk  = Some(Chunk { index:*chunk_index,chunk_size});
                                *chunk_index+=1;
                                reader.advance(index+2);
                                if chunk_size == 0 {
                                    if reader.len() < 2{return  Err(()) }
                                    reader.advance(2);
                                    return Ok(())
                                }
                                else if chunk_size > reader.len() {
                                    if reader.read_buf(holder.stream).await.is_err() {
                                        return Err(())
                                    }
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
                #[cfg(feature = "debugging")]
                {
                    tracing::debug!("now chunk size is {}",chunk_oop.chunk_size);
                }

                if reader.is_empty()   {
                    #[cfg(feature = "debugging")]
                    tracing::info!("[Chunk Handler]: trying to read more data from stream ");
                    if reader.read_buf(holder.stream).await.is_err() {return Err(())}
                    #[cfg(feature = "debugging")]
                    tracing::info!("[Chunk Handler]: after reading data from stream {:?}",String::from_utf8_lossy(reader.chunk()));
                }
                let data = reader.chunk();
                if data.is_empty() {return Err(())}
                #[cfg(feature = "debugging")]
                {
                    tracing::debug!("looking for new line in {:?}",String::from_utf8_lossy(data));
                }
                match find_new_line(data,chunk_oop.chunk_size  + 2 ) {
                    Ok(op) => {
                        match op {
                            None => {
                                match callback(chunk_oop,data) {
                                    Ok(f)=>{
                                        if let Some(f) = f {
                                            if f.await.is_err() {
                                                #[cfg(feature = "debugging")]
                                                {
                                                    tracing::error!("error while processing chunk future ");
                                                }
                                                return Err(())}
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
                    Err(_) => {
                        #[cfg(feature = "debugging")]
                        {
                            tracing::error!("failed to found new line while chunk_size = {} while data is {:?}",
                                chunk_oop.chunk_size,
                                 String::from_utf8_lossy(data)
                            );
                        }

                        return Err(())}
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

