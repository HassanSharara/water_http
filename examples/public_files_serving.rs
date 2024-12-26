use std::collections::HashMap;
use water_http::{InitControllerRoot, WaterController};
use water_http::server::ServerConfigurations;
type MainHolderType = HashMap<String,String>;
InitControllerRoot!{
    name:MAIN_ROOT,
    holder_type:MainHolderType,
}
#[tokio::main]
async  fn main(){
    let config = ServerConfigurations::bind("127.0.0.1",8084);

    water_http::RunServer!(
        config,
        MAIN_ROOT,
        MainController
    );
}

WaterController! {
    holder -> super::MainHolderType,
    name -> MainController,
    functions -> {
        GET => public/{allRestPath} => main(context) async {
            let path = format!("./public{}",allRestPath);
            let result =  context.send_file(http::FileRSender::new(path.as_ref())).await;
            match result {
                Ok(_) => {println!("file sent successfully");}
                Err(e) => {
                    println!("{e}")
                }}
        }
    }
}