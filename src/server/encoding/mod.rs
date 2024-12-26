

/// for preparing Encoding logic to http response
 pub struct EncodingConfigurations {

}

/// defining encoding logic
pub enum EncodingLogic {
    All,
    Zstd,
    Brotli,
    Gzip,
    Deflate,
    Custom(fn (&[u8],&mut Vec<u8>))
}