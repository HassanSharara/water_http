
use water_http::{RunServer,WaterController,InitControllersRoot};

InitControllersRoot!{
        ROOTTYPE,
       RootControllersType
}
pub type RootControllersType = u8;

#[tokio::main]
async fn main() {
    let configs = water_http::server::ServerConfigurations::bind("localhost",8084);
    RunServer!(
      configs,
        ROOTTYPE,
        MainController
    );
}


WaterController! {
    holder -> super::RootControllersType,
    name -> MainController,
    functions -> {
        // the first and the fastest way
        GET -> / -> file(context) async {
            response!(context file ->"./public/text/test1.txt");
        }
    }
}