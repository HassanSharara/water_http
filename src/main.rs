use std::collections::HashMap;
use water_http::framework_http::HTTPFrameworkConfigs;
use water_http::FrameWorkControllerBuilder;
water_http::DataHolderInitializer!(HashMap<String,String>);


#[tokio::main]
async fn main() {
    let _configurations = HTTPFrameworkConfigs::bind_port(80);
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
            let mut  headers = water_http::framework_http::HttpResponseHeaders::success();
            headers.set_header_key_value("Cache-Control","public, max-age=3600");
            let data = "hello world2";
            headers.set_header_key_value("Content-Length",data.as_bytes().len());
            let _ = context.send_headers(headers).await;
            let _ = context.send_string_data(data,true).await;
        }
    },
}