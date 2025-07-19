
use bytes::Buf;
use h2::RecvStream;
use http::Request;
use crate::server::connection::BodyReadingBuffer;
use crate::server::errors::{ServerError, WaterErrors};
use crate::server::HttpStream;

#[derive(Debug)]
pub (crate) enum StreamBytesPuller<'a> {
    H1(H1BytesPuller<'a>),
    H2(H2BytesPuller<'a>)
}
#[derive(Debug)]
pub (crate) struct H1BytesPuller<'a> {
    pub(crate)stream:&'a mut HttpStream,
    pub(crate)reading_buffer:&'a mut BodyReadingBuffer,
    pub(crate) left_bytes:&'a [u8]
}
#[derive(Debug)]
pub (crate) struct  H2BytesPuller<'a>{
    pub(crate)batch:&'a mut Request<RecvStream>,
}

#[derive(Debug)]
/// struct for handling  body bytes as chunks with very efficient way
 pub struct BytesPuller<'a >{
       puller:StreamBytesPuller<'a>,
       content_length:usize,
 }


impl <'a> BytesPuller <'a> {



    pub (crate) fn new(
        puller:StreamBytesPuller<'a>,
        content_length:usize)->BytesPuller<'a>{
        BytesPuller {
            puller,
            content_length
        }
    }

    /// reading each chunk seperated and parsed to [FnMut] Closure
    pub async fn on_chunk(&mut self,mut callback:impl FnMut(&[u8])-> Result<(),()> )
    ->Result<(),WaterErrors>{
        let content_length = self.content_length;

        match &mut self.puller {
            StreamBytesPuller::H1(h1) => {
                let left_bytes = h1.left_bytes;
                let err= Err(WaterErrors::Server(
                    ServerError::HANDLING_INCOMING_BODY_ERROR
                ));


                if content_length < left_bytes.len() {
                    let data = &left_bytes[..content_length];
                    if let Err(_) = callback(data) {
                        return  err
                    }
                    return Ok(())
                } else {
                    let mut remaining = content_length;
                    if let Err(_) = callback(left_bytes) {
                        return  err
                    }
                    remaining-=left_bytes.len();
                    loop {
                        if remaining < 1 {
                            return  Ok(())
                        }
                        if h1.reading_buffer.read_buf(h1.stream).await.is_err() { return  err}

                        let data = h1.reading_buffer.chunk();
                        let to_index = remaining.min(data.len());
                        if callback(&data[..to_index]).is_err() { return  err}
                        remaining-=to_index;
                        continue;
                    }
                }
            }
            StreamBytesPuller::H2(h2) => {
                let mut remaining = self.content_length;
                let body_mut = h2.batch.body_mut();
                let err= Err(
                    WaterErrors::Server(
                        ServerError::HANDLING_INCOMING_BODY_ERROR
                    )
                );
                while remaining > 0 {
                    let data = body_mut.data().await;
                    match data {
                        None => { break }
                        Some(data) => {
                            match data {
                                Ok(data) => {
                                    if callback(data.as_ref()).is_err()  { return err}
                                    remaining-=data.len();
                                    continue;
                                }
                                Err(_) => {
                                    return err
                                }
                            }
                        }
                    }
                }
                return Ok(())
            }
        }


    }

}