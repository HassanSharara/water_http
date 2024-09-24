use std::collections::HashMap;
use water_http::{WaterController, DataHolderInitializer, RunServer, WaterServerConfigurations};
// notice that this data holder could be used to parse data from middleware to another or
// from controller to controller , so you could choose what ever you want from data type ,
// or you could make it u8 to have single byte if you don`t use it
DataHolderInitializer!(HashMap<String,String>);

#[tokio::main]
async fn main() {
    let config = WaterServerConfigurations::bind("127.0.0.1",8084);
      RunServer!(
        config,
        MainController::build());
}
WaterController! {
    holder -> crate::_DataHolderOfWaterCapsuleController,
    name -> MainController,
      functions -> {
        GET => / => test_second_function(context) async {
         let _ =   context.send_str_data("Hello, world!",true).await;
        }

    },
}
