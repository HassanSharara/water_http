#![allow(unused)]
use crate::framework_http::HttpContext;
use nom::FindSubstring;
#[derive(Debug,Clone)]
pub struct  HttpMultiPartFormDataField {
    file_name:Option<String>,
    name:String,
    content_type:Option<String>,
}

impl  HttpMultiPartFormDataField {

    pub fn from_string(st:&str)->Result<Self,String>{
        let mut name = String::new();
        let mut file_name :Option<String>= None;
        let mut content_type = None;
        for line in st.lines() {
            if !name.is_empty() && ! file_name.as_ref().unwrap_or(&"".to_string()).is_empty() {
                if let Some(_) = content_type {
                    break;
                }
            }
            if line.contains("Content-Disposition"){
                if let Some(last) = line.split("name=").last() {
                    if let Some(n) = last.split(";").next(){
                        if let Some(n) = n.split("\r\n").next() {
                            name = String::from(n.replace('"',""));
                        }
                    }
                }
                if line.contains("filename=") {
                    if let Some(last) = line.split("filename=").last() {
                        if let Some(n) = last.split(";").next(){
                            if let Some(n) = n.split("\r\n").next() {
                                file_name = Some(String::from(n.replace('"',"")));
                            }
                        }
                    }
                }
            }
            else if line.contains("Content-Type"){
                if let Some(t)  = line.split("Content-Type: ").last(){
                    content_type = Some(String::from(t));
                }
            }
        }
        if !name.is_empty()  {
            return  Ok(HttpMultiPartFormDataField {
                file_name,
                name,
                content_type
            });
        }
        Err("Fail to Build HttpRFile".to_string())
    }
    pub fn get_file_name(&self)->Option<&String>{
        return self.file_name.as_ref();
    }

    pub fn get_name_key(&self)->&str{
        return &self.name;
    }
}


pub async  fn parse_body_to_list_of_multipart_fields<DataHolderGeneric:Send>(
    context:&mut HttpContext<DataHolderGeneric>)->Vec<(HttpMultiPartFormDataField,Vec<u8>)>

  where DataHolderGeneric : Send{
    let mut body_files = Vec::<(HttpMultiPartFormDataField,Vec<u8>)>::new();
    let mut r_field:Option<HttpMultiPartFormDataField> = None ;
    let mut data = Vec::<u8>::new();
    let _ =  parse_body_to_multipart_field_chunks(context,|field,chunk| {
        if let Some(_rf) = r_field.as_ref() {
            if field.get_name_key() != _rf.get_name_key() {
                body_files.push((_rf.clone(),data.clone()));
                r_field = Some(field.clone());
                data.clear();
            }
        } else {
            r_field = Some(field.clone());
        }
       data.extend_from_slice(chunk);
    }).await;
    if data.len() > 2 && data.ends_with(b"\r\n") {
        if let Some(field) = r_field {
            body_files.push((field.clone(),(&data[..(data.len() - 2)]).to_vec()));
        }
    }
    body_files
}
pub async fn parse_body_to_multipart_field_chunks
<DataHolderGeneric>(context:&mut HttpContext<DataHolderGeneric>,
                                    mut on_file_detected:impl FnMut(&HttpMultiPartFormDataField,&[u8]))
                                    ->Result<(),String>
 where DataHolderGeneric:Send
{
    let _bound = context.get_request_content_boundary();
    return if let Some(bou) = _bound {
        let boundary = bou.to_vec();
        let mut headers_vec = Vec::<u8>::new();
        let mut data = Vec::<u8>::new();
        let mut extra_bytes: Vec<u8> = vec![];
        let mut field_option: Option<HttpMultiPartFormDataField> = None;
        let _ = context.body_as_chunks(|chunk| {
            extra_bytes.extend_from_slice(chunk);
            while !extra_bytes.is_empty() {

                let casting_res = multipart_bytes_to(&extra_bytes, &boundary, &mut headers_vec, &mut data);
                if let Some(eb) = casting_res {
                    extra_bytes = eb.to_vec();
                } else {
                    extra_bytes.clear();
                }
                if headers_vec.is_empty() && data.is_empty() {
                    break;
                }
                // checking if field is exist
                if let Some(field) = field_option.as_ref() {
                    on_file_detected(field, &data);
                } else if !headers_vec.is_empty() {
                    let field = HttpMultiPartFormDataField::from_string(String::from_utf8_lossy(&headers_vec).as_ref());
                    if let Ok(field) = field {
                        on_file_detected(&field,&data);
                        field_option = Some(field);
                    }
                }
                if data.ends_with(b"\r\n") {
                    headers_vec.clear();
                    data.clear();
                    field_option = None;
                }
            }
        }).await;
        Ok(())
    } else {
        Err("invalid request".to_string())
    }

}




fn multipart_bytes_to<'a>(
    mut chunk:&'a[u8],
    boundary:&[u8],
 old_headers:&mut Vec<u8>,
 old_data:&mut Vec<u8>
)->Option<&'a [u8]>{
    if old_headers.is_empty() {
        if let Some(boundary_index) = chunk.windows(boundary.len()).position(|w| w==boundary) {
            let headers_start_chunk:&[u8] = &chunk[boundary_index+boundary.len()+2..];
            if let Some(headers_end) = headers_start_chunk.find_substring("\r\n\r\n"){
                old_headers.extend_from_slice(&headers_start_chunk[..headers_end]);
                chunk = &headers_start_chunk[headers_end+4..];
            }

        }
    }
    if old_headers.is_empty() { return  Some(chunk); }
    if !chunk.is_empty(){

        if let Some(_end_of_data) = chunk.windows(boundary.len()).position(|w| w == boundary) {

            old_data.extend_from_slice(&chunk[.._end_of_data-
                 if _end_of_data > 2 {
                       2
                 } else { 0 }
                ]);
            chunk = &chunk[_end_of_data..];
            if chunk.is_empty() {
                return  None;
            }
        }
        return Some(chunk);
    }
    None
}
