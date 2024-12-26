#![allow(non_snake_case)]

use water_http::server::{ServerConfigurations};
use water_http::{InitControllerRoot, WaterController};

type MainHolderType = u8;
InitControllerRoot!{
    name:MAIN_ROOT,
    holder_type:MainHolderType,
}


#[tokio::main]
async fn main() {

    #[cfg(feature = "debugging")]
    {
        let sub = tracing_subscriber::FmtSubscriber::builder()
            .with_level(true)
            .with_max_level(tracing::Level::DEBUG)
            .finish();
        _=tracing::subscriber::set_global_default(sub);
    }
    let config = ServerConfigurations::bind("127.0.0.1",8084);

    water_http::RunServer!(
        config,
        MAIN_ROOT,
        hassan
    );
}




WaterController! {
    holder -> crate::MainHolderType,
    name -> hassan,
    functions -> {
        GET => / => a(context) async {
            context.send_str("hello world");
        },

        GET => ali => mm(_c) async {
        },
        GET => ali/test => ssss(_c) async {
        }
    }
}




