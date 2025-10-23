use std::collections::HashMap;
use tokio::io::AsyncWriteExt;
use water_http::server::{HttpContext, ServerConfigurations};
use water_http::{InitControllersRoot, response, WaterController};
use water_http::http::HttpSenderTrait;
use water_http::http::request::DynamicBodyMapTrait;


InitControllersRoot! {
    name:MAIN_ROOT,
    holder_type:MainHolderType,
}
type MainHolderType = CHolder;

#[derive(Debug)]
pub struct CHolder {
    pub user:Option<HashMap<String,String>>,

}

#[tokio::main]
async fn main() {

    // when debugging feature enabled
    #[cfg(feature = "debugging")]
    {
        let subscriber  = tracing_subscriber::FmtSubscriber::builder()
            .with_max_level(tracing::Level::DEBUG)
            .finish();
        tracing::subscriber::set_global_default(subscriber)
            .expect("no thing");
    }


    let  config = ServerConfigurations::bind("0.0.0.0",8084);
    water_http::RunServer!(
        config,
        MAIN_ROOT,
        MainController
    );
}

pub async fn upload_file<'a,H:Send + 'static ,const HS:usize,const Q:usize>(context:&mut HttpContext<'a,H,HS,Q>) {

    let body = context.get_body_as_multipart().await;
    if let Ok(body) = body {
        let image = body.get_as_bytes("image");
        if let Some(image) = image  {
            let path = std::path::Path::new("./public/images/test.png");
            let file = tokio::fs::File::create(path).await;
            if let Ok(mut file) = file {
             _= file.write_all(image).await;
            }
            _= context.send_str("success").await;
        }
    }
    _= context.send_str("hello world").await;
}
// you can use one style to make it your default and favorite one
// my personal favorite one is
// method -> path -> function_name(context) async {
//   function body
// }
WaterController! {
    holder -> crate::MainHolderType,
    name -> MainController,
    functions -> {

          // in this case path is "/" while method is GET
        "/" hello_world(context){

            let mut sender = context.sender();
            let date = httpdate::fmt_http_date(std::time::SystemTime::now());
            sender.set_header_ef("Date",date);
            sender.set_header_ef("Content-Type","text/plain; charset=utf-8");
            _= sender.send_str("Hello World").await;
        }

        POST -> upload -> upload(context) [super::upload_file]
         // in this case POST is Method and api/auth/login is path
        POST => api/auth/login => login_handler(context) async {
            response!(context -> "hello from login api endpoint");
        }


        GET => "categories/byId/{id}" => get_cat(context) [super::get_cat_by_id]

        // in this case GET is the method and 'api/v1/users/' the path
        // and this type could inject string parameter like
        // 'api/v1/users/t22' so t22 is now represented by id variable
        GET -> api/v1/users/{id} -> get_user(context) async{
            println!("user id is {id}");
            super::get_response(context).await;
        }




        // in this case hi is the path and method is  GET
        hi(context) [crate::get_response]

        // in this case hi_post would be the path
        POST => hi_post(context) [crate::get_response]

        // in this case POST is the name of path and the method is GET
        "POST" => post(context) [crate::get_response]

        // in this case method is POST and path is ['test/post1']
        POST test/post1 g(context) async {
            super::get_response(context).await
        }

        // in this case method is get and path is hello
        hello(context) [super::get_response]

        // in this case method is post and get is path
        POST => get(context) async {
            super::get_response(context).await;
        }



        GET info(context)[super::get_response]

        // in this case GET is the method and api/v2 is path ,
        #[POST,api/v2/{id}]
        get_profiles(context)  {
         println!("v2 id is {id}");
         super::get_response(context).await;
        }

        // in this example GET is the method and api/v23 is path
        #[GET,api/v23]
        get_profiles_v2(context) async {
          super::get_response(context).await;
        }

         // in this example GET is the method and api/v3 is path
        #[GET,api/v3]
        get_profiles_v3(context) async [super::get_response]

         // in this example GET is the method and api/v4 is path
        #[GET,api/v4]
        get_profiles_v4(context)  [super::get_response]

         // in this example GET is the method and api/v5 is path
        #[GET,api/v5]
        async get_profiles_v5(context)  [super::get_response]

        #[GET,api/6]
        async get_profiles_6(context)  {
            super::get_response(context).await;
        }

        getFile(context) [super::send_files]

    }
    extra_code->(..{

    })

    // , middleware-> (context {
    //     println!("middleware function invoked");
    //
    //     if let Some(ref holder) = context.holder {
    //         if holder.user.is_some() {
    //             println!("user is authenticated");
    //         }
    //     }
    //
    //     if 1 == 1  { return server::MiddlewareResult::Pass }
    //
    //     response!(context -> "invalid middleware passing");
    //     server::MiddlewareResult::Stop
    // })
}

// notice that writing methods like POST,post,Post,posT,POst
// it would give the same result cause the framework has auto under table requests handler

water_http::functions_builder!{

    pub async fn get_response(context)  {
        let method = context.method();
        let path = context.path();

        // sending response
        response!(context string -> "method is {method} while path is {path}");
    }

    pub async fn get_cat_by_id(context) (id){
        let method = context.method();
        let path = context.path();
      response!(context string -> "method is {method} , id is {id} while path is {path}");
    }

    pub async fn send_files(context)  {

           response!(context file -> "./public/text/test1.txt");

        // response!(context file -> "./public/text/test1.txt",|c|{
        //     // if we need to modify or encrypt every chunk sent to the user
        //     for i in &mut *c {
        //         *i = b'a';
        //     }
        // });

    }

      pub async fn send_response(context){

        response!(context -> "hi this is api response") ;
      }

      pub async fn send_file(context){
         response!(context file -> "./public/text/test1.txt");
      }
}

// to generate normal function without helper
// pub async fn fn_name<'context, MainHolderType: Send + 'static, const header_length: usize, const query_length: usize>
//   (context: &mut water_http::server::HttpContext<'context, MainHolderType, header_length, query_length>) {
// }

// so we created water_http::functions_builder macro to help you create functions in fast and easy way







