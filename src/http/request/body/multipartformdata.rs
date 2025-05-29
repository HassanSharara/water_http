
/// when ever incoming request has body of type multipartForm-data
#[derive(Debug)]
 pub struct MultiPartFormDataField<'a> {
   pub headers:KeyValueList<'a,12>,
   /// determining the main length of field headers
   pub field_header_length:usize
}




impl <'a> MultiPartFormDataField<'a> {

    /// checking if incoming field is a file or not
    /// by checking file name property
    /// you could check also content-type manually by using [self.content_type]
    pub fn is_file(&self)->bool{
        if let Some(v) = self.headers.get_as_header_value("Content-Disposition") {
            return v.get_from_values_as_bytes("filename").is_some();
        }
        false
    }


    /// getting content_disposition name
    pub fn content_disposition_name(&self)->Option<Cow<str>>{
        if let Some(ref v) = self.headers.get_as_header_value("Content-Disposition") {
            if let Some(v) = v.get_from_values_as_str("name") {
                return Some(Cow::Owned(v.to_string()))
            }
        }
        None
    }

    /// for getting content type of field [MultiPartFormDataField]
    pub fn content_type(&self)->Option<&'a [u8]>{
        self.headers.get_as_bytes("Content-Type")
    }


    pub (crate) fn new(payload:&'a[u8])->Option<MultiPartFormDataField<'a> >{
        let key_list = KeyValueList::<12>::try_parse(payload);
        if let Some((list,length)) = key_list {
            return Some(MultiPartFormDataField{
                field_header_length:length,
                headers:list,
            })
        }
        None
    }
    // #[allow(unused_assignments)]
    // pub (crate) fn new(payload:&'a[u8])->Option<MultiPartFormDataField<'a>>{
    //     let mut key:Option<&'a[u8]> = None;
    //     let mut disposition_key:Option<&'a[u8]> = None;
    //     let mut name = None;
    //     let mut disposition_type = ContentDispositionType::FormData;
    //     let mut file_name = None;
    //     let mut content_type = None;
    //     let mut end_counter:usize = 0;
    //     let mut start:usize = 0;
    //     let mut disposition_indexes_start = false;
    //     let  payload_length = payload.len();
    //     for (index,bytes) in payload.iter().enumerate() {
    //
    //         match bytes {
    //             b'\r' => {
    //                 end_counter+=1;
    //                 if disposition_indexes_start {
    //                     disposition_indexes_start = false;
    //                     if let Some(key) = disposition_key {
    //                         match key {
    //                             b"name"=>{ name = Some(&payload[start..index]);
    //                                 inc_start_pointer!(start,index,payload_length);
    //                                 disposition_key = None;
    //                             }
    //                             b"filename"=>{
    //                                 file_name = Some(&payload[start..index]);
    //                                 inc_start_pointer!(start,index,payload_length);
    //                                 disposition_key = None;
    //                             }
    //                             _=>{disposition_key = None;}
    //                         }
    //                     }
    //                 }
    //                 else if let Some(key) = key {
    //                     if key == b"Content-Type" {
    //                         content_type = Some(&payload[start..index]);
    //                     }
    //                 }
    //                 inc_start_pointer!(start,index,payload_length);
    //             }
    //             b'\n' => {
    //                 end_counter+=1;
    //                 if end_counter >=3 {
    //                     start = index+1;
    //                     break;
    //                 }
    //                 inc_start_pointer!(start,index,payload_length);
    //             }
    //             b' '=> {
    //                 end_counter = 0 ;
    //                 if key.is_some() && start == index {
    //                     inc_start_pointer!(start,index,payload_length);
    //                 }
    //             }
    //             b';'=> {
    //                 end_counter = 0 ;
    //                 if disposition_indexes_start {
    //                     if let Some(key) = disposition_key {
    //                         match key {
    //                             b"name"=>{ name = Some(&payload[start..index]);
    //                                 inc_start_pointer!(start,index,payload_length);
    //                             },
    //                             b"filename"=>{
    //                                 file_name = Some(&payload[start..index]);
    //                                 inc_start_pointer!(start,index,payload_length);
    //                             },
    //                             _ => {}
    //                         }
    //                     }
    //                     else {
    //                         let data = &payload[start..index];
    //                         match data {
    //                             b"inline"=>{
    //                                 disposition_type = ContentDispositionType::Inline;
    //                             },
    //
    //                             b"attachment"=>{
    //                                 disposition_type = ContentDispositionType::Attachment;
    //                             },
    //                             _ => {}
    //                         }
    //                         inc_start_pointer!(start,index,payload_length);
    //                     }
    //                 }
    //             }
    //             b'='=> {
    //                 end_counter = 0 ;
    //                 if disposition_indexes_start {
    //                     disposition_key = Some(&payload[start..index]);
    //                     inc_start_pointer!(start,index,payload_length);
    //                 }
    //             }
    //             b':'=>{
    //                 end_counter = 0 ;
    //                 let kd = &payload[start..index];
    //                 if !disposition_indexes_start && kd == b"Content-Disposition"  {
    //                     disposition_indexes_start = true;
    //                 }
    //                 key = Some(kd);
    //                 inc_start_pointer!(start,index,payload_length);
    //             }
    //             _ =>{
    //                 end_counter = 0 ;
    //             }
    //         };
    //     }
    //     if let Some(name) = name {
    //          return Some(MultiPartFormDataField {
    //              content_disposition: disposition_type,
    //              name,
    //              file_name,
    //              field_header_length:start,
    //              content_type
    //          });
    //     }
    //     None
    // }


}


use std::borrow::Cow;
use crate::http::request::KeyValueList;



