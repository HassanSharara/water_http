use water_http::configurations::HTTPFrameworkConfigs;
use std::collections::HashMap;


water_http::DataHolderInitializer!(HashMap<String,String>);

#[tokio::main]
async fn main() {
    let _configuration = HTTPFrameworkConfigs::default();
     water_http::RunServer!(
      _configuration,
       [
           MainController::build()
       ]
    );
}


water_http::FrameWorkControllerBuilder!{
    holder -> super::___ContextDataHolder,
    name -> MainController,
    functions-> {
    GET => / => hello_world(context)async {
          let _  =  context.send_string_data("hello world",true).await;
        }
   },
}