
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
            let mut response = http::LazyResponse::new();
            response.set_text_response("hello world from lazy response");
            context.set_lazy_response(response);
        }
    }

}




