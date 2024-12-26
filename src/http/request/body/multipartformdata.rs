
/// when ever incoming request has body of type multipartForm-data
#[derive(Debug)]
 pub struct MultiPartFormDataField<'a> {
   /// detecting Content-Disposition head
   pub content_disposition:ContentDispositionType,
   /// holding the name of content-disposition
  pub name:&'a [u8],
   /// holding file name if exist
   pub file_name:Option<&'a [u8]>,
   /// holding content-type of disposition if exist
   pub content_type:Option<&'a[u8]>,
   /// determining the main length of field headers
   pub field_header_length:usize
}




impl <'a> MultiPartFormDataField<'a> {

    /// checking if incoming field is a file or not
    /// by checking file name property
    /// you could check also content-type manually by using [self.content_type]
    pub fn is_file(&self)->bool{ self.file_name.is_some() }

    /// for getting content type of field [MultiPartFormDataField]
    pub fn content_type(&self)->Option<&'a [u8]>{self.content_type}

    #[allow(unused_assignments)]
    pub (crate) fn new(payload:&'a[u8])->Option<MultiPartFormDataField<'a>>{
        let mut key:Option<&'a[u8]> = None;
        let mut disposition_key:Option<&'a[u8]> = None;
        let mut name = None;
        let mut disposition_type = ContentDispositionType::FormData;
        let mut file_name = None;
        let mut content_type = None;
        let mut end_counter:usize = 0;
        let mut start:usize = 0;
        let mut disposition_indexes_start = false;
        let  payload_length = payload.len();
        for (index,bytes) in payload.iter().enumerate() {

            match bytes {
                b'\r' => {
                    end_counter+=1;
                    if disposition_indexes_start {
                        disposition_indexes_start = false;
                        if let Some(key) = disposition_key {
                            match key {
                                b"name"=>{ name = Some(&payload[start..index]);
                                    inc_start_pointer!(start,index,payload_length);
                                    disposition_key = None;
                                }
                                b"filename"=>{
                                    file_name = Some(&payload[start..index]);
                                    inc_start_pointer!(start,index,payload_length);
                                    disposition_key = None;
                                }
                                _=>{disposition_key = None;}
                            }
                        }
                    }
                    else if let Some(key) = key {
                        if key == b"Content-Type" {
                            content_type = Some(&payload[start..index]);
                        }
                    }
                    inc_start_pointer!(start,index,payload_length);
                }
                b'\n' => {
                    end_counter+=1;
                    if end_counter >=3 {
                        start = index+1;
                        break;
                    }
                    inc_start_pointer!(start,index,payload_length);
                }
                b' '=> {
                    end_counter = 0 ;
                    if key.is_some() && start == index {
                        inc_start_pointer!(start,index,payload_length);
                    }
                }
                b';'=> {
                    end_counter = 0 ;
                    if disposition_indexes_start {
                        if let Some(key) = disposition_key {
                            match key {
                                b"name"=>{ name = Some(&payload[start..index]);
                                    inc_start_pointer!(start,index,payload_length);
                                },
                                b"filename"=>{
                                    file_name = Some(&payload[start..index]);
                                    inc_start_pointer!(start,index,payload_length);
                                },
                                _ => {}
                            }
                        }
                        else {
                            let data = &payload[start..index];
                            match data {
                                b"inline"=>{
                                    disposition_type = ContentDispositionType::Inline;
                                },

                                b"attachment"=>{
                                    disposition_type = ContentDispositionType::Attachment;
                                },
                                _ => {}
                            }
                            inc_start_pointer!(start,index,payload_length);
                        }
                    }
                }
                b'='=> {
                    end_counter = 0 ;
                    if disposition_indexes_start {
                        disposition_key = Some(&payload[start..index]);
                        inc_start_pointer!(start,index,payload_length);
                    }
                }
                b':'=>{
                    end_counter = 0 ;
                    let kd = &payload[start..index];
                    if !disposition_indexes_start && kd == b"Content-Disposition"  {
                        disposition_indexes_start = true;
                    }
                    key = Some(kd);
                    inc_start_pointer!(start,index,payload_length);
                }
                _ =>{
                    end_counter = 0 ;
                }
            };
        }
        if let Some(name) = name {
             return Some(MultiPartFormDataField {
                 content_disposition: disposition_type,
                 name,
                 file_name,
                 field_header_length:start,
                 content_type
             });
        }
        None
    }

    /// for cloning [MultiPartFormDataField]
    pub fn clone<'b>(& self,data:&'b mut Vec<u8>) -> MultiPartFormDataField<'b> {
        let index = data.len();
        data.extend_from_slice(self.name);
        let name = (index,data.len());
        let mut file_name = None;
        let mut content_type = None;
        if let Some(f_name) = self.file_name {
            let index = data.len();
            data.extend_from_slice(f_name);
            file_name = Some((index,data.len()));
        }

        if let Some(c_t) = self.content_type {
            let index = data.len();
            data.extend_from_slice(c_t);
            content_type = Some((index,data.len()));
        }

        let name = &data[name.0..name.1];
        let mut f_name = None;
        if let Some(file_name) = file_name {
            f_name = Some(&data[file_name.0..file_name.1]);
        }

        let mut c_disposition = None;
        if let Some(content_type) = content_type {
            c_disposition = Some(&data[content_type.0..content_type.1]);
        }
        return MultiPartFormDataField {
            name,
            file_name:f_name,
            content_type:c_disposition,
            content_disposition:self.content_disposition.clone(),
            field_header_length:self.field_header_length
        }

    }
}


use crate::http::request::inc_start_pointer;




/// for specifying Content-Disposition if the incoming request
/// and als FormData type is the most used one
#[derive(Debug,Clone)]
pub enum ContentDispositionType {
 /// indicating that this content could be use inside web page
 Inline,
 /// indicating that this content meant to be downloaded and saved locally
 Attachment,
 /// indicating that this content Treated as Form Content Holding Data
 FormData
}