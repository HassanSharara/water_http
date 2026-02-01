use water_http::server::{ ServerConfigurations};
use water_http::{InitControllersRoot, response, WaterController};
use water_http::http::HttpSenderTrait;


InitControllersRoot! {
    name:MAIN_ROOT,
    holder_type:MainHolderType,
}
type MainHolderType = CHolder;
#[derive(Debug,Clone)]
pub struct CHolder {
    pub user:Option<std::collections::HashMap<String,String>>,

}

fn main() {

    // when debugging feature enabled
    #[cfg(feature = "debugging")]
    {
        let subscriber  = tracing_subscriber::FmtSubscriber::builder()
            .with_max_level(tracing::Level::DEBUG)
            .finish();
        tracing::subscriber::set_global_default(subscriber)
            .expect("no thing");
    }


    let config = ServerConfigurations::bind("0.0.0.0",8084);
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
        GET => / => main(context)async {
            response!(context -> "hello world")
        }
    }
}
