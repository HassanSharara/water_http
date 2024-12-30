#![allow(non_snake_case)]

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
        POST => / => a(context) async {
            let body = context.get_body_as_multipart().await;

            if let Ok(body) = body {
                let field = body.get_field("image");
                if let Some(field) = field {
                    let ref data = field.data;
                    let file = tokio::fs::File::create("./public/images/new_image.jpg").await;
                    if let Ok(mut file) = file {
                        _= file.write_all(data).await.expect("can not write data to the specific file");
                        _= context.send_str("file uploaded successfully").await;
                        return;
                    }
                    _=context.send_str("can not open file").await;
                }
            }

            _= context.send_str("not exist").await;
        }
    }
    extra_code -> (..{
        use tokio::io::AsyncWriteExt;

    })
}




