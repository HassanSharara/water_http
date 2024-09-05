use crate::framework_http::HttpResponseHeaders;
macro_rules! server_structure_generator {
    (->&*) => {

pub enum WaterTcpStream {
    Tls(tokio_rustls::server::TlsStream<TcpStream>),
    Stream(TcpStream)
}
type IncomeBody =Option<Option<HttpIncomeBody>>;
static mut ___SERVER_CONFIGURATIONS : Option<WaterServerConfigurations> = None;
const SIZE_OF_READ_WRITE_CHUNK:usize = 100000;
const SIZE_OF_FILES_WRITE_CHUNK:u64 = 100000;
#[allow(unused)]
pub struct HttpContext<DataHolderGeneric:Send> {
    pub protocol:Protocol,
    bytes_sent:usize,
    pub data_holder:Option<DataHolderGeneric>,
    pub path_params_map:HashMap<String,String>,
    body:IncomeBody,
}

 unsafe impl<T:Send>  Send for HttpContext<T> {
}
 pub struct Http1 {
    _peer:(WaterTcpStream,SocketAddr),
     pub request:Request,
    _extra_bytes_from_headers:Vec<u8>,
    _header_sent:bool,
}

        type  Http2Request = http::Request<RecvStream>;
        type  Http2ResponseSender = SendResponse<Bytes>;
         pub struct Http2 {
    pub cached_headers:HttpHeadersMap,
    pub request:Http2Request,
    pub send_response:Http2ResponseSender,
    pub body_sender:Option<SendStream<Bytes>>,
    pub path_query:HashMap<String,String>
}
         pub enum Protocol {
    Http1(Http1),
    Http2(Http2)
}
         pub enum HttpIncomeBody {
         MultiPartFormat(Vec<(HttpMultiPartFormDataField,Vec<u8>)>),
         XWWWForm(XWWWFormUrlEncoded),
         Json(Vec<u8>),
          Unit8Vec(Vec<u8>)
    }
    };



}




macro_rules! server_structure_impl_context {
    (->!) => {



      fn from_http2_connection(
        request:Http2Request,
        send_response:Http2ResponseSender)->Result<Self,String>{
       let mut path_query = HashMap::new();
       let q = request.uri().query();
       if let Some(query) = q {
           path_query = Request::parse_to_query_map(query);
       }
        Ok(HttpContext{
        protocol:Protocol::Http2(Http2 {
            cached_headers:HashMap::new(),
            request,
            send_response,
            body_sender:None,
            path_query
        }),
        bytes_sent:0,
        data_holder:None,
            body:None,
        path_params_map:HashMap::new()
      })
    }

  async fn wait_for_another_request(&mut self){
        self.refresh_stream().await;
    }

    pub fn is_http1(&self)->bool{
        if let Protocol::Http1(_) = self.protocol {
            return  true;
        }
        false
    }
    pub(crate) async fn refresh_stream(&mut self){
        self.bytes_sent = 0;
        if let Protocol::Http1(protocol) = &mut self.protocol {
            protocol._extra_bytes_from_headers.clear();
            protocol._header_sent = false;

            if let Ok((request,extra_body_bytes)) =
                build_headers(&mut protocol._peer.0).await {
                protocol.request = request;
                protocol._extra_bytes_from_headers = extra_body_bytes;
            }

        }
    }

    async  fn from_http1_connection(
        mut _peer:(WaterTcpStream,SocketAddr),
    )->Result<Self,String>{
        let (request,extra_body_bytes) =
         build_headers(&mut _peer.0).await?;
        let context = HttpContext {
            protocol:Protocol::Http1(
                Http1 {
                    _header_sent:false,
                    _peer,
                    request,
                   _extra_bytes_from_headers:extra_body_bytes,
                }
            ),
            bytes_sent:0,
            data_holder:None,
            body:None,
            path_params_map:HashMap::new()
        };
        return Ok(context);
    }
     fn from_h2_protocol_to_valid_headers_map<'a>(key:&str,_p2:&'a mut Http2)->Option<&'a Vec<Vec<String>>>{
        if let Some(_h) = _p2.request.headers().get(key) {
            let s = String::from_utf8_lossy(_h.as_bytes());
            let res = convert_string_value_to_vec_of_strings(&s);
            _p2.cached_headers.insert(key.to_owned(), res);
            return  _p2.cached_headers.get(key);
        }
        None
    }

     include_response_getters_functions!();
     include_response_senders_functions!();
}


}