use std::borrow::Cow;
use std::future::Future;
use std::io::Write;
use std::pin::Pin;
use flate2::write::{GzEncoder,DeflateEncoder};
/// for providing encoding configurations for response data
#[derive(Debug)]
pub struct EncodingConfigurations {
    pub(crate) logic:EncodingLogic,
    /// - this framework support encoding with all encoding algorithms
    /// ['zstd,Gzip,Deflate,Brotli'] so the response will be compressed with one of these
    /// algorithms depending on the threshold of the data you need to send
    /// so the default value is 4000000 which is approximately [4 MB ]
    /// so if your server is very close to your clients leave this value as default but
    /// if your server is a little far from your client then try to decrease this threshold
    /// to get the best response latency
    /// also notice that when you send a custom headers you should implement this encoding manually
    pub threshold_for_encoding_response:usize,

}

impl EncodingConfigurations {

    pub (crate) fn is_not_none(&self)->bool {self.logic.is_not_none()}

    /// creating new [EncodingConfigurations] with default values
    /// logic : [EncodingLogic::None]
    /// threshold: 4_000_000
    pub fn default()->Self{
        EncodingConfigurations {
            logic:EncodingLogic::None,
            threshold_for_encoding_response:4_000_000
        }
    }


    /// for setting new logic [EncodingLogic]
    pub fn set_logic(&mut self,logic:EncodingLogic){
        self.logic = logic;
    }


    /// for setting new threshold
    pub fn set_threshold(&mut self,max:usize){
        self.threshold_for_encoding_response = max;
    }


    pub (crate) async fn encode(&self,accept_encoding:Cow<'_, str>,data:&[u8])->
                                                                              Option<EncodedData> {
        let accept_encoding = accept_encoding.to_lowercase();
        match self.logic {
            EncodingLogic::All => {return self.default_encode(accept_encoding,data).await}
            EncodingLogic::Default => {return self.default_encode(accept_encoding,data).await}
            EncodingLogic::Zstd => {
                if accept_encoding.contains("zstd") {
                    let data = zstd_encode(data);
                    if let Some(data) = data {
                        return Some(
                            EncodedData::new(
                                EncodingLogic::Zstd,
                                data
                            )
                        )
                    }
                }
            }
            EncodingLogic::Brotli => {
                if accept_encoding.contains("br") {
                    let data = brotli_encode(data);
                    if let Some(data) = data {
                        return Some(
                            EncodedData::new(
                                EncodingLogic::Brotli,
                                data
                            )
                        )
                    }
                }
            }
            EncodingLogic::Gzip => {
                if accept_encoding.contains("gzip") {
                    let data = gzip_encode(data);
                    if let Some(data) = data {
                        return Some(
                            EncodedData::new(
                                EncodingLogic::Gzip,
                                data
                            )
                        )
                    }
                }
            }
            EncodingLogic::Deflate => {
                if accept_encoding.contains("deflate") {
                    let data = deflate_encode(data);
                    if let Some(data) = data {
                        return Some(
                            EncodedData::new(
                                EncodingLogic::Default,
                                data
                            )
                        )
                    }
                }
            }
            EncodingLogic::Lz4 => {
                if accept_encoding.contains("lz4") {
                    let data = lz4_encode(data);
                    if let Some(data) = data {
                        return Some(
                            EncodedData::new(
                                EncodingLogic::Lz4,
                                data
                            )
                        )
                    }
                }
            }
            EncodingLogic::Bzip2 => {
                if accept_encoding.contains("bzip2") {
                    let data = b2zip_encode(data);
                    if let Some(data) = data {
                        return Some(
                            EncodedData::new(
                                EncodingLogic::Bzip2,
                                data
                            )
                        )
                    }
                }
            }
            EncodingLogic::Snappy => {
                if accept_encoding.contains("snappy") {
                    let data = b2zip_encode(data);
                    if let Some(data) = data {
                        return Some(
                            EncodedData::new(
                                EncodingLogic::Snappy,
                                data
                            )
                        )
                    }
                }
            }
            EncodingLogic::None => {return None}
            EncodingLogic::Custom(callback) => {
                if let Ok(res) = callback(&accept_encoding,data).await {
                    return Some(res)
                }
                return None;
            }
        }
        None
    }
    pub (crate) async fn default_encode(&self,accept_encoding:String,data:&[u8])->Option<EncodedData>{
        let logic = match Self::pick_best_encoding(&accept_encoding) {
            Some(d)=>{d}
            _ => { return None}
        };
         match logic {
            EncodingLogic::Zstd => {
                if accept_encoding.contains("zstd") {
                    let data = zstd_encode(data);
                    if let Some(data) = data {
                        return Some(
                            EncodedData::new(
                                EncodingLogic::Zstd,
                                data
                            )
                        )
                    }
                }
            }
            EncodingLogic::Brotli => {
                if accept_encoding.contains("br") {
                    let data = brotli_encode(data);
                    if let Some(data) = data {
                        return Some(
                            EncodedData::new(
                                EncodingLogic::Brotli,
                                data
                            )
                        )
                    }
                }
            }
            EncodingLogic::Gzip => {
                if accept_encoding.contains("gzip") {
                    let data = gzip_encode(data);
                    if let Some(data) = data {
                        return Some(
                            EncodedData::new(
                                EncodingLogic::Gzip,
                                data
                            )
                        )
                    }
                }
            }
            EncodingLogic::Deflate => {
                if accept_encoding.contains("deflate") {
                    let data = deflate_encode(data);
                    if let Some(data) = data {
                        return Some(
                            EncodedData::new(
                                EncodingLogic::Default,
                                data
                            )
                        )
                    }
                }
            }
            EncodingLogic::Lz4 => {
                if accept_encoding.contains("lz4") {
                    let data = lz4_encode(data);
                    if let Some(data) = data {
                        return Some(
                            EncodedData::new(
                                EncodingLogic::Lz4,
                                data
                            )
                        )
                    }
                }
            }
            EncodingLogic::Bzip2 => {
                if accept_encoding.contains("bzip2") {
                    let data = b2zip_encode(data);
                    if let Some(data) = data {
                        return Some(
                            EncodedData::new(
                                EncodingLogic::Bzip2,
                                data
                            )
                        )
                    }
                }
            }
            EncodingLogic::Snappy => {
                if accept_encoding.contains("snappy") {
                    let data = snappy_encode(data);
                    if let Some(data) = data {
                        return Some(
                            EncodedData::new(
                                EncodingLogic::Snappy,
                                data
                            )
                        )
                    }
                }
            }
            _ => {}

        }
        None
    }
    fn pick_best_encoding(header: &str) -> Option<EncodingLogic> {
        let supported_encodings = vec!["br", "zstd", "gzip", "deflate", "lz4", "bzip2", "snappy"];
        for encoding in supported_encodings {
            if header.contains(encoding) {
                return match encoding {
                    "br" => Some(EncodingLogic::Brotli),
                    "zstd" => Some(EncodingLogic::Zstd),
                    "gzip" => Some(EncodingLogic::Gzip),
                    "deflate" => Some(EncodingLogic::Deflate),
                    "lz4" => Some(EncodingLogic::Lz4),
                    "bzip2" => Some(EncodingLogic::Bzip2),
                    "snappy" => Some(EncodingLogic::Snappy),
                    _ => None,
                };
            }
        }
        None
    }

}

/// for encoding configurations
#[derive(Debug)]
pub enum EncodingLogic{
    /// to support all encoding algorithms
    All,
    /// works as well as [`EncodingLogic::All`] option
    Default,
    /// for providing zstd encoding only
    Zstd,
    /// for providing brotli encoding only
    Brotli,
    /// for providing gzip encoding only
    Gzip,
    /// for providing deflate encoding only
    Deflate,
    /// for providing Snappy encoding only
    Snappy,
    /// for providing Bzip2 encoding only
    Bzip2,
    /// for providing Lz4 encoding only
    Lz4,
    /// to never encode response
    None,
    /// when custom creating encoding logic
    Custom(fn (&str,&[u8])->Pin<Box<dyn Future<Output=Result<EncodedData,()>> + Send>>)
}


impl EncodingLogic {

    pub (crate) fn is_not_none(&self)->bool{
        if let EncodingLogic::None = self {return false}
        return  true
    }


    pub (crate) fn content_encoding(&self)->Option<&str>{
        match self {

            EncodingLogic::Zstd => {Some("zstd")}
            EncodingLogic::Brotli => {Some("br")}
            EncodingLogic::Gzip => {Some("gzip")}
            EncodingLogic::Deflate => {Some("deflate")}
            EncodingLogic::Snappy => {Some("snappy")}
            EncodingLogic::Bzip2 => {Some("bzip2")}
            EncodingLogic::Lz4 => {Some("lz4")}
            _ =>{None}
        }

    }

}


/// for providing encoding generic use cases
pub struct EncodedData {
    pub(crate) logic:String,
    pub(crate) data:Vec<u8>
}


impl EncodedData {

    pub (crate) fn new(logic:EncodingLogic,data:Vec<u8>)->Self{
        Self {
            logic:logic.content_encoding().unwrap_or("").to_owned(),
            data
        }
    }
}
unsafe impl  Send for EncodedData {}
unsafe impl Send for EncodingLogic {}
unsafe impl Send for EncodingConfigurations {}

fn gzip_encode(data:&[u8])->Option<Vec<u8>>{
    let mut encoder = GzEncoder::new(Vec::new(),flate2::Compression::best());
    if encoder.write_all(data).is_err() {return  None;}
    if let Ok(res) = encoder.finish() { return  Some(res)}
    None
}
fn deflate_encode(data:&[u8])->Option<Vec<u8>>{
    let mut encoder = DeflateEncoder::new(Vec::new(),flate2::Compression::best());
    if encoder.write_all(data).is_err() {return  None;}
    if let Ok(res) = encoder.finish() { return  Some(res)}
    None
}
fn zstd_encode(data:&[u8])->Option<Vec<u8>>{
    let mut encoder =
    match  zstd::stream::Encoder::new(Vec::new(),3) {
        Ok(r) => {r}
        Err(_) => { return None}
    };
    if encoder.write_all(data).is_err() {return  None}
    if let Ok(res) = encoder.finish() {
        return Some(res)
    }
    None
}
fn brotli_encode(data:&[u8])->Option<Vec<u8>>{
    let mut encoder = brotli::CompressorWriter::new(Vec::new(), 4096, 5, 22);
    if encoder.write_all(data).is_err() {return None}
    Some(encoder.into_inner())
 }
fn snappy_encode(data:&[u8])->Option<Vec<u8>> {
    let mut encoder = snap::write::FrameEncoder::new(Vec::new());
    if encoder.write_all(data).is_err() { return None}
    if let Ok(res) = encoder.into_inner() {
        return Some(res)
    }
    return None;
}
fn lz4_encode(data: &[u8]) -> Option<Vec<u8>> {
    let mut encoder = lz4::EncoderBuilder::new().build(Vec::new()).expect("Failed to create LZ4 encoder");
    if let Err(_) = encoder.write_all(data) { return None}
    let (compressed, result) = encoder.finish();
    if let Err(_) = result { return None}
    Some(compressed)
}

fn b2zip_encode(data:&[u8])->Option<Vec<u8>>{
    let mut encoder = bzip2::write::BzEncoder::new(Vec::new(),bzip2::Compression::best());
    if encoder.write_all(data).is_err() { return None}
    if let Ok(res) = encoder.finish() {
        return  Some(res);
    }
    None
}
