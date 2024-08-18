use std::option::Option;
use std::collections::HashMap;
use knife_web_framework_lib::configurations::HTTPFrameworkConfigs;
use knife_web_framework_lib::FrameWorkControllerBuilder;


knife_web_framework_lib::DataHolderInitializer!(HashMap<String,String>);
#[tokio::main]
async fn main(){
    knife_web_framework_lib::RunServer!(
        HTTPFrameworkConfigs::default(),
        MainController::build()
    );
}


FrameWorkControllerBuilder! {
    holder -> super::___ContextDataHolder,
    name -> MainController,
    functions ->  {

    GET =>  g/g/a => hello(context) async {
            let _ = context.send_string_data("hello",true).await;
      }

     GET =>  g/g/{y} => hello_test(context) async {
            let _ = context.send_string_data(&format!("{y}"),true).await;
            
      }
    },
    children->vec![],
}


