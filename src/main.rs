use std::collections::HashMap;

use water_http::{WaterController, DataHolderInitializer, RunServer,get_route_by_name};
// notice that this data holder could be used to parse data from middleware to another or
// from controller to controller , so you could choose what ever you want from data type ,
// or you could make it u8 to have single byte if you don`t use it
DataHolderInitializer!(HashMap<String,String>);
#[tokio::main]
async fn main() {

    RunServer!(MainController::build());
}
WaterController! {
    holder -> crate::_WaterContextDataHolder,
    name -> MainController,
      functions -> {

        GET => test_second => test_second_function(context) async {
         let _ =   context.send_str_data("Second Response",true).await;
        }

        GET => test/newpath => tn(context) async {
            // redirecting to ---> our_new_path
            let _route_url = super::get_route_by_name("our_new_path",None);
            println!("{:?}",_route_url);
            // if you want to directly redirect url by using route name
            let _ = context.redirect_by_route_name("our_new_path",None).await;


            // redirecting to ---> test_v
            let test_v1_atts = [("id","your_custom_id")];
            let _ = context.redirect_by_route_name("test_v",Some(&test_v1_atts));

        }



        GET_test_v => v1/{id} => test_v_function_name(context) async {
            let res = format!("id =  {id}");
            let _ = context.send_string_data(res,true).await;
        }



        GET_our_new_path => hello => h(context) async {
         let _ =   context.send_str_data("test route response",true).await;
        }


    },
}
