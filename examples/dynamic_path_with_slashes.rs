use water_http::{fast_build};
fn main() {
    start_fast_server();
}

fast_build!{
    port -> 8081,
    functions -> {

        GET => public/{path}/>> => hello(context)async{
            let path = format!("path is {}",path);
            _= context.send_string_slice(&path).await;
        }
    }
}
