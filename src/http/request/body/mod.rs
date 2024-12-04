mod xwwwformurlencoded;
mod multipartformdata;
mod multer;
mod bytes_puller;
mod stream_holders;
pub (crate) use stream_holders::*;

pub use multer::*;
pub use bytes_puller::*;
use std::future::Future;
use std::pin::Pin;
use tokio::io::{AsyncRead, AsyncWrite};
pub use multipartformdata::MultiPartFormDataField;
use crate::http::request::body::multipartformdata::ContentDispositionType;
pub use crate::http::request::body::xwwwformurlencoded::XWWWFormUrlEncoded;
use crate::server::errors::WaterErrors;

/// indicates the incoming body
/// as each request has it`s own body
#[derive(Debug)]
pub enum IBody<'a> {
    /// when body type of request should be handled as
    /// general bytes or manually handled
    Bytes(&'a [u8]),
    /// handling multipart-form-data within request
    MultiPartFormData(FormDataAll),
    /// handling x-www-form-data within request
    XWWWFormUrlEncoded(XWWWFormUrlEncoded<'a>)
}

type HandlingBodyChunksWorker<M> = Pin<Box<dyn FnMut(Result<M,&str>)->dyn Future<Output=HandlingChunkResult>>>;
/// if the incoming body need to be handled as chunks
pub enum IBodyChunks<'a> {
    /// handling incoming body bytes as chunked
    Bytes(
       BytesPuller<'a>
    ),
    /// parsing incoming bytes to [MultiPartFormDataField] Fields
    /// so each field would be given to callback function as single field handler
    FormData(
        MultipartData<'a>
    ),
}

/// telling the context how we would like to handle incoming body
 pub enum ParsingBodyMechanism {
    /// letting the context chose how to handle the incoming body bytes depending on
    /// Content-Type header value
    Default,
    /// handling the incoming bytes manually
    JustBytes,
    /// parsing incoming bytes to [MultiPartFormDataField] Fields
    FormData,
    /// parse incoming body bytes to [XWWWFormUrlEncoded] struct
    XWWWFormData
 }

/// parsing body mechanism results
pub enum  ParsingBodyResults<'a> {
    /// handling the incoming body as chunks because of the size
    /// # why we need to handle body using chunks ?
    ///
    /// well the incoming body could be very long data size for example ( 4GB )
    /// and you are running your framework on a vps or low server resources
    /// so if your server ram size lower than the incoming data size you would face
    /// stack overflow error or your server would be panicked or encounter kill lag
    /// or even facing hacking attacks ( example : if the hacker sent 4 request with 4gb on ram )
    /// this would make your server down
    ///
    /// so we need to handle data as chunks to load small chunks one the context reading buffer,
    /// and then we could handle this chunks on server slightly  like saving incoming data on hard
    /// disk synchronously
    Chunked(IBodyChunks<'a>),

    /// handling the incoming data as full body bytes which is the common use case
    FullBody(IBody<'a>),

    /// when request is get for example and it`s not having any payload
    None,
    /// when could not handle body
    Err(WaterErrors<'a>)
}


impl <'a> ParsingBodyResults<'a> {

    pub async fn on_multipart_form_data_detect(
        payload:&'a[u8],
        mut on_detect: impl FnMut(
        Result<MultiPartFormDataField<'a>,&str>
    ) -> Pin<Box<dyn Future<Output=HandlingFormDataResult>>>
    )->Result<(),&str>{
        let mut index:usize = 0;
        loop {
            match MultiPartFormDataField::new(&payload[index..]) {
                None => { break;}
                Some(data) => {
                    index = index + data.field_header_length;
                    let res = on_detect(Ok(data)).await;
                    match res {
                        HandlingFormDataResult::Pass => { continue;}
                        HandlingFormDataResult::Stop => { break; }
                        HandlingFormDataResult::Shutdown => { return Err("shutdown connection")}
                    }
                }
            }
        }
        Ok(())
    }

    /// checking if parsing body has error
    pub fn is_err(&self)->bool {
        if let ParsingBodyResults::Err(_) = self { return  true}
        false
    }

    /// checking if parsing body returns a [None] Value
    pub fn is_none(&self)->bool {
        if let ParsingBodyResults::None = self { return  true}
        false
    }
}

/// telling the framework to stop handling incoming data or resume
 pub enum HandlingFormDataResult {
    /// to continue reading and parsing new form-data field
     Pass,
    /// to stop  parsing new form-data field but continue reading
     Stop,
    /// to stop reading and parsing new data
     Shutdown
 }



/// generated multipart form data field on heap
/// keep in mind that this approach will allocate new space on heap portion of ram
/// which take a little much time and considering less efficient than calling chunks function
#[derive(Debug)]
pub struct HeapFormField {
    pub content_disposition_type: ContentDispositionType,
    pub name:String,
    pub file_name:Option<String>,
    pub content_type:Option<String>,
    pub data:Vec<u8>
}

impl  HeapFormField {
    fn from(value: &MultiPartFormDataField,data:&[u8]) -> Self {
        let name = String::from_utf8_lossy(value.name).to_string().replace("\"","");
        let mut file_name = None;
        if let Some(f) = value.file_name.as_ref() {
            file_name = Some(String::from_utf8_lossy(*f).to_string());
        }

        let mut content_type = None;
        if let Some(f) = value.content_type.as_ref() {
            content_type = Some(String::from_utf8_lossy(*f).to_string());
        }
        return HeapFormField {
            name,
            file_name,
            content_type,
            content_disposition_type:value.content_disposition.clone(),
            data:data.to_vec()
        }

    }


    /// getting the data of this field
    pub fn data(&self)->&[u8]{
        self.data.as_ref()
    }

}



 /// for building incoming form data
 #[derive(Debug)]
 pub struct  FormDataAll {
     pub fields:Vec<HeapFormField>,
 }



impl  FormDataAll {
    /// for getting specific field form incoming multipart data
    pub fn get(&self,key:&str)->Option<&HeapFormField>{

        self.fields.iter().find(|c| {
            c.name == key
        })
    }

    /// initiating new [FormDataAll]
    pub  fn new()->FormDataAll{
        FormDataAll {
            fields:Vec::new(),
        }
    }




    pub (crate) fn push(& mut self,
                                           field:&
    MultiPartFormDataField,data:&[u8]){


        self.fields.push(
            HeapFormField::from(field,data)
        );
    }


}




/// telling context handler of request
/// if the body handling failed
/// so that context force stop handling request which cost resources
/// and continue handling another concurrent requests
pub enum  HandlingChunkResult<'a> {
    Consumed,
    Err(&'a str)
}