use h2::RecvStream;
use http::Request;
use crate::server::HttpStream;

#[derive(Debug)]
pub (crate) struct  H2StreamHolder<'a>{
    pub(crate)batch:&'a mut Request<RecvStream>,

}

#[derive(Debug)]
pub (crate) struct H1StreamHolder<'a>{
    pub(crate)stream:&'a mut HttpStream,
    pub(crate)left_bytes:&'a [u8]
}