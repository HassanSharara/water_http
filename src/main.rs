use water_http::server::{ServerConfigurations};
use water_http::{InitControllersRoot, response, WaterController};


InitControllersRoot! {
    name:MAIN_ROOT,
    holder_type:MainHolderType,
}
type MainHolderType = u8;


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


    let  config = ServerConfigurations::bind("127.0.0.1",8084);
    water_http::RunServer!(
        config,
        MAIN_ROOT,
        MainController
    );
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

        // in this case GET is the method and 'api/v1/users/' the path
        // and this type could inject string parameter like
        // 'api/v1/users/t22' so t22 is now represented by id variable
        GET -> api/v1/users/{id} -> get_user(context) async{
            println!("user id is {id}");
            super::get_response(context).await;
        }

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
         // in this example GET is the method and api/v4 is path

        #[GET,api/v5]
        async get_profiles_v5(context)  [super::get_response]

        #[GET,api/6]
        async get_profiles_6(context)  {
            super::get_response(context).await;
        }

    }
    extra_code->(..{



    })
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
}







