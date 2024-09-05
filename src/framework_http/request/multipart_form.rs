#![allow(unused)]
use crate::framework_http::HttpContext;
use nom::{AsChar, FindSubstring};
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
                for part_in_line in line.split(";") {
                 if part_in_line.starts_with(" filename=") {
                        let filename= part_in_line.replace(" ","").replace("filename=" ,"")
                            .replace('"',"");
                     file_name = Some(filename);
                 }
                      else  if part_in_line.starts_with(" name=") {
                          name = part_in_line.replace(" ","").replace("name=" ,"")
                              .replace('"',"");
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
                                    mut on_file_detected:impl FnMut(&HttpMultiPartFormDataField,&[u8]))->Result<(),String>
 where DataHolderGeneric:Send
{

    let boundary = context.get_request_content_boundary();
    if let Some(boundary) = boundary {
        let boundary = boundary.to_owned();
        let mut working_on_field:Option<(HttpMultiPartFormDataField)> = None;
        let mut extra_bytes = Vec::<u8>::new();

        context.body_as_chunks(move |chunk| {
            _handle_chunk(
                &mut working_on_field,
                Some(chunk),
                &boundary,
                &mut extra_bytes,
                &mut on_file_detected
            )
        }).await;

        return Ok(());
    }
    return Err("could not found boundary".to_string());
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



fn _handle_chunk<'a>(
    working_on_field:&'a mut  Option<(HttpMultiPartFormDataField)>,
    chunk:Option<&'a [u8]>,
    boundary:&'a [u8],
    extra_bytes:&'a mut Vec<u8>,
    mut on_file_detected:&mut impl FnMut(&HttpMultiPartFormDataField,&[u8])
){
    if let Some(chunk) = chunk {
        extra_bytes.extend_from_slice(chunk);
    }
    if extra_bytes.is_empty() { return; }
    if let Some((field)) = working_on_field {
        if let Some((start_new_boundary,e_nd)) = fast_finding_pattern_in_bytes(&extra_bytes,boundary){
            let written_data = &extra_bytes[..start_new_boundary-2];
            on_file_detected(field,written_data);
            *extra_bytes = (&extra_bytes[start_new_boundary..]).to_vec() ;
            if extra_bytes.ends_with(b"--\r\n"){
                if &extra_bytes[..extra_bytes.len()-b"--\r\n".len()] == boundary{
                    extra_bytes.clear();
                    return;
                }
            }
            *working_on_field = None;
            return  _handle_chunk(
                working_on_field,
                None,
                boundary,
                extra_bytes,
                on_file_detected
            );
        }
        else if extra_bytes.len() > boundary.len() {
            if let Some((index,_)) =
                fast_finding_pattern_in_bytes(&extra_bytes[(extra_bytes.len()-boundary.len())..]
                                              ,&boundary[..2]){
                let suspected_bytes = &extra_bytes[index..];
                on_file_detected(field,&extra_bytes[..index]);
                *extra_bytes = suspected_bytes.to_vec();
                return  _handle_chunk(
                    working_on_field,
                    None,
                    boundary,
                    extra_bytes,
                    on_file_detected
                );
            }
        }
        else {

            on_file_detected(field,&extra_bytes);
            extra_bytes.clear();
        }
    }else{
        // find the headers by finding boundary and \r\n\r\n
        let first_boundary = fast_finding_pattern_in_bytes(&extra_bytes,boundary);
        if let Some((_,end_of_boundary_index))= first_boundary {
            // let us find the end of multipart data headers which it \r\n\r\n
            if let Some((_,end_of_headers_index)) = fast_finding_pattern_in_bytes(&extra_bytes,b"\r\n\r\n") {
                let mut headers_bytes = &extra_bytes[end_of_boundary_index+2..=end_of_headers_index];
                let headers_string = String::from_utf8_lossy(headers_bytes);
                let field = HttpMultiPartFormDataField::from_string(
                 &headers_string
                );
                *extra_bytes = (&extra_bytes[end_of_headers_index+1..]).to_vec();
                if let Ok(field) = field {
                    *working_on_field = Some((field));
                    return  _handle_chunk(
                        working_on_field,
                        None,
                        boundary,
                        extra_bytes,
                        on_file_detected
                    );
                }
            }
        }
    }

}

use super::super::fast_finding_pattern_in_bytes;