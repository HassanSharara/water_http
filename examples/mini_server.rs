use water_http::server::mini::{CtxPtr, HandlerFn, serve};
use water_http::server::ServerConfigurations;

fn main() {
    let no_init = || async {};
    let conf = ServerConfigurations::bind("0.0.0.0", 8084);
    serve::<16, 10, _,_,_>(conf, HandlerFn(|ctx: CtxPtr<16, 10>| handler(ctx)),Some(no_init));
}

async fn handler(mut ctx: CtxPtr<16,10>){
    let ctx = ctx.get();
    ctx.set_header("Content-Length", "11");
    ctx.write_body_bytes(b"Hello World");
}