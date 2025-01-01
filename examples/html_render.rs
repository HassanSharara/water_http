use water_http::server::ServerConfigurations;
use water_http::{InitControllersRoot, WaterController};

type MainHolderType = u8;
InitControllersRoot! {
    name:MAIN_ROOT,
    holder_type:MainHolderType,
}
#[tokio::main]
async fn main() {
    let config = ServerConfigurations::bind("127.0.0.1", 8084);
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
            let html_page = MainPage;
            if let Ok(html_page) = html_page.render() {
             _= context.send_html_text(html_page.as_str()).await;
            } else {
                _= context.send_str("error!").await;
            }
        }
    }
    extra_code -> (..{
           use askama::Template;
           #[derive(Template)]
           #[template(path = "base.html")]
           struct MainPage;
    })

}






