
use water_http::server:: ServerConfigurations;
use water_http::{InitControllersRoot, WaterController};

type MainHolderType = u8;
InitControllersRoot!{
    name:MAIN_ROOT,
    holder_type:MainHolderType,
}
 fn main() {
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
        POST => / =>main_post(context)async {
            _= context.send_str("hello world from post method").await;
        }
        GET => check/{id}/name => name(context) async {
            let res = format!("hello name {id}");
            _= context.send_string_slice(&res).await;
        }

    }

}




