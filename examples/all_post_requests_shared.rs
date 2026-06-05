
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use water_http::server::{HttpContext, ServerConfigurations};
use water_http::{functions_builder, InitControllersRoot, WaterController};

use water_http::http::request::{DynamicBodyMap, DynamicBodyMapTrait, HttpGetterTrait, IBodyChunks, ParsingBodyMechanism, ParsingBodyResults};
use water_http::http::status_code::HttpStatusCode;


InitControllersRoot! {
    name:MAIN_ROOT,
    holder_type:MainHolderType,
    shared_type:u8,
}
type MainHolderType = CHolder;

#[derive(Debug)]
pub struct CHolder {
    pub user:Option<HashMap<String,String>>,

}

fn main() {

    #[cfg(any(feature = "debugging",feature = "count_connection_parsing_speed"))]
    {
        let subscriber  = tracing_subscriber::FmtSubscriber::builder()
            .with_max_level(tracing::Level::DEBUG)
            .finish();
        tracing::subscriber::set_global_default(subscriber)
            .expect("no thing");
    }

    let  config = ServerConfigurations::bind("0.0.0.0",8084);
    water_http::RunServer!(
        config,
        MAIN_ROOT,
        MainController,
        shared
    );
}

fn shared()->Pin<Box<dyn Future<Output=u8>>>{
    Box::pin(async {8})
}

WaterController! {
    holder -> super::MainHolderType,
    shared -> u8,
    name -> MainController ,
    functions -> {

        hello(c){_=c.send_str("Hello, World!").await}

        POST -> "submit-form" -> url_encoded(context)[super::url_encoded]
        POST -> "upload" -> upload(context)[super::upload]
        POST -> "transfer-chunked" -> handle_chunked(context) [super::handle_chunked]
        POST -> "upload-bin" -> upload_bin(context) [super::upload_bin]
    }
}

async fn upload_bin<H:Send,SHARED:Clone,const HS:usize,const QS:usize>(g:&mut HttpContext<'_,H,SHARED,HS,QS> ){
    #[cfg(feature = "debugging")]
    {
        tracing::info!("upload binary request invoked");
    }
    let mut gf = g.getter();
    let puller = gf.get_body().await;
    // println!("res is {:?}",puller);
    if let ParsingBodyResults::Chunked(IBodyChunks::Bytes(mut puller)) = puller {
        if puller.on_chunk(|data|{
             println!("binary data is {:?}",String::from_utf8_lossy(data));
            return Ok(());
        }).await.is_ok() {
            println!("sending response from bytes puller");
            _= g.send_str("success").await;
            return;
        }
    }
    else if let ParsingBodyResults::FullBody(
    water_http::http::request::IBody::Bytes(_b)
    ) = puller {
        _= g.send_str("Success").await;
    }
    _= g.send_str("Failed").await;
}

functions_builder!{


    fn upload(context) {
        let mut  body = context.getter();
        let b = body.get_body_by_mechanism(ParsingBodyMechanism::FormData).await;
        if let ParsingBodyResults::Chunked(IBodyChunks::FormData(mut m)) = b {
            _=m.on_field_detected(|f,data|{
                println!("field detected {:?} while data is {:?}",f.content_disposition_name(),String::from_utf8_lossy(data));
                Ok(None)
            }).await;
            println!("sending response  ");
            _= context.send_str("success").await;
            return
        }
        _= context.send_status_code_as_final_response(HttpStatusCode::INTERNAL_SERVER_ERROR).await;
    }

    fn url_encoded(context) {

        let body = context.get_body_map().await;

        println!("url encoded called {:?}",body);
        if let Ok( body_map ) = body {
            if let DynamicBodyMap::Xww(x_map ) = body_map {
                println!("body is {:?}",x_map.all());
                _= context.send_str("success").await;
                return
            }
        }
        _=context.send_status_code_as_final_response(HttpStatusCode::INTERNAL_SERVER_ERROR).await;
    }


    async fn handle_chunked(context){
        #[cfg(not(feature = "accept_transfer_chunked"))]
        {
            _= context.send_str("you should apply feature ( accept_transfer_chunked ) ").await;
            return
        }
        #[cfg(feature = "accept_transfer_chunked")]
        {
            use water_http::http::request::{ParsingBodyResults,IBodyChunks,HttpGetterTrait,ParsingBodyMechanism};
        let mut getter = context.getter();
        if let ParsingBodyResults::Chunked(IBodyChunks::Chunked(mut chunk_reader)) =  getter.get_body_by_mechanism(ParsingBodyMechanism::ChunkedTransferEncoding).await {
            if             chunk_reader.on_chunk_detected(|chunk,data|{
                println!("chunk index is {} while chunk size is {}",chunk.index,chunk.chunk_size);
                println!("incoming chunk data length : {} for chunk index {} ",data.len(),chunk.index);
                // if you not have async task on each chunk you could simply return Ok(None)
                Ok(Some(Box::pin(async {
                    Ok(())
                })))
            }).await.is_ok() {

                _=context.send_str("success").await;

                return
            }
        }
        _= context.send_status_code_as_final_response(HttpStatusCode::INTERNAL_SERVER_ERROR).await;
           return
        }

        }
}



