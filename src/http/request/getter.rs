use std::borrow::Cow;
use bytes::{Buf, BytesMut};
use h2::RecvStream;
use http::{ Request};
use tokio::io::AsyncReadExt;
use crate::http::request::{FormDataAll, IBody, BytesPuller, IBodyChunks, IncomingRequest, MultipartData, ParsingBodyMechanism, ParsingBodyResults, XWWWFormUrlEncoded, MultipartStreamHolder, H2StreamHolder, H1StreamHolder, StreamBytesPuller, H1BytesPuller, H2BytesPuller};
use crate::http::request::MultipartStreamHolder::H1;
use crate::http::status_code::HttpStatusCode;
use crate::server::errors::{ServerError, WaterErrors};
use crate::server::HttpStream;
use crate::util::split;

/// implementing get functions from incoming request
pub  trait  HttpGetterTrait <'a>{

    async fn get_body(&'a mut self)->ParsingBodyResults<'a>;

     async fn get_body_by_mechanism(&'a mut self,mechanism:ParsingBodyMechanism)->ParsingBodyResults<'a>;
 }



pub (crate) struct Http1Getter<'a,'request,
const HEADER_SIZE:usize,
const PATH_QUERY_COUNT:usize
> {
    pub(crate)body_reading_buffer:&'a mut BytesMut,
    pub(crate)left_bytes:&'a [u8],
    pub(crate)stream:&'a mut HttpStream,
    pub(crate)request:&'a IncomingRequest<'request,HEADER_SIZE,PATH_QUERY_COUNT>,
}


impl <'a:'request,'request
    ,
    const HEADER_SIZE:usize,
    const PATH_QUERY_COUNT:usize
> Http1Getter<'a,'request,HEADER_SIZE,PATH_QUERY_COUNT> {
    pub (crate) fn new(
        body_reading_buffer:&'a mut BytesMut,
        left_bytes:&'a [u8],
        stream:&'a mut HttpStream,
        request:&'a IncomingRequest<'a,HEADER_SIZE,PATH_QUERY_COUNT>
    )->Http1Getter<'a,'request,HEADER_SIZE,PATH_QUERY_COUNT>{
        Http1Getter {
            body_reading_buffer,
            left_bytes,
            stream,
            request,
        }
    }


    #[inline]
    pub (crate) async fn get_full_body_multipart_mechanism(&mut self,content_type:&[u8],content_length:&usize)
    ->ParsingBodyResults<'a>{
        let split = split(content_type,b"boundary=");
        if let Some(boundary) = split.last() {
            if !boundary.is_empty() {
                let mut data = MultipartData::new(
                    H1(
                        H1StreamHolder {
                            stream:self.stream,
                            left_bytes:&self.left_bytes[..*content_length]
                        }
                    ),
                    self.body_reading_buffer,

                    String::from_utf8_lossy(boundary),
                    *content_length
                );
                let mut fields= FormDataAll::new();
                if let Ok(_) = data.on_field_detected(
                    |field,data|    {
                        fields
                            .push(
                                field
                                ,
                                data
                            );
                        Ok(())
                    }
                ).await {
                    return  ParsingBodyResults::FullBody(
                        IBody::MultiPartFormData(
                            fields
                        )
                    )
                }
                return ParsingBodyResults::Err(
                    WaterErrors::Http(
                        HttpStatusCode::INTERNAL_SERVER_ERROR
                    )
                )

            }
        }

        return ParsingBodyResults::Err(
            WaterErrors::Server(
                ServerError
                ::MULTIPARTFORMDATA_ERROR
            )
        )
    }


    #[inline]
    pub (crate) async fn get_chunked_body_multipart(&'a mut self,content_type:&'a [u8],content_length:&usize)
    ->ParsingBodyResults<'a>{

            if let Some(boundary) = split(content_type,b"boundary=").last() {
                let  data = MultipartData::new(
                    H1(H1StreamHolder {
                        stream:self.stream,
                        left_bytes:&self.left_bytes[..*content_length]
                    }),
                    self.body_reading_buffer,
                    String::from_utf8_lossy(boundary),
                    *content_length
                );
                return ParsingBodyResults::Chunked(
                    IBodyChunks::FormData(
                        data
                    )
                )
            }

        return ParsingBodyResults::Err(
            WaterErrors::Http(
                HttpStatusCode::BAD_REQUEST
            )
        )
    }
}



impl <'a:'request,'request
    ,
    const HEADER_SIZE:usize,
    const PATH_QUERY_COUNT:usize
>  HttpGetterTrait<'a> for Http1Getter<'a,'request,HEADER_SIZE,PATH_QUERY_COUNT> {
    async fn get_body(&'a mut self) ->ParsingBodyResults<'a>{
        self.get_body_by_mechanism(ParsingBodyMechanism::Default).await
    }

    async fn get_body_by_mechanism(&'a mut self,mechanism: ParsingBodyMechanism)->ParsingBodyResults<'a>{
        let content_length = self.request.content_length();
        if let Some( content_length) = content_length {
            let body_should_handled_as_chunks = self.left_bytes.len() < *content_length;
            if body_should_handled_as_chunks {
                match mechanism {
                    ParsingBodyMechanism::Default => {
                        let content_type = self.request.headers
                            .get_as_bytes(b"Content-Type");
                        if let Some(content_type) = content_type {
                            match content_type {
                                b"multipart/form-data" => {
                                    return self.get_chunked_body_multipart(
                                        content_type,
                                        content_length
                                    ).await
                                }
                                b"application/x-www-form-urlencoded"=>{

                                }
                                _=>{}
                            }
                        }
                    }
                    ParsingBodyMechanism::JustBytes => {
                        let puller = BytesPuller::new(
                            StreamBytesPuller::H1(
                                H1BytesPuller {
                                    stream:self.stream,
                                    reading_buffer:self.body_reading_buffer,
                                    left_bytes:self.left_bytes
                                }
                            ),
                            *content_length
                        );
                        return ParsingBodyResults::Chunked(
                            IBodyChunks::Bytes(
                                puller
                            )
                        );
                    }
                    ParsingBodyMechanism::FormData => {
                        let content_type = self.request.headers
                            .get_as_bytes(b"Content-Type");
                           if let Some(content_type) = content_type {
                            return self.get_chunked_body_multipart(
                                content_type,
                                content_length
                            ).await
                        }
                    }
                    ParsingBodyMechanism::XWWWFormData => {
                        let  remaining = *content_length - self.left_bytes.len();
                        let mut rem = remaining;
                        while rem > 0 {
                            match self.stream.read_buf(self.body_reading_buffer).await {
                                Ok(s) => {
                                    rem -= rem.min(s);
                                }
                                Err(_) => {
                                    return ParsingBodyResults::Err(
                                        WaterErrors::Server(
                                            ServerError::HANDLING_INCOMING_BODY_ERROR
                                        )
                                    )
                                }
                            }

                        }
                        let data = self.left_bytes;
                        let second_data = &self.body_reading_buffer[..remaining];
                        let data = XWWWFormUrlEncoded::from_multiple_payloads(
                            (data,second_data)
                        );
                        return ParsingBodyResults::FullBody(
                            IBody::XWWWFormUrlEncoded(data)
                        )
                    }
                }
            }
            else {
                match mechanism {
                    ParsingBodyMechanism::Default => {
                        match self.request.headers
                            .get_as_bytes(b"Content-Type") {
                            None => { return ParsingBodyResults::Err(WaterErrors::Http(HttpStatusCode::BAD_REQUEST))}
                            Some(content_type) => {
                                let lower_case = String::from_utf8_lossy(content_type).to_lowercase();
                                if lower_case.contains("application/x-www-form-urlencoded") {
                                    let data = &self.left_bytes[..*content_length];
                                    let x_fields = XWWWFormUrlEncoded::new(data);
                                    return ParsingBodyResults::FullBody(
                                        IBody::XWWWFormUrlEncoded(
                                            x_fields
                                        )
                                    )
                                }
                                else if lower_case.contains("multipart/form-data") {
                                    return self.get_chunked_body_multipart(
                                      content_type,
                                      content_length
                                    ).await;
                                }
                                  ParsingBodyResults::FullBody(
                                    IBody::Bytes(
                                        &self.left_bytes[..*content_length]
                                    )
                                )
                            }
                        }
                    }
                    ParsingBodyMechanism::JustBytes => {
                        return  ParsingBodyResults::FullBody(
                            IBody::Bytes(
                                &self.left_bytes[..*content_length]
                            )
                        )
                    }
                    ParsingBodyMechanism::FormData => {
                        let content_type = self.request.headers
                            .get_as_bytes(b"Content-Type");
                        if let Some(content_type) = content_type {
                            return self.get_chunked_body_multipart(
                                content_type,
                                content_length
                            ).await
                        }
                        return  ParsingBodyResults::Err(
                            WaterErrors::Http(HttpStatusCode::BAD_REQUEST)
                        )
                    }
                    ParsingBodyMechanism::XWWWFormData => {
                        let data = &self.left_bytes[..*content_length];
                        let x_fields = XWWWFormUrlEncoded::new(data);
                        return ParsingBodyResults::FullBody(
                            IBody::XWWWFormUrlEncoded(
                                x_fields
                            )
                        )
                    }
                };
            }
        }
         ParsingBodyResults::None
    }


}









pub (crate) struct Http2Getter<'a> {
    pub(crate)batch:&'a mut Request<RecvStream>,
    pub(crate)content_length:usize,
    pub(crate)reading_buffer:&'a mut BytesMut,
}
impl<'a>   Http2Getter<'a> {

    pub (crate) async fn get_body_as_multipart
    (&'a mut self,
                                               boundary:Cow<'a,str>)
    ->ParsingBodyResults<'a>

    {
        let data = MultipartData::<'a>::new(
            MultipartStreamHolder::H2(
                H2StreamHolder {
                    batch: self.batch
                }
            ),
            self.reading_buffer,
            boundary,
            self.content_length
        );
        ParsingBodyResults::Chunked(IBodyChunks::FormData(data))
    }


    pub (crate) async fn get_body_as_bytes
    (&'a mut self)->ParsingBodyResults<'a>{
        if self.content_length == 0 { return ParsingBodyResults::Err(
            WaterErrors::Http(HttpStatusCode::BAD_REQUEST)
        )}
        let puller = BytesPuller::new(
            StreamBytesPuller::H2(
                H2BytesPuller{
                    batch:self.batch,

                },
            ),
            self.content_length
        );
        return ParsingBodyResults::Chunked(IBodyChunks::Bytes(puller))
    }

    pub (crate) async fn get_body_as_xww
    (&'a mut self)->ParsingBodyResults<'a>{
        let mut remaining = self.content_length;
        let  body_mut = self.batch.body_mut();
        self.reading_buffer.clear();


        let err : ParsingBodyResults<'_>= ParsingBodyResults::Err(
            WaterErrors::Server(
                ServerError::HANDLING_INCOMING_BODY_ERROR
            )
        );

        while remaining > 0 {
            let data = body_mut.data().await;
            match data {
                None => { break }
                Some(data) => {
                    match data {
                        Ok(data) => {
                            self.reading_buffer.extend_from_slice(data.as_ref());
                            remaining-=data.len();
                            continue;
                        }
                        Err(_) => {
                            return err
                        }
                    }
                }
            }
        }

        return ParsingBodyResults::FullBody(
            IBody::XWWWFormUrlEncoded(
                XWWWFormUrlEncoded::new(
                    self.reading_buffer.chunk()
                )
            )
        )

    }
}
impl<'a> HttpGetterTrait<'a> for Http2Getter<'a> {
    async fn get_body(&'a mut self) ->ParsingBodyResults<'a>{
        self.get_body_by_mechanism(ParsingBodyMechanism::Default).await
    }

    async fn get_body_by_mechanism(&'a mut self,mechanism: ParsingBodyMechanism)->ParsingBodyResults<'a> {

        let request_err = ParsingBodyResults::Err(WaterErrors::Http(HttpStatusCode::BAD_REQUEST));
        match mechanism {

            ParsingBodyMechanism::Default => {
                // preparing content type
                let content_type =
                  match self.batch.headers().get("Content-Type") {
                      None => {
                          return  request_err
                      }
                      Some(data) => {
                         String::from_utf8_lossy(data.as_ref())
                      }
                  };



                   if content_type.contains( "multipart/form-data" ) {

                       let boundary  =
                       match  content_type.split("boundary=")
                           .last() {
                           None => { return ParsingBodyResults::Err(
                               WaterErrors::Http(HttpStatusCode::BAD_REQUEST)
                           )}
                           Some(data) => {data}
                       };
                      return  self.get_body_as_multipart(
                          Cow::Owned(boundary.into())
                      ).await;
                   }
                   else if content_type.contains("application/x-www-form-urlencoded"){
                       return self.get_body_as_xww().await;
                   }
                   self.get_body_as_bytes().await
               }

            ParsingBodyMechanism::JustBytes => {
                return self.get_body_as_bytes().await;
            }
            ParsingBodyMechanism::FormData => {
                let content_type =
                    match self.batch.headers().get("Content-Type") {
                        None => {
                            return  request_err
                        }
                        Some(data) => {
                            let string  = data.to_str().unwrap_or("");
                            match string.split("boundary=").last() {
                                None => { return request_err}
                                Some(data) => {
                                    data.to_string()
                                }
                            }
                        }
                    };
                return  self.get_body_as_multipart(
                    content_type.into()
                ).await;
            }
            ParsingBodyMechanism::XWWWFormData => {
                return self.get_body_as_xww().await;
            }
        }
    }
}



/// Http Getter for getting data from incoming request and request queries
/// also headers values
pub enum HttpGetter<'a,'request,const headers:usize,const qs:usize> {
    H1(Http1Getter<'a,'request,headers,qs>),
    H2(Http2Getter<'a>)
}




impl <
    'a:'request,'request,
    const HEADER_SIZE:usize,
    const PATH_QUERY_COUNT:usize
>  HttpGetterTrait<'a> for HttpGetter<'a,'request,HEADER_SIZE,PATH_QUERY_COUNT> {
    async fn get_body(&'a mut self) -> ParsingBodyResults<'a,> {
        match self {
            HttpGetter::H1(h1) => {h1.get_body().await}
            HttpGetter::H2(h2) => {h2.get_body().await}
        }
    }

    async fn get_body_by_mechanism(&'a mut self, mechanism: ParsingBodyMechanism) -> ParsingBodyResults<'a,> {
        match self {
            HttpGetter::H1(h1) => {h1.get_body_by_mechanism(mechanism).await}
            HttpGetter::H2(h2) => {h2.get_body_by_mechanism(mechanism).await}
        }
    }
}


