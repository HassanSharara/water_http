use std::path::Path;

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

pub (crate) struct BytesSplit<'a> {
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

    pub (crate) fn is_none(&self)->bool {
    self.last.is_none() && self.first.is_none()
   }
    pub (crate) fn set_first(&mut self,bytes:&'a[u8]){
        self.first = Some(bytes);
    }

    pub (crate) fn set_last(&mut self,bytes:&'a[u8]){
        self.last = Some(bytes);
    }


    pub fn last(&self)->Option<&'a[u8]> { self.last }
    pub fn first(&self)->Option<&'a[u8]> { self.first }
}



pub (crate) fn bytes_to_usize(bytes:&[u8])->Option<usize>{
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
pub (crate) fn found_boundary_in(mut data:&[u8],pattern:&[u8])->PatternExistResult{
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

#[inline]
pub (crate) fn extend_to_buffer_until(mut data:&[u8],pattern:&[u8])->PatternExistResult{
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

pub (crate) enum PatternExistResult {
    Some(usize),
    MaybeExistOnLastBytesFromLen(usize),
    None
}



#[test]
fn check_finder(){
    let data = [4,8,6,1,0,10,8,6,4];
    let pattern = [8,6,1,0,10,];
    _= extend_to_buffer_until(&data,&pattern);
}