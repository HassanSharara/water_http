use water_http::{fast_build, WaterController};
fn main() {
  start_fast_server();
}

fast_build!{
    port -> 8081,
    shared_struct-> async shared_function()->String {
        "Injected Hello World From Shared Struct".to_string()
    },
    functions -> {

        GET => / => hello(context)async{
            let shared = context.thread_shared_struct.as_ref().unwrap().to_string();
            _= context.send_string_slice(&shared).await;
        }
    }

    children -> ([
        SecondController
    ])
}


WaterController! {
    holder -> super::MainHolderType,
    shared -> String,
    name -> SecondController,
    functions -> {
        GET => second => s(context)async {
            _= context.send_str("second controller").await;
        }
    }
}
