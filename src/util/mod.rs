
use std::path::Path;
/// for finding custom bytes in custom buffer or data
pub fn find(original:&[u8], patter:&[u8]) ->Option<usize>{
    twoway::find_bytes(original,patter)
}



pub (crate) fn split<'bytes>(original:&'bytes [u8], patter:&[u8])->BytesSplit<'bytes>{
    let length = patter.len();
    let mut split = BytesSplit::new();
    if let Some(index) = find(original,patter) {
        split.set_first(&original[..index]);
        split.set_last(&original[index+length..]);
    }
    return split
}

/// for implementing splitting functions to custom data flow
pub  struct BytesSplit<'a> {
    first:Option<&'a [u8]>,
    last:Option<&'a [u8]>
}
impl <'a> BytesSplit<'a> {
    pub fn new()->BytesSplit<'a> {
        BytesSplit {
            first:None,
            last:None
        }
    }


    pub (crate) fn set_first(&mut self,bytes:&'a[u8]){
        self.first = Some(bytes);
    }

    pub (crate) fn set_last(&mut self,bytes:&'a[u8]){
        self.last = Some(bytes);
    }


    /// getting the last element or bytes of split object
    pub fn last(&self)->Option<&'a[u8]> { self.last }
    /// getting the first element or bytes of split object
    pub fn first(&self)->Option<&'a[u8]> { self.first }

}


/// for converting usize bytes to usize object in rust
pub  fn bytes_to_usize(bytes:&[u8])->Option<usize>{
    const LEN:usize = size_of::<usize>();
    let  mut data =  [0u8;LEN];
    let res = &mut data;

    if bytes.len() > 8 { return None}
    let mut index = LEN - 1;
    for byte in bytes {
        res[index] = *byte;
        if index == 0 { break; }
        index-=1;
    }
    Some(usize::from_be_bytes(data))
}


/// for converting hexadecimals bytes to usize data type
pub fn hex_bytes_to_usize(data: &[u8]) -> Option<usize> {
    // Convert byte slice to UTF-8 string
    std::str::from_utf8(data)
        .ok() // Handle invalid UTF-8
        .and_then(|s| {
            // Trim whitespace and parse as hexadecimal (base-16)
            usize::from_str_radix(s.trim(), 16).ok()
        })
}
pub (crate) fn content_type_from_file_path(path: &&Path) -> Option<&'static str> {
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("");

    match extension.to_lowercase().as_str() {
        "ico"=>Some("image/x-icon"),
        "txt" => Some("text/plain"),
        "html" | "htm" => Some("text/html"),
        "css" => Some("text/css"),
        "js" => Some("application/javascript"),
        "xml" => Some("application/xml"),
        "jpeg" | "jpg" => Some("image/jpeg"),
        "png" => Some("image/png"),
        "gif" => Some("image/gif"),
        "bmp" => Some("image/bmp"),
        "webp" => Some("image/webp"),
        "svg" => Some("image/svg+xml"),
        "mp3" => Some("audio/mpeg"),
        "wav" => Some("audio/wav"),
        "ogg" => Some("audio/ogg"),
        "aac" => Some("audio/aac"),
        "midi" => Some("audio/midi"),
        "mp4" => Some("video/mp4"),
        "webm" => Some("video/webm"),
        "avi" => Some("video/x-msvideo"),
        "mpeg" => Some("video/mpeg"),
        "json" => Some("application/json"),
        "pdf" => Some("application/pdf"),
        "zip" => Some("application/zip"),
        "gz" | "gzip" => Some("application/gzip"),
        "bin" => Some("application/octet-stream"),
        "docx" => Some("application/vnd.openxmlformats-officedocument.wordprocessingml.document"),
        "xlsx" => Some("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"),
        "pptx" => Some("application/vnd.openxmlformats-officedocument.presentationml.presentation"),
        "rtf" => Some("application/rtf"),
        "ttf" => Some("font/ttf"),
        "otf" => Some("font/otf"),
        "woff" => Some("font/woff"),
        "woff2" => Some("font/woff2"),
        "tar" => Some("application/x-tar"),
        "rar" => Some("application/vnd.rar"),
        _ => None,
    }
}




#[inline]
pub (crate) fn found_boundary_in(data:&[u8],pattern:&[u8])->PatternExistResult{
    let pattern_length = pattern.len();

    if data.len() < pattern_length {return PatternExistResult::None}
    let mut find_counter = 0;
    let mut start_sign_counter = 0_usize;
    for (index,byte) in data.iter().enumerate() {
        let target = pattern.get(find_counter).unwrap_or(&0);
        if byte ==  target {
            if start_sign_counter < 4 {
                if start_sign_counter>=2 {
                    start_sign_counter+=1;
                }
            } else if start_sign_counter == 4 {
                find_counter=0;
                start_sign_counter = 5;
            }
            find_counter+=1;
            if find_counter >= pattern_length {
                let res = index+1 - find_counter - (start_sign_counter-1);
                return PatternExistResult::Some(res);
            }
        }
        else {
            if byte == &b'\r' && start_sign_counter==0 {
                start_sign_counter+=1;
            } else if byte == &b'\n' && start_sign_counter == 1 {
                start_sign_counter+=1;
            }
            find_counter = 0;
        }
    }
    if find_counter > 1 {
        return  PatternExistResult::MaybeExistOnLastBytesFromLen(pattern_length-find_counter)
    }
    PatternExistResult::None
}


/// for extending data (bytes) to custom buffer or vector until we found some pattern or needle
#[inline]
pub  fn extend_to_buffer_until(data:&[u8],pattern:&[u8])->PatternExistResult{
    let pattern_length = pattern.len();
    if data.len() < pattern_length {return PatternExistResult::None}
    let mut find_counter = 0;
    for (index,byte) in data.iter().enumerate() {
        let target = pattern.get(find_counter).unwrap_or(&0);

        if byte ==  target {
            find_counter+=1;
            if find_counter >= pattern_length {
                let res = index+1 - find_counter;
                return PatternExistResult::Some(res);
            }
        }
        else {
            find_counter = 0;
        }
    }
    if find_counter > 1 {
        return  PatternExistResult::MaybeExistOnLastBytesFromLen(pattern_length-find_counter)
    }
    PatternExistResult::None
}

/// designed to work with utils mod
/// and specially the function that return if there is any possibilities that pattern may be existed in some bytes order
pub  enum PatternExistResult {
    /// if the pattern was existed
    Some(usize),
    /// if there is a chance that pattern may be existed
    /// to be aware handling that portion of data
    MaybeExistOnLastBytesFromLen(usize),
    /// if the patter has no any possible way to be existed
    None
}



#[test]
fn check_finder(){
    let data = [4,8,6,1,0,10,8,6,4];
    let pattern = [8,6,1,0,10,];
    _= extend_to_buffer_until(&data,&pattern);
}