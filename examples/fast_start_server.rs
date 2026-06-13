use water_http::fast_build;
fn main() {
  start_fast_server();
}

fast_build!{
    port -> 8081,
    functions -> {

        GET => / => hello(context)async{
            _= context.send_str("hello from fast server").await;
        }
    }
}
