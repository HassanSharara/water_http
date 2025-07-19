use std::path::Path;
use crate::server::WRITING_FILES_BUF_LEN;

/// helper struct to specify file sending response configurations
 pub struct FileRSender<'a>{
     pub path:&'a Path,
     pub buffer_size_for_reading_from_file_and_writing_to_stream:usize,
     pub range:Option<(Option<usize>,Option<usize>)>,
     pub edit_each_chunk:Option<Box<dyn FnMut(&mut [u8] )+ Send >>
 }


impl <'a> FileRSender<'a> {

    /// creating new file response config
    pub fn custom(path:&Path,
               buffer_size_for_reading_from_file_and_writing_to_stream:usize,
    )->FileRSender{
        FileRSender {
            path,
            buffer_size_for_reading_from_file_and_writing_to_stream,
            range:None,
            edit_each_chunk:None
        }
    }
    /// creating new file response with only path
    pub fn new(path:&str)->FileRSender{
        Self::custom(Path::new(path),WRITING_FILES_BUF_LEN)
    }

    /// setting edit each chunk function to encode data or bitwise shifting or whatever you want
    pub fn set_edit_each_chunk(&mut self,callback:impl FnMut(&mut [u8]) + Send + 'static){
        self.edit_each_chunk = Some(Box::new(callback));
    }
    /// to initiate range bytes
    /// and this is used to determine what is the start and the end of bytes that sending
    pub fn set_bytes_range(&mut self,start:Option<usize>,end:Option<usize>){
        self.range = Some((start,end));
    }
}


