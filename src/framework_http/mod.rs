
// #![allow(dead_code)]
// #![allow(unused)]
#![allow(unused_assignments)]
pub mod request;
pub mod server_runner;
#[macro_use]
pub mod response;
pub use response::*;
pub use request::*;

#[allow(unused)]
pub use crate::configurations::WaterServerConfigurations;
mod chose_encoding_algorithm;
mod tls;
#[macro_use]
mod server_structure;
pub mod util;
pub use util::*;
use h2::{server::SendResponse,SendStream, RecvStream};
use http::{HeaderName, HeaderValue};
use serde::Serialize;
use std::{collections::HashMap, net::SocketAddr, str::FromStr, vec};
use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io::{SeekFrom, Write};
use std::path::Path;
use std::string::ToString;
use brotli::CompressorWriter;
use flate2::write::{DeflateEncoder, GzEncoder};
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::TcpStream};
use tokio::io::{AsyncSeekExt};
use nom::{AsBytes, Parser};
use nom::error::context;
use tokio_util::bytes::{Bytes, BytesMut};
use crate::framework_http::multipart_form::{HttpMultiPartFormDataField,
                                            parse_body_to_list_of_multipart_fields,
                                            parse_body_to_multipart_field_chunks
};
use crate::framework_http::x_www_form_urlencoded::XWWWFormUrlEncoded;
use crate::framework_http::chose_encoding_algorithm::HttpEncodingAlgorithms;



server_structure_generator!(->&*);



impl<DataHolderGeneric:Send> HttpContext<DataHolderGeneric> {
    server_structure_impl_context!(->!);


    /// notice that this function work with single read of tcp stream
    /// its mean if had already got the body of request
    /// ,or you had already used this function in scope before its will not work
    /// cause it`s designed for high performance
    /// so make sure to call this function one time at scope per context
    /// if you expect for users to upload multi files or single file
    /// its will work successfully with efficient speed
    pub async fn save_files_from_request_and_get_the_rest_of_fields<'a>(
        &mut self,
        rules_for_saving:&[SaveMetadataForMultipart<'a>],
    )
    ->Result<SaveForMultipartResults,String>{

        let mut rest_fields :Vec<(HttpMultiPartFormDataField,Vec<u8>)> = vec![];
        let mut saved_files :Vec<(HttpMultiPartFormDataField,Result<File,String>)> = vec![];
        let mut working_on_field:Option<HttpMultiPartFormDataField> = None;
        let mut  for_saving:Option<(&SaveMetadataForMultipart,Result<File,String>)> = None;
        let mut single_field_data = Vec::<u8>::new();
        parse_body_to_multipart_field_chunks(
            self,
         |field,data|   {
             let _  =  _save_from_request_multipart_sync_and_get_the_rest_of_fields_function_helper(
                 &mut single_field_data,
                 &mut rest_fields,
                 &mut saved_files,
                 &mut working_on_field,
                 &mut for_saving,
                 field,
                 data,
                 rules_for_saving);
             ()
         }
        ).await;
        if let Some(for_saving) = for_saving {
            if let Some(field) = working_on_field {
                saved_files.push(
                   (
                       field,
                       for_saving.1
                   )
                );
            }
        }else {
            if  !single_field_data.is_empty() {
                rest_fields.push(
                    (
                        working_on_field.unwrap(),
                        single_field_data
                    )
                );
            }
        }


       Ok(
           SaveForMultipartResults {
               saved_as_files_fields:saved_files,
               rest_fields
           }
       )
    }
}
  fn _save_from_request_multipart_sync_and_get_the_rest_of_fields_function_helper<'a>(
                single_field_data:&mut Vec<u8>,
                rest_fields:&mut Vec<(HttpMultiPartFormDataField,Vec<u8>)>,
                saved_files:&mut Vec<(HttpMultiPartFormDataField,Result<File,String>)>,
                working_on_field:&mut Option<HttpMultiPartFormDataField>,
                for_saving:&mut Option<(&'a SaveMetadataForMultipart<'a>,Result<File,String>)>,
                field:&HttpMultiPartFormDataField,
                data:&[u8],
                rules_for_saving:&'a [SaveMetadataForMultipart<'a>]
) ->Result<(),String>{
    /// now we had a field
    if let Some(wf) = working_on_field {
        if wf.get_file_name() != field.get_file_name() {
            if let Some((save_rule,file)) = for_saving {
                match file {
                    Ok(file) => {
                        if let Ok(file) = file.try_clone() {
                            saved_files.push(
                                ( wf.clone(),
                                  Ok(file)
                                )
                            );
                        }
                    }
                    Err(err) => {
                        saved_files.push(
                            ( wf.clone(),
                              Err(err.to_string())
                            )
                        );
                    }
                }
            } else{
                rest_fields.push(
                    (
                        wf.clone(),
                        single_field_data.clone()
                    )
                );
            }
            single_field_data.clear();
            *working_on_field = None;
            _save_from_request_multipart_sync_and_get_the_rest_of_fields_function_helper(single_field_data,
                                                                                         rest_fields,
                                                                                         saved_files,
                                                                                         working_on_field,for_saving,field,data,rules_for_saving);
        }
        if let Some((save_rule,file)) = for_saving {
            if let Ok(file) = file {
                file.write(data);
            } else {
                return Ok(());
            }
        } else{
            single_field_data.extend(data);
        }
        Ok(())

    }
    // if we are signing new field
    else {
        *working_on_field = Some(field.clone());
        *for_saving = None;
        for save_rule in rules_for_saving {
            if save_rule.field_name != field.get_name_key() { continue; }
            let mut file_path = save_rule.saving_path.to_string();
            let last = file_path.split("/").last();
            if let Some(last) = last{
                if !last.contains(".") {
                    if !last.ends_with("/") {file_path.push_str("/");}
                    if let Some(name) = field.get_file_name() {
                        file_path.push_str(name);
                    }
                }
            }
            let file  = File::create(&file_path);
            if let Ok(file) = file {
             *for_saving = Some((save_rule,Ok(file)));
            }
            else{
             *for_saving = Some((save_rule,Err("could not initiate file with this path".to_string())));
            }
            break;
        }
        _save_from_request_multipart_sync_and_get_the_rest_of_fields_function_helper
            (single_field_data,rest_fields,
             saved_files,
             working_on_field,for_saving,field,data,rules_for_saving)
    }
}



pub fn get_route_by_name<'a>(
    name:&str,
    options:Option<&[(&str,&str)]>)->Option<String>{
    unsafe {
        let results = crate::___ROUTERS.as_ref();
        if let Some(map) = results {
            let mut path = map.get(name);
            if let Some(_path) = path {
                let mut path = _path.to_string();
                if let Some(options) = options {
                    if !options.is_empty() {
                        for (k,v) in options.iter() {
                            let replace_pattern = format!("{{{}}}",k);
                            if !_path.contains(&replace_pattern){
                                return None;
                            }
                            path = path.replace(&replace_pattern,v);
                        }
                    }
                }
                if !path.is_empty() {
                    return Some(path);
                }
            }
        }
    }
    None
}

#[derive(Debug)]
pub struct SaveMetadataForMultipart <'a>{
    pub field_name:&'a str,
    pub saving_path:&'a str,
}

impl<'a> SaveMetadataForMultipart<'a> {
    pub fn new(name:&'a str,saving_path:&'a str)->Self {
        Self {
            field_name:name,
            saving_path
        }
    }
}

#[derive(Debug)]
pub struct SaveForMultipartResults {
    pub saved_as_files_fields:Vec<(HttpMultiPartFormDataField,Result<File,String>)>,
    pub rest_fields:Vec<(HttpMultiPartFormDataField,Vec<u8>)>,
}














