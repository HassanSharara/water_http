use water_http::server::{ ServerConfigurations};
use water_http::{InitControllersRoot, WaterController};

type MainHolderType = u8;
InitControllersRoot!{
    name:MAIN_ROOT,
    holder_type:MainHolderType,
}

#[tokio::main]
async fn main() {

    #[cfg(feature = "debugging")]
    {
        let subscriber  = tracing_subscriber::FmtSubscriber::builder()
            .with_max_level(tracing::Level::DEBUG)
            .finish();
        tracing::subscriber::set_global_default(subscriber)
            .expect("no thing");
    }
   _= tokio::spawn(async move {
       let  config = ServerConfigurations::bind("127.0.0.1",8084);
       water_http::RunServer!(
        config,
        MAIN_ROOT,
        MainController
    );
   }).await;
}
WaterController! {
    holder -> crate::MainHolderType,
    name -> MainController,
    functions -> {
        GET => / => main(context) async {
            _= context.send_str(
                "hello world"
            ).await;
        },
        POST => / => post(context) async {
            let mut getter = context
            .getter();
            let body_chunks_reader =  getter.get_body_by_mechanism(ParsingBodyMechanism::ChunkedTransferEncoding).await;
            if let ParsingBodyResults::Chunked(IBodyChunks::Chunked(mut reader)) =
            body_chunks_reader {
                _= reader.on_chunk_detected(|c,data|{
                    println!("chunk {} {} {}",c.chunk_size,c.index,String::from_utf8_lossy(data).len());
                    return Ok(None);
                }).await;
            }
            _= context.send_str("hello world").await;
        }
    }
    extra_code->(..{

    })
}











