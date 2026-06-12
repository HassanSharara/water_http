
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
    children -> ([
        SecondController
    ]),
    interceptor -> (context {
        let mut response = http::LazyResponse::new();
            response.set_text_response("response intercepted by interceptor");
            context.set_lazy_response(response);
    })


}

WaterController! {
    holder -> crate::MainHolderType,
    name -> SecondController,
    functions -> {
        GET => secondController => main(context) async {
            let mut response = http::LazyResponse::new();
            response.set_text_response("hello world from lazy response");
            context.set_lazy_response(response);
        }
    }

    interceptor -> (context {
        let mut response = http::LazyResponse::new();
            response.set_text_response("response intercepted by SecondController");
            context.set_lazy_response(response);
    }),
    children -> ([
        ThirdController
    ])
}


WaterController! {
    holder -> crate::MainHolderType,
    name -> ThirdController,
    functions -> {
        GET => thirdController => main(context) async {
            let mut response = http::LazyResponse::new();
            // this will never be called  because we applied parents interceptor
            // so the response will be
            // "response intercepted by SecondController"
            response.set_text_response("hello world from ThirdController");
            context.set_lazy_response(response);
        }
    }

    apply_parents_interceptors -> (true)
}




