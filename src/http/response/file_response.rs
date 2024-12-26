use std::path::Path;
use crate::server::WRITING_FILES_BUF_LEN;

/// helper struct to specify file sending response configurations
 pub struct FileRSender<'a>{
     pub path:&'a Path,
     pub buffer_size_for_reading_from_file_and_writing_to_stream:usize,
     pub range:Option<(Option<usize>,Option<usize>)>
 }


impl <'a> FileRSender<'a> {

    /// creating new file response config
    pub fn custom(path:&Path,
               buffer_size_for_reading_from_file_and_writing_to_stream:usize,
    )->FileRSender{
        FileRSender {
            path,
            buffer_size_for_reading_from_file_and_writing_to_stream,
            range:None
        }
    }
    /// creating new file response with only path
    pub fn new(path:&str)->FileRSender{
        Self::custom(Path::new(path),WRITING_FILES_BUF_LEN)
    }

    /// to initiate range bytes
    /// and this is used to determine what is the start and the end of bytes that sending
    pub fn set_bytes_range(&mut self,start:Option<usize>,end:Option<usize>){
        self.range = Some((start,end));
    }
}


