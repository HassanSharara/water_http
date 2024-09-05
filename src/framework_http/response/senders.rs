use crate::HttpResponseHeaders;
macro_rules! include_response_senders_functions {
     () => {

    /// for redirecting request to another path by using
    /// routers names
    ///
    /// if you need to know how to set name for each router you could use this
    /// GET_make_order => / => ....
    ///
    /// in this request route name is ["make_order"] which came after GET_ or POST_
    ///
    /// and also you can use this function [get_route_by_name] to get your route
    ///
    pub async fn redirect_by_route_name(&mut self,key:&str,options:Option<&[(&str,&str)]>)->Result<(),String>{
        let path = get_route_by_name(key,options);
        if let Some(_path) = path {
            self.redirect_to_by_url(&_path).await?;
        }
        Err(format!("could not found this route name: {key}"))
    }

    /// for redirecting request to another path
    pub async fn redirect_to_by_url(&mut self,url:&str)->Result<(),String>{
         let mut headers = HttpResponseHeaders::found_redirect_header(url);
         let data = b" ";
         headers.set_header_key_value("Content-Length",data.len());
         self.send_headers(headers).await?;
         self.send_data(data,true).await?;
        Ok(())
    }

    pub async fn send_string_data(&mut self,slice:String,end_of_stream:bool)->Result<(),String>{
        self.send_data(slice.as_bytes(), end_of_stream).await?;
        return Ok(());
    }


    /// Sends a string slice [&str] as a response.
    /// Note that the second parameter, `end_of_stream`, indicates whether this
    /// is the last response packet for the request.
    /// If it is, the system will flush all the data, clear any necessary
    /// data from RAM, and close unnecessary connections if needed.
    pub async fn send_str_data(&mut self,slice:&str,end_of_stream:bool)->Result<(),String>{
        self.send_data(slice.as_bytes(), end_of_stream).await?;
        return Ok(());
    }


    /// for sending json response that depends on [serde]
    /// forexample each struct that derive [#[derive(Serialize,Deserialze)]]
    pub async fn send_json_data<T>(&mut self,value:&T,end_of_stream:bool)->Result<(),String> where T: ?Sized + Serialize {
        let v = serde_json::to_string(value);
        if let Ok(_v) = v {
            return self.send_data(_v.as_bytes(), end_of_stream).await;
        }
        Err("Can not convert json data to string".to_owned())
    }
    /// let`s say that you want to return file as response and this file stored
    /// in public path
    /// so you do not need to provide the path of the whole file
    /// you just need to provide the path since of public directory path
    /// for exampel
    ///
    /// if your file have this path ["public/images/customer1_profile.jpg"]
    /// then you could use this function [send_file_from_public_resources("images/customer1_profile.jpg")]
    pub async fn send_file_from_public_resources(&mut self,path:&str)->Result<(),String>{
        let public_path = unsafe {&___SERVER_CONFIGURATIONS.as_ref().unwrap().public_files_path};
        let path = format!("{public_path}/{path}").replace("//","/");
        self.send_file_as_response(&path).await
    }


    /// for sending any file in any directory of the system as response
    ///
    pub async fn send_file_as_response(&mut self,path:&str)
     ->Result<(),String>{
        let file_path = Path::new(path);
        if !file_path .exists() {
            let mut headers = HttpResponseHeaders::not_found_headers();
            let msg = b"the path is not satisfied ! ";
            headers.set_header_key_value(
                "Content-Type",
                "text/plain"
            );
            headers.set_header_key_value("Content-Length",msg.len());
            let _ = self.send_headers(headers).await;
            let _  = self.send_data(msg.as_bytes(),true).await;
        }
        let content_type = content_type_from_file_path(&file_path);
        let mut h = None;
        match content_type {
            None => {
                h = Some(HttpResponseHeaders::bad_request_headers());
                let  headers = h.as_mut().unwrap();
                headers.set_header_key_value("Content-Type","application/octet-stream");
                headers.set_header_key_value("Accept-Ranges","bytes");
                headers.set_header_key_value("Content-Disposition",
                                             format!("attachment; filename={:?}"
                                                     ,file_path.file_name().unwrap_or(
                                                     &OsStr::new(
                                                         &format!("file_downloaded.{:?}",
                                                                  file_path.extension().unwrap_or(
                                                                      &OsString::new()
                                                                  )
                                                         )
                                                     )
                                                 ))

                );
            }
            Some(content_type) => {
                h = Some(HttpResponseHeaders::success());
                let  headers = h.as_mut().unwrap();
                headers.set_header_key_value("Content-Type",content_type);
            }
        }

        let  file = tokio::fs::File::open(path).await;
        if let Ok(mut file) = file {
            let metadata = file.metadata().await;
            let mut file_size = 0_u64;

            // checking over file size
            if let Ok(metadata) = metadata {
                file_size = metadata.len();
            }
            else { return  Err("could not read total file size from file metadata".to_string()) ; }



            if let Some(headers) = h.as_mut() {
                let income_range = self.get_from_header_as::<String>("Range");
                return match income_range {
                    None => {
                       headers.set_header_key_value("Content-Length", file_size);
                        self.send_headers(h.unwrap()).await?;
                        let mut buffer = [0; 4000];
                        while let Ok(size) = file.read(&mut buffer).await {
                            if size == 0 {
                                return  Ok(());
                            }
                            if size < buffer.len() {
                                return self.send_data(&buffer[..size], true).await;
                            }
                            self.send_data(&buffer[..size], false).await?;
                        }
                        Err("encounter error while sending file ".to_string())
                    }
                    Some(range) => {
                        let mut ranges = range.split(",").next().unwrap_or("")
                            .split("=").last().unwrap_or("").split("-");
                        let start = ranges.next().unwrap_or("").parse::<u64>().unwrap_or(0);
                        let end = ranges.next().unwrap_or("").parse::<u64>().unwrap_or_else(
                            |_| {
                                let factor  = start + SIZE_OF_FILES_WRITE_CHUNK;
                                if file_size >= factor {
                                    factor
                                } else {
                                    file_size
                                }
                            }
                        );

                        if start == end || start > end || end > file_size {
                            return Err("Ranges Not Satisfiable".to_string());
                        }
                        headers.change_first_line_to_partial_content();
                        let content_length = (end - start)  + 1  ;
                        headers.set_header_key_value("Content-Length", content_length );
                        headers.set_header_key_value("Access-Control-Allow-Origin","*");
                        headers.set_header_key_value("Content-Range",
                                                     format!("bytes {}-{}/{}", start, end, file_size - 1 )
                        );
                        if let Some(content_type) = content_type {
                            headers.set_header_key_value("Content-Type",content_type);
                        }
                        headers.set_header_key_value("Accept-Ranges","bytes");
                        if let Err(_) = file.seek(SeekFrom::Start(start)).await {
                            return Err("Could not Seek to this start range".to_string());
                        }
                        self.send_headers(h.unwrap()).await?;
                        let mut remaining = content_length as usize;
                        while remaining > 0 {

                            let mut buffer = Vec::with_capacity(SIZE_OF_FILES_WRITE_CHUNK as usize);
                            if let Ok(size) = file.read_buf(&mut buffer).await {
                                if size < 1 {
                                    break;
                                }
                                let to_send = size.min(remaining);
                                let _e = self.send_data(&buffer[..to_send],
                                               to_send >= remaining
                                ).await;
                                remaining -= to_send;
                                if remaining < 1 { return Ok(()); }
                            } else{
                                return  Err("can not send this file range".to_string());
                            }
                        }
                        Ok(())
                    }
                }
            }
        }

        Err("Can not Send this file".to_string())
    }
    /// in order to render html your request should have Content-Length and Content-Type
    /// with specific data so that the browsers would understand what type of response
    /// they are receiving
    pub async fn render_html(&mut self,data:&str,with_headers:bool)->Result<(),String>{
        let data = data.as_bytes();
        if with_headers {
            let mut headers = HttpResponseHeaders::success();
            headers.set_header_key_value("Content-Type","text/html; charset=UTF-8");
            headers.set_header_key_value("Content-Length",data.len());
            self.send_headers(headers).await?;
        }
        self.send_data(data,true).await?;
        Ok(())
    }

    async fn write_bytes(&mut self,bytes:&[u8],end_of_stream:bool)->Result<(),String>{
        match &mut self.protocol {
            Protocol::Http1(p) => {
                match &mut p._peer.0 {
                    WaterTcpStream::Tls(  stream) => {
                        if let Ok(_) = stream.write_all(&bytes).await {
                            if  end_of_stream {
                                if let Err(_) = stream.flush().await {
                                    return Err("can not Flushing All The Data in The Stream".to_string());
                                }
                            }
                            return Ok(());
                        } else {

                        }
                    }
                    WaterTcpStream::Stream(  _stream) => {
                        if let Ok(_) = _stream.write_all(&bytes).await {
                            if  end_of_stream {
                                if let Err(_) = _stream.flush().await {
                                    return Err("can not Flushing All The Data in The Stream".to_string());
                                }
                            }
                            return Ok(());
                        }
                    }
                }

            },
            Protocol::Http2(_p2) => {
                if let Some(_s) = &mut _p2.body_sender {
                    let bytes = bytes.to_vec();
                    let _ = _s.send_data(Bytes::from(bytes), end_of_stream);
                    return Ok(());
                }
                return Err("cant send h2 response".to_owned());
            },
        }

        Err("Cant Write Data to Stream".to_owned())
    }


    /// for sending a most likely headers and custom headers
    /// you could check this headers by using [HttpResponseHeaders] struct
    /// if you are returning success content with 200 status code you could use
    /// this factory [HttpResponseHeaders::success()]
    pub async fn send_headers(
        &mut self,
        mut headers:HttpResponseHeaders,
    )->Result<(),String>{
        self._send_headers(headers,false).await
    }

    async fn _send_headers(
        &mut self,
        mut headers:HttpResponseHeaders,
        end_of_stream:bool
    )->Result<(),String>{

        match  &mut self.protocol {
            Protocol::Http1(h1) => {
                if h1._header_sent {
                    return Ok(());
                }
                h1._header_sent = true;

                let bytes =  headers.to_bytes();
                self.write_bytes(&bytes,end_of_stream).await?;
                return Ok(());
            },
            Protocol::Http2(h2) => {
                if let Some(_) = h2.body_sender {
                    return Ok(());
                }
                let  mut response = http::Response::builder()
                    .status(headers.first_line.status.code);
                let _headers = response.headers_mut();
                if let Some(h) = _headers {
                    for (k,v) in headers.headers.iter() {
                        let key =HeaderName::from_str(k);
                        let value =HeaderValue::from_str(v);
                        if let Err(_) = key {
                            return Err(format!("Can not form header name with {}",k));
                        }
                        if let Err(_) = value {
                            return Err(format!("Can not form header name with {}",v));
                        }
                        h.append(key.unwrap(), value.unwrap());
                    }
                }
                let r = response.body(());
                if let Ok(_r) = r {
                    let body_sender = h2.send_response.send_response(_r,false);
                    if let Ok(sender) = body_sender {
                        h2.body_sender = Some(sender);
                        return Ok(());
                    }
                }},
        }
        Err("None Of Protocols Succeed".to_owned())
    }

    /// for sending bytes data [[u8]]
    /// if you need to use lower level and use custom data to send back
    /// you could use this function and also notice that end_of_stream
    /// indicating if this the last response for your current request or not
    #[async_recursion::async_recursion]
    pub async fn send_data(&mut self,bytes:&[u8],
                           end_of_stream:bool)->Result<(),String>{
        let bytes_length = bytes.len();
        let mut encoded_data : Option<Vec<u8>> = None;
        match &mut self.protocol {
            Protocol::Http1(h1) => {
                let mut  headers = HttpResponseHeaders::success();
                if !h1._header_sent {
                    let threshold = unsafe {___SERVER_CONFIGURATIONS.as_ref().unwrap().threshold_for_encoding_response};
                    if bytes_length >= threshold as usize {
                        if let Some(encoded_message_from_headers) = self.get_from_headers("Accept-Encoding") {
                            let encoding_algorithm =
                                chose_encoding_algorithm::detect_encoding_algorithm(encoded_message_from_headers);
                            if let Some(encoding_algorithm)  = encoding_algorithm {
                                match encoding_algorithm {
                                    HttpEncodingAlgorithms::ZStd => {
                                        let mut data = Vec::new();
                                        if let Ok(_) = chose_encoding_algorithm::encode_data_with_z_std(
                                            bytes,&mut data
                                        ) {
                                            headers.set_header_key_value("Content-Encoding","zstd");
                                            encoded_data = Some(data);
                                        }

                                    }
                                    HttpEncodingAlgorithms::Brotli => {
                                        let mut data = Vec::new();
                                        if let Ok(_) = chose_encoding_algorithm::encode_data_with_brotli(
                                            bytes,&mut data
                                        ) {
                                            headers.set_header_key_value("Content-Encoding","br");
                                            encoded_data = Some(data);
                                        }
                                    }
                                    HttpEncodingAlgorithms::Gzip => {
                                        let mut data = Vec::new();
                                        if let Ok(_) = chose_encoding_algorithm::encode_data_with_gzip(
                                            bytes,&mut data
                                        ) {
                                            headers.set_header_key_value("Content-Encoding","gzip");
                                            encoded_data = Some(data);
                                        }
                                    }
                                    HttpEncodingAlgorithms::Deflate => {
                                        let mut data = Vec::new();
                                        if let Ok(_) = chose_encoding_algorithm::encode_data_with_deflate(
                                            bytes,&mut data
                                        ) {
                                            headers.set_header_key_value("Content-Encoding","deflate");
                                            encoded_data = Some(data);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    match encoded_data {
                        None => {
                            if end_of_stream {
                                headers.set_header_key_value("Content-Length",bytes.len());
                            }
                            headers.set_header_key_value("Date",chrono::Utc::now().to_rfc2822());
                            self.send_headers(headers).await?;
                            if let Ok(_) = self.write_bytes(bytes, end_of_stream).await {
                                self.bytes_sent += bytes_length;
                                return  Ok(());
                            }
                        }
                        Some(ref bytes) => {
                            if end_of_stream {
                                headers.set_header_key_value("Content-Length",bytes.len());
                            }
                           headers.set_header_key_value("Date",chrono::Utc::now().to_rfc2822());
                            let _ = self.send_headers(headers).await?;
                            if let Ok(_) = self.write_bytes(bytes, end_of_stream).await {
                                self.bytes_sent += bytes_length;
                                return  Ok(());
                            }
                        }
                    }
                } else {
                    if let Ok(_) = self.write_bytes(bytes, end_of_stream).await {
                        self.bytes_sent += bytes_length;
                        return  Ok(());
                    }
                }
            },
            Protocol::Http2(h2) => {

                match &mut h2.body_sender {
                    Some(sender) => {
                        let bytes = bytes.to_vec();
                        match sender.send_data(Bytes::from(bytes), end_of_stream) {
                            Ok(_)=>{
                                self.bytes_sent += bytes_length;
                            },
                            Err(e)=>{
                                return Err(format!("{}",e));
                            }
                        }
                    },
                    None => {
                     let mut headers = HttpResponseHeaders::success();
                     let threshold = unsafe {___SERVER_CONFIGURATIONS.as_ref().unwrap().threshold_for_encoding_response};
                     if bytes_length >= threshold as (usize) {
                        if let Some(encoded_message_from_headers) = self.get_from_headers("Accept-Encoding") {
                            let encoding_algorithm =
                                chose_encoding_algorithm::detect_encoding_algorithm(encoded_message_from_headers);
                            if let Some(encoding_algorithm)  = encoding_algorithm {
                                match encoding_algorithm {
                                    HttpEncodingAlgorithms::ZStd => {
                                        let mut data = Vec::new();
                                        if let Ok(_) = chose_encoding_algorithm::encode_data_with_z_std(
                                            bytes,&mut data
                                        ) {
                                            headers.set_header_key_value("Content-Encoding","zstd");
                                            encoded_data = Some(data);
                                        }

                                    }
                                    HttpEncodingAlgorithms::Brotli => {
                                        let mut data = Vec::new();
                                        if let Ok(_) = chose_encoding_algorithm::encode_data_with_brotli(
                                            bytes,&mut data
                                        ) {
                                            headers.set_header_key_value("Content-Encoding","br");
                                            encoded_data = Some(data);
                                        }
                                    }
                                    HttpEncodingAlgorithms::Gzip => {
                                        let mut data = Vec::new();
                                        if let Ok(_) = chose_encoding_algorithm::encode_data_with_gzip(
                                            bytes,&mut data
                                        ) {
                                            headers.set_header_key_value("Content-Encoding","gzip");
                                            encoded_data = Some(data);
                                        }
                                    }
                                    HttpEncodingAlgorithms::Deflate => {
                                        let mut data = Vec::new();
                                        if let Ok(_) = chose_encoding_algorithm::encode_data_with_deflate(
                                            bytes,&mut data
                                        ) {
                                            headers.set_header_key_value("Content-Encoding","deflate");
                                            encoded_data = Some(data);
                                        }
                                    }
                                }
                            }
                        }
                    }

                        match encoded_data {
                         Some(data) => {
                             if end_of_stream {
                                 headers.set_header_key_value(
                                     "Content-Length",
                                     data.len(),
                                 );
                             }
                         }
                         None => {
                             if end_of_stream {
                                 headers.set_header_key_value(
                                     "Content-Length",
                                     bytes_length,
                                 );
                             }
                         }
                     }
                        headers.set_header_key_value(
                         "Date",chrono::Utc::now().to_rfc2822()
                     );
                        self.send_headers(headers).await?;
                        self.send_data(bytes,end_of_stream).await?;
                        return  Ok(());
                    },
                }

            },
        }
        Err("Cant Write Data to Stream".to_owned())
    }
     };
 }