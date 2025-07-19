use water_http::server::{ServerConfigurations};
use water_http::{InitControllersRoot, WaterController};


InitControllersRoot! {
    name:MAIN_ROOT,
    holder_type:MainHolderType,
}
type MainHolderType = u8;


#[tokio::main]
async fn main() {

    // when debugging feature enabled
    #[cfg(feature = "debugging")]
    {
        let subscriber  = tracing_subscriber::FmtSubscriber::builder()
            .with_max_level(tracing::Level::DEBUG)
            .finish();
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

            response!( context -> "hello world" );
        }


    }
    extra_code->(..{



    })
}
#[derive(serde::Serialize)]
pub struct Response {
    status:&'static str,
    body:&'static str
}
impl Response {

    pub fn test()->Self{ Self {status:"success",body:"yes this is response"}}
}








