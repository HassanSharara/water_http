use water_http::server::{ ServerConfigurations};
use water_http::{InitControllersRoot, WaterController};

type MainHolderType = u8;
InitControllersRoot!{
    name:MAIN_ROOT,
    holder_type:MainHolderType,
}

#[tokio::main]
async fn main() {

    #[cfg(feature = "debugging")]
    {
        let subscriber  = tracing_subscriber::FmtSubscriber::builder()
            .with_max_level(tracing::Level::DEBUG)
            .finish();
        tracing::subscriber::set_global_default(subscriber)
            .expect("no thing");
    }
   _= tokio::spawn(async move {
       let  config = ServerConfigurations::bind("127.0.0.1",8084);
       water_http::RunServer!(
        config,
        MAIN_ROOT,
        MainController
    );
   }).await;
}
WaterController! {
    holder -> crate::MainHolderType,
    name -> MainController,
    functions -> {
        GET => / => main(context) async {
            _= context.send_str(
                "hello world"
            ).await;
        },
        PATCH => / => patch(context) async{

            _= context.send_str("patch successfully").await;
        },
         put => / => put(context) async{

            _= context.send_str("put successfully").await;
        },
         head => / => head(context) async{

            _= context.send_str("head successfully").await;
        },
         Options => / => options(context) async{

            _= context.send_str("patch successfully").await;
        },
         Delete => / => delete(context) async{

            _= context.send_str("delete successfully").await;
        },
         Trace => / => trace(context) async{

            _= context.send_str("trace successfully").await;
        },

        POST => / => post(context) async {
            _= context.send_str("hello world").await;
        }
    }
    extra_code->(..{

    })
}











