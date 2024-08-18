use std::collections::HashMap;
use water_http::framework_http::HTTPFrameworkConfigs;
use water_http::FrameWorkControllerBuilder;
water_http::DataHolderInitializer!(HashMap<String,String>);


#[tokio::main]
async fn main() {
    let _configurations = HTTPFrameworkConfigs::bind_port(8082);
    water_http::RunServer!(
        _configurations,
        [
            MainController::build()
        ]
    );
}

FrameWorkControllerBuilder! {
    holder -> super::___ContextDataHolder,
    name -> MainController,
    functions -> {
        GET => / => main_function(context) async {
            let _ = context.send_string_data("hello world",true).await;
        }
    },
}