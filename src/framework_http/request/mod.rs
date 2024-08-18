mod cookies;
pub mod multipart_form;
pub mod x_www_form_urlencoded;

use std::collections::HashMap;

pub type HttpHeadersMap = HashMap<String,Vec<Vec<String>>>;
#[derive(Debug)]
pub struct Request {
    pub method:String,
    pub path:String,
    pub http_version:String,
    pub headers_map:HttpHeadersMap,
    
}


impl Request {
    pub fn build_request(string_header:String,important_headers:Option<Vec<&str>>)->Result<Request,String>{
        let mut lines = string_header.lines();
        let mut request = Request {
            method:"".to_string(),
            path:"".to_string(),
            http_version:"".to_string(),
            headers_map:HashMap::new(),
        };
        if let Some(first_line) = lines.next(){
            let _e = Err("Request Error".to_string());
            if let Some(method) = first_line.find(" ") {
                request.method = first_line[..method].to_string();
                let rest = &first_line[method..].trim();
                if let Some(_path) = rest.find(" ") {
                    let path =  &rest[.._path];
                    request.path = path.to_string();
                    request.http_version = rest[_path+1..rest.len()].to_string();
                } else { return _e;}
            } else { return _e;}
        }

         let never_filter_headers = important_headers.is_none();
         while let Some(_line) = lines.next() {
            let mut splitter : Vec<&str> = _line.split(": ").collect();
            if splitter.len() < 2 {
                continue;
            }
            let key = splitter.remove(0);
             if !never_filter_headers  {
                 if let Some(important_headers) = &important_headers {
                     if !important_headers.contains(&key) {
                         continue;
                     }
                 }
             }

            let  values:Vec<Vec<String>> = 
            convert_string_value_to_vec_of_strings(splitter.first().unwrap());
            let _ = &request.headers_map.insert(key.to_string(),values);    
         }
        Ok(request)
    }
}

pub fn convert_string_value_to_vec_of_strings(slice:&str)->Vec<Vec<String>> {
    let mut values:Vec<Vec<String>> = vec![];
                let internal_splitter = slice.split(";");
                for lvi in internal_splitter {
                    let mut internal_values :Vec<String>= vec![];
                    let last_interval =  lvi.split(",");
                    for v in last_interval {
                        internal_values.push(v.to_string());
                    }
                    values.push(internal_values);
                }
                values
}
