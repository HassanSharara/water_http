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
            .with_env_filter(tracing_subscriber::EnvFilter::new("debug"))
            .finish()
            ;
        tracing::subscriber::set_global_default(subscriber)
            .expect("no thing");
    }
    let  config = ServerConfigurations::bind("127.0.0.1",8084);
    water_http::RunServer!(
        config,
        MAIN_ROOT,
        MainController
    );
}
WaterController! {
    holder -> crate::MainHolderType,
    name -> MainController,
    functions -> {
        GET => / => main(context) async {
            _= context.send_str("hello world").await;
        }
    }

}






