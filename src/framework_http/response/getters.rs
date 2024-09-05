 macro_rules! include_response_getters_functions {
    () => {



    pub fn get_route_path(&self)->&str{
        return match &self.protocol {
            Protocol::Http1(h1) => {
                return h1.request.path.as_str()
            }
            Protocol::Http2(h2) => {
                h2.request.uri().path()
            }
        };
    }
    pub fn get_method(&self)->&str{
        return match &self.protocol {
            Protocol::Http1(h1) => {
                return h1.request.method.as_str();
            }
            Protocol::Http2(h2) => {
                h2.request.method().as_str()
            }
        };
    }

    fn get_request_content_boundary(&mut self)->Option<&[u8]>{
        let _r = self.get_from_headers("Content-Type");
        const PATTERN :&str = "boundary=";
        if let Some(ve) = _r {
            for i in ve {
                for value in i {
                    if !value.contains(PATTERN){
                        continue;
                    }
                    let splitter = value.split(PATTERN).last();
                    if let Some(_boundary) = splitter { return  Some(_boundary.as_bytes());}
                }
            }
        }
        None
    }
    pub fn get_all_headers(&mut self )->&HttpHeadersMap{
        match &mut self.protocol {
            Protocol::Http1(h1) => {
                return &h1.request.headers_map;
            }
            Protocol::Http2(h2) => {
                {
                    let all_keys  = h2.request.headers();
                    for (k,value) in all_keys {
                        let key = k.to_string();
                        if h2.cached_headers.contains_key(key.as_str()){
                            continue;
                        }
                        let res = convert_string_value_to_vec_of_strings(
                            &String::from_utf8_lossy(value.as_bytes())
                        );
                        h2.cached_headers.insert(key,res);
                    }
                };
                &h2.cached_headers
            }
        }
    }

    pub fn get_from_headers(&mut self,key:&str)->Option<&Vec<Vec<String>>>{
         match &mut self.protocol {
            Protocol::Http1(_p1) => {
                return _p1.request.headers_map.get(key);
            },
            Protocol::Http2(_p2) => {
                let y = _p2.cached_headers.contains_key(key);
                if y {
                    return _p2.cached_headers.get(key);
                }else{
                  Self::from_h2_protocol_to_valid_headers_map(key,_p2)
                }
            },
            }
    }

    pub fn get_from_headers_as_string(&mut self,key:&str)->Option<&String>{
        if let Some(c) = self.get_from_headers(key) {
            if !c.is_empty() {
                let first = c.first().unwrap().first();
                if let Some(content_length_string) = first {
                   return Some(content_length_string);
                }
            }
        }

        None
    }
    pub fn get_from_header_as<T:FromStr>(&mut self,key:&str)->Option<T>{
        if let Some(value) = self.get_from_headers_as_string(key) {
            if let Ok ( v ) = value.parse::<T>() {
                return Some(v);
            }
        }

        None
    }



    pub async fn body_as_chunks(&mut self,mut bytes_chunk: impl FnMut (&[u8]))->Result<(),String>{
        match &mut self.protocol {
            Protocol::Http1(p) => {
                let mut total_bytes_received = 0;

                if !p._extra_bytes_from_headers.is_empty() {
                    total_bytes_received += p._extra_bytes_from_headers.len();
                    bytes_chunk(&p._extra_bytes_from_headers);

                }
                if let Some(value) = p.request.headers_map.get("Content-Length") {
                    if !value.is_empty() && !value[0].is_empty(){
                      let length = (&value[0][0]).parse::<usize>();
                      if let Ok(length) = length {
                     if total_bytes_received >= length { return Ok(());}
                       let mut buf = BytesMut::with_capacity(SIZE_OF_READ_WRITE_CHUNK);
                       match &mut p._peer.0 {
                           WaterTcpStream::Tls(stream) => {
                               'label: while let Ok(_c) = stream.read_buf(&mut buf).await {
                                   total_bytes_received += _c;
                                   bytes_chunk(&buf[.._c]);
                                   if _c == 0 || total_bytes_received >= length  {
                                       break 'label;
                                   }
                                   buf.clear();
                               }
                           }
                           WaterTcpStream::Stream(stream) => {
                               while let Ok(_c) = stream.read_buf(&mut buf).await {

                                   total_bytes_received += _c;

                                   bytes_chunk(&buf[.._c]);
                                   if _c == 0 || total_bytes_received >= length  {
                                       break;
                                   }
                                   buf.clear();
                               }
                           }
                       }
                      } else {
                        return Err("Can not Read Content Length from Http Request".to_string());
                      }

                    }
                }
            },
            Protocol::Http2(ref mut h2) => {
               let  body = h2.request.body_mut();
                if let Some(body) = body.data().await {
                    if let Ok(bytes) = body {
                        bytes_chunk(&bytes);
                    }
                }
            },
        }
        Ok(())
    }

    pub async fn body_as_string(&mut self)->Result<Option<String>,String>{
        match self.whole_body_as_bytes().await {
            Ok(bytes) => {
                if let Some(bytes) = bytes {
                    if let Ok(_data) = String::from_utf8(bytes) {
                        return Ok(Some(_data));
                    }
                }
                return Err("Cant Creating String from this Body".to_string());
            },
            Err(_e) => Err(_e),
        }
    }
    pub async fn whole_body_as_bytes(&mut self)->Result<Option<Vec<u8>>,String>{
        let content_length: Option<usize> = self.get_from_header_as::<usize>("Content-Length");
        match &mut self.protocol {
            Protocol::Http1(p1) => {
                let mut bytes:Vec<u8> = vec![];
                bytes.append(&mut p1._extra_bytes_from_headers);
                if let Some(content_length) = content_length {
                    if bytes.len() >= content_length {
                        return Ok(Some(bytes));
                    }
                }else {
                    return Ok(None);
                }
                let mut buf = BytesMut::with_capacity(SIZE_OF_READ_WRITE_CHUNK);

                match &mut p1._peer.0 {
                    WaterTcpStream::Tls(stream) => {
                        loop {
                            match stream.read_buf(&mut buf).await {
                                Ok(_s) => {
                                    if _s == 0 {
                                        return Ok(Some(bytes));
                                    }
                                    bytes.extend(&buf);

                                    if _s < buf.capacity() {
                                        return Ok(Some(bytes));
                                    }
                                    buf.clear();
                                }
                                Err(_) => {
                                    return Err("There An Error Happen While Reading Bytes From Tls\
                                     Stream".to_string());
                                }
                            }
                        }
                    }
                    WaterTcpStream::Stream(stream) => {
                        loop {
                            if let Ok(_s) = stream.read_buf(&mut buf).await {
                                if  _s <= 0 {
                                    return  Ok(Some(bytes));
                                }
                                bytes.extend(&buf);

                                if _s < buf.capacity()   {
                                    return  Ok(Some(bytes));
                                }
                                buf.clear();
                            }else {
                                return  Err("There An Error Happen While Reading Bytes From Stream".to_string());
                            }
                        }
                    }
                }

            }
            Protocol::Http2(_p2) => {
                let _body =  _p2.request.body_mut().data().await;
                if let None = _body {
                    return Ok(None);
                }
                else if let Some(body) = _body {
                    return if let Ok(body) = body {
                        Ok(Some(body.into()))
                    } else {
                        Err("Error reading body bytes".to_string())
                    }
                }
            },
            }
            Err("can not handle whole body as bytes".to_owned())
        }
    pub async fn get_from_path_query_as_cow(&mut self,key:&str)->Option<std::borrow::Cow<str>>{
        let res = self.get_from_path_query(key);
        if let Some(res) = res {
            return Some(String::from_utf8_lossy(res));
        }
        None
    }
    pub async fn get_from_path_query_as_string(&mut self,key:&str)->Option<String>{
        let res = self.get_from_path_query_as_cow(key).await;
        if let Some(res) = res {
            return  Some(res.to_string());
        }
        None
    }
    pub fn      get_from_path_query(&self,key:&str)->Option<&[u8]> {
        return  match self.protocol {
            Protocol::Http1(ref h1) => {
                let res = h1.request.headers_query.get(key);
                if let Some(res) = res {
                    return Some(res.as_bytes());
                }
                None
            }
            Protocol::Http2(ref h2) => {
                if let Some(data) = h2.path_query.get(key){
                    return Some(data.as_bytes());
                }
                None
            }
        };
    }
    pub async fn get_from_body_as_cow(&mut self,key:&str)->Option<std::borrow::Cow<str>>{
        if let Some(res) = self.get_from_body(key).await {
            return Some(String::from_utf8_lossy(res));
        }
        None
    }
    pub async fn get_from_body_as_string(&mut self,key:&str)->Option<String>{
        if let Some(res) = self.get_from_body_as_cow(key).await {
            return Some(res.to_string());
        }
        None
    }

    /// you could use this method to retrive data from GET OR POST method the data has been passed to the
    /// server
    pub async fn get_from_all_params<'a>(&'a mut self,key:&str)->Option<&'a [u8]>{
        if &self.get_method().to_lowercase() == "get" {
            return self.get_from_path_query(key);
        }
        self.get_from_body(key).await
    }
    pub async fn get_from_body(&mut self,key:&str)->Option<&[u8]>{
        let body = self.get_body().await;
        return  match body {
            None => {
                None
            }
            Some(body) => {
                match body {
                    HttpIncomeBody::MultiPartFormat(body) => {
                        for (body,bytes) in body {
                             if body.get_name_key() == key {
                                 return Some(bytes);
                            }
                        }
                        None
                    }
                    HttpIncomeBody::XWWWForm(body) => {
                       let body =  body.data.get(key);
                        if let Some(body) = body {
                            return  Some(body.as_bytes());
                        }
                        None
                    }

                    _ => {
                        None
                    }
                }
            }
        }
    }
     pub async fn get_body<'a>(&'a mut self )-> &'a Option<HttpIncomeBody>{
         match (self).body {
             None => {}
             Some(ref body) => {
                 return body;
             }
         };
         let body = self.serialized_body().await;
         return match body {
             None => {
                 self.body = Some(None);
                  &None
             }
             Some(body) => {
                 self.body = Some(Some(body));
                   self.body.as_ref().unwrap()
             }
         }
     }
     async fn serialized_body(&mut self)->Option<HttpIncomeBody>{
        let content_type = self.get_from_header_as::<String>("Content-Type");
        if let Some(mut content_type) = content_type {
            content_type = content_type.to_lowercase();
            if content_type == "multipart/form-data" {
                let fields = parse_body_to_list_of_multipart_fields(self).await;
                return  HttpIncomeBody::MultiPartFormat(fields).into();
            }
            else  if content_type == "application/json" {
                let body = self.whole_body_as_bytes().await;
                if let Ok(Some(body)) = body {
                    return  Some(HttpIncomeBody::Json(body));
                }
                return  None;
            }
            else if content_type == "application/x-www-form-urlencoded" {
                let body = self.whole_body_as_bytes().await;
                if let Ok(body) = body {
                    if let Some(body) = body {
                        let x_body = XWWWFormUrlEncoded::from_str(
                            &String::from_utf8_lossy(&body)
                        );
                        if let Ok(x_body)  = x_body{
                            return  HttpIncomeBody::XWWWForm(x_body).into();
                        }
                        return  HttpIncomeBody::Unit8Vec(body).into();
                    }
                }
            }
        }
        None
    }
    };
}