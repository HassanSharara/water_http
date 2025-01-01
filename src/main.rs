use water_http::server:: ServerConfigurations;
use water_http::{InitControllersRoot, WaterController};
type MainHolderType = u8;
InitControllersRoot!{
    name:MAIN_ROOT,
    holder_type:MainHolderType,
}
#[tokio::main]
async fn main() {
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
            _= context.send_html_text("<h4>hello </h4><br></br><h1> hello</h1>").await;
        }
    }

}






