
use water_http::{RunServer,WaterController,InitControllersRoot};

InitControllersRoot!{
        ROOTTYPE,
       RootControllersType
}
pub type RootControllersType = u8;

#[tokio::main]
async fn main() {
    let configs = water_http::server::ServerConfigurations::bind("127.0.0.1",8084);
    RunServer!(
      configs,
        ROOTTYPE,
        MainController
    );
}


WaterController! {
    holder -> super::RootControllersType,
    name -> MainController,
    functions -> {
        // the first and the fastest way
        GET -> / -> file(context) async {
            response!(context file ->"./public/text/test1.txt");
        }

        GET => v2 => file2(context) async {
            _= context.send_file(
                http::FileRSender::new("./public/text/test1.txt")
            ).await;
        }

          GET => v3 => file3(context) async {
            let mut sender = context.sender();
            let mut file = http::FileRSender::new("./public/text/test1.txt");
            // when you need to modify each chunk
            file.set_edit_each_chunk(
                |c|{
                    for byte in c {
                        *byte = b'y';
                    }
                }
            );
            _= sender.send_file(file).await;
        }
    }
}