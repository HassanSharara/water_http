

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
           let route =  route!("create_posts").expect("can not fount route name");
           _= context.redirect(route.as_ref()).await;
        }
    }

    children -> ([
        PostsController
    ]),
    middleware -> (context{
        _= context.send_str("MainController middleware checkpoint").await;
        server::MiddlewareResult::Stop
    })
}



WaterController! {
    holder -> crate::MainHolderType,
    name -> PostsController,
    functions -> {
        GET_create_posts => create => create(context)async {
            _= context.send_str("hello from create posts route inside PostsController").await;
        }
    }
    prefix -> ("postsGroup"),
    apply_parents_middlewares->(true)
}





