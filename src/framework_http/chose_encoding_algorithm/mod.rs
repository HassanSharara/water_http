use std::io::Write;
use brotli::CompressorWriter;
use flate2::write::{DeflateEncoder, GzEncoder};


pub (crate) enum  HttpEncodingAlgorithms {
    ZStd,
    Brotli,
    Gzip,
    Deflate
}
pub (crate)  fn detect_encoding_algorithm(encoding_header:&Vec<Vec<String>>)->Option<HttpEncodingAlgorithms>{
    for _m in encoding_header {
        if let Some(_) =  _m.iter().find(|value |value.to_lowercase().contains("zstd")) {
            return Some(HttpEncodingAlgorithms::ZStd);
        }
        else if let Some(_) = _m.iter().find(|value|value.contains("br") || value.contains("brotli")) {
              return  Some(HttpEncodingAlgorithms::Brotli);
        }
        else if let Some(_) =  _m.iter().find(|value |value.contains("gzip")) {
           return  Some(HttpEncodingAlgorithms::Gzip);
        }
        else if let Some(_) = _m.iter().find(|value|value.contains("deflate") ){
           return  Some(HttpEncodingAlgorithms::Deflate);
        }
    }
   None
}


pub (crate)  fn encode_data_with_z_std(bytes:&[u8],data:&mut Vec<u8>,)->Result<(),()>{
    let compressor = zstd::Encoder::new(bytes.to_vec(),3);
    if let Ok(_compressor) = compressor {
        if let Ok(compressor_data) = _compressor.finish() {
            *data = compressor_data;
            return  Ok(());
        }
    }
    Err(())
}
pub (crate)  fn encode_data_with_brotli(bytes:&[u8],data:&mut Vec<u8>,)->Result<(),()>{
    let mut compressed_data = Vec::new();
    let mut compressor = CompressorWriter::new(&mut compressed_data, 4096, 11, 22);
    let _r =  compressor.write_all(bytes);
    if let Ok(_) = _r {
        drop(compressor);
        *data = compressed_data;
        return Ok(());
    }
    Err(())
}

pub (crate)  fn encode_data_with_gzip(bytes:&[u8],data:&mut Vec<u8>)->Result<(),()>{
    let mut encoder = GzEncoder::new(Vec::new(), flate2::Compression::default());
    if let Ok(_) = encoder.write_all(bytes) {
        if let Ok(compressed_data ) =  encoder.finish() {
            *data = compressed_data;
            return Ok(());
        }
    }
    Err(())
}
pub (crate)  fn encode_data_with_deflate(bytes:&[u8],data:&mut Vec<u8>)->Result<(),()>{
    let mut encoder = DeflateEncoder::new(Vec::new(), flate2::Compression::default());
    if let Ok(__) = encoder.write_all(bytes) {
        if let Ok(compressed_data ) =  encoder.finish() {
            *data = compressed_data;
            return Ok(());
        }
    }
    Err(())
}
// pub fn encode(bytes:&[u8],data:&mut Vec<u8>,encode_method:&HttpEncodingAlgorithms){
//     match encode_method {
//         HttpEncodingAlgorithms::ZStd => {
//             let _ = encode_data_with_z_std(bytes,data);
//         }
//         HttpEncodingAlgorithms::Brotli => {
//             let _ =  encode_data_with_brotli(bytes,data);
//         }
//         HttpEncodingAlgorithms::Gzip => {
//             let _ = encode_data_with_gzip(bytes,data);
//         }
//         HttpEncodingAlgorithms::Deflate => {
//            let _ = encode_data_with_deflate(bytes,data);
//         }
//     }
// }