

# Water_http
http framework meant to be the fastest and easiest 
http framework by using the power of rust macros
and it`s provide stable workload over os systems 
and all the important features for servers 


# Features 

 - very slight and easy to use 
 - blazingly fast with very advanced concepts and services to provide, you can see Benchmarks repository
 - very simple and familiar constructor
 - support both protocols http2 and http1 with all existed features and more
 - support all http encoding algorithms with custom encoding for low levels like
    - brotli
    - zstd
    - gzip
    - deflate
    - lz4
    - snappy
    - bzip2
    - custom encoding algorithms for low level of programming
 - provide simple routing structure with many patterns of write
 - very fast http parsing algorithms which archived http 1 parsing with 1 micro second
 - can handle millions of requests in given seconds 
 - support videos streaming for web pages videos
 - provide easy approaches for saving and downloading files from and to the server
 - support tls or ssl implementation for secure connections without any confusing
 - naming routes and redirecting to these routes by their names using one single method
 - multi pattern ways to generate your code and very easy approaches
 - support controlling low actions like block custom ip addresses from connecting your server or strict your service to custom ip addresses
 - thresholding actions like the threshold the maximum size of using compressing algorithms when sending response back considering clients encoding support



# Nice Information
- we need to understand that http request is based on another protocols like 
tcp (http1 and htt2) or udp (http3) and what framework is basically do is to read incoming bytes
into certain bytes buffer which located on ram memory
so when the framework uses the same bytes which read by the OS as much as possible that would make the framework better because at this logic we had zero additional memory allocations
and that`s what water_http framework is meant to be
- when we need to serve requests in better pattern 
we need to understand one thing (  less memory allocations leads to better performance )
so water http used another way of serving post request in http 1 protocol
and it`s allocating one buffer for each connection
and reusing the same allocated buffer for each http request 
instead of using new buffer for each http request 
- we are using IncomingRequest struct for parsing Http requests
which it self-developed struct for using the same buffer bytes to handling request and also
single bytes iteration with zero allocations
- we used the power of rust macros to makes code very easy to build and use /


# Installation
for start building your own web server app
you need to follow the steps
- install water_http by 
   - using shell 
     ```shell
     cargo add water_http
     ```
   - using cargo.toml file
      ```toml
      water_http = "[last_version]"
      ```
- install tokio by
    - using shell
      ```shell
      cargo add tokio --features=full
      ```
    - using cargo.toml file
       ```toml
       tokio = {version = "[last_version]",features = ["full"] }     
  Notes : you may encounter errors while building your Application
  because there are some packages depends on clang compiler
  specially tls crates , so you need to install clang llvm compiler
  and so on

## Concepts

- water http built with concepts of controllers tree 
let`s say we have  

      
     -MainController
                     __ child 1 Controller
                    |
                    child 2 controller

   child 1 and child2 controller are both depends on 
   MainController prefix if he has one 
   and also  could depend on MainController middleware 
   if apply_parents_middlewares was set to true

- the context has shared public object of type generic 
,so we could parse anything to children controllers
also we need to specify what is the maxy headers count (
  which means how many headers we would read from incoming request
  )
and the max query count ( which means how many queries we could request using path )
for example : [http://example.com/post?id=1&name=2]()
in this example we have two queries in the path (id,name)

so to init these for controllers we could use
[InitControllerRoot]() macro

```rust
use std::collections::HashMap;
use water_http::InitControllersRoot;

type MainHolderType = HashMap<String,String>;
InitControllersRoot! {
    /// the name of root
    name:MAIN_ROOT,
    /// holder type
    holder_type:MainHolderType,
    /// (optional) default (16)
    headers_length:16,
    /// (optional) default(16)
    queries_length:16
}
```
so after initializing our controllers root we could build our 
controllers

  Note : we are specifying headers length and queries length for two purpose

  1- for providing security and refuse all malicious big load requests

  2- to allocate memory on the stack which need known sized bytes so that we could make the app significantly faster


# Some Tips
- if you need to trace debugging hints you could use feature debugging
  by running shell 
  ```shell
  cargo run --features debugging
  ```
- if you need to count speed of parsing bytes to http 1 protocol
 as  request you could use feature "count_connection_parsing_speed"
 ```shell
  cargo run --features count_connection_parsing_speed
 ```
- if you need to run one of the examples files 
```shell
 cargo run example public_files_serving
```

- to set the framework to auto handle content encoding when sending back response 
```rust
use water_http::server::{EncodingConfigurations, EncodingLogic, ServerConfigurations};

 fn enable_encoding(config:&mut ServerConfigurations){

     let mut  encoding = EncodingConfigurations::default();
     encoding.set_logic(EncodingLogic::All);
     config.set_response_encoding_configuration(
         encoding
     );
 }
```
- to enable using tls 
```rust
use water_http::server:: ServerConfigurations;

fn enabling_tls(config:&mut ServerConfigurations){
    // you could set multiple tls ports
    config.tls_ports = vec![8084,443];
    // specify the location of certificate.crt file and private.key
    config.set_tls_certificate(
        "./ssl/local_ssl/certificate.crt",
        "./ssl/local_ssl/private.key",
        None
    );
}
```

 Note: you could choose your needed file from examples folder
 

# Starting
 
- firstly you need to define Controllers Root as we
  explain in [Concepts](#concepts)

- create controller using `water_http::WaterController` macro
```rust
 use water_http::WaterController;
/// we use crate key word because this macro will 
/// encapsulate everything inside new mod
/// and holder is the type that we defined in the previous step

WaterController! {
    holder -> crate::MainHolderType,
    name -> RootController,
    functions -> {
        GET => / => any_thing(context) async {
            _=context.send_str("hello world").await;
        }
    }
  }
```

- now inside main fn in rust we will create configurations
 and run the server app
```rust
 #[tokio::main]
 async fn main(){
    let  configs = ServerConfigurations::bind("127.0.0.1",8084);
    water_http::RunServer!(
        configs,
        MAIN_ROOT,
        RootController
    );
}
```
# Basic Example 
 ```rust 
 use std::collections::HashMap;
use water_http::server:: ServerConfigurations;
use water_http::{InitControllersRoot, WaterController};

type MainHolderType = HashMap<String,String>;
InitControllersRoot!{
    name:MAIN_ROOT,
    holder_type:MainHolderType,
}


#[tokio::main]
async fn main() {

    let  configs = ServerConfigurations::bind("127.0.0.1",8084);

    water_http::RunServer!(
        configs,
        MAIN_ROOT,
        MainController
    );
}
WaterController! {
    holder -> crate::MainHolderType,
    name -> MainController,
    functions -> {
        GET => / => any_thing(context) async {
            _=context.send_str("hello world").await;
        }
    }

}
```


# Notes :
 - water_http use tokio runtime for multithreading tasks
 - using WaterController macro need to have (name,holder,function) in order arrangement but the following properties no needs for that
 - you may need to install [cmake](https://cmake.org/download/) and [clang](https://clang.llvm.org/) compiler for compiling
 - in linux make sure to have build-essential and cmake and you sometime gcc 
  to do install any of them 
```shell
sudo apt-get install build-essential cmake 
```
 - if you want to create fn which take context as parameter
 
  you would need to parse parameters as following
 ```rust
 use water_http::server::HttpContext;
 type MainHolderType = crate::MainHolderType;
 async fn handle_context<'context>(
     context:&mut HttpContext<'context,MainHolderType,16,16>){
     // Main Holder type it`s what you defined when InitControllersRoot
     // 16 and 16 is the headers and query length ,and they are the default values
     // if you need to change them then you need to chane the MainRoot defined lengths or
 }
```
 or you could use also
 ```rust
 use water_http::server::HttpContext;
 type MainHolderType = crate::MainHolderType;
 async fn handle_context<'context,
  const HL:usize,
  const QL:usize   
 >(
     context:&mut HttpContext<'context,
         MainHolderType,HL,QL>){
 }
``` 

## Writing Responses
 - using sender 
 ```rust
 
  water_http::functions_builder!{
      
      
      pub async fn send_response(context){
          let mut sender = context.sender();
          if sender.send_str("hello this is api response").await.is_ok() {
             println!("response sent successfully");
        }
      } 
      
      
 }
```
and there is alot of functions that facilitate sending responses like sending json or file from public directory
```rust
  water_http::functions_builder!{
    
    
  pub async fn send_response(context){
          let mut sender = context.sender();
          let file = FileRSender::new("./public/text/test1.jpg");  
           if sender.send_file(file).await.is_success() {
            println!("file sent successfully");
        }
      } 
    
    
 }
```

 - using context sending methods
```rust
 water_http::functions_builder! {
    
    
       pub async fn send_response(context){
        if context.send_str("hi this is api response").await.is_ok() {
            println!("response sent successfully");
        }
      }  
    
      pub async fn send_file(context){
          
        if context.send_file(FileRSender::new("./public/text/test1.txt")).await.is_success() {
            println!("response sent successfully");
        }
      }   
    
    
}
``` 

- using water_http macros
 ```rust
 
 water_http::functions_builder! {
     
     
      pub async fn send_response(context){
          
        response!(context -> "hi this is api response") ;
      }  
    
      pub async fn send_file(context){
         response!(context file -> "./public/text/test1.txt");
      }   
     
     
 }
```
also you could use `response!(context json -> jsonValue );` to send json response
## Writing Controllers Functions styles


```rust
use water_http::WaterController;
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
            _=context.send_str("hello world").await;
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

        // in this case GET is the method and 'api/v1/users/' the path
        // and this type could inject string parameter like
        // 'api/v1/users/t22' so t22 is now represented by id variable
        GET -> api/v1/users/{id} -> get_user(context) async{
            println!("user id is {id}");
            super::get_response(context).await;
        }

        // in this case POST is Method and api/auth/login is path
        POST => api/auth/login => login_handler(context) async {
            response!(context -> "hello from login api endpoint");
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
}
// notice that writing methods like POST,post,Post,posT,POst
// it would give the same result cause the framework has auto under table requests handler
```


## Full Code example 

```rust
use std::collections::HashMap;
use water_http::server::{ServerConfigurations};
use water_http::{InitControllersRoot, response, WaterController};
use water_http::http::HttpSenderTrait;


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


        // in this case path is "/" while method is GET
        "/" hello_world(context){
            _=context.send_str("hello world").await;
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
         // in this example GET is the method and api/v4 is path

        #[GET,api/v5]
        async get_profiles_v5(context)  [super::get_response]

        #[GET,api/6]
        async get_profiles_6(context)  {
            super::get_response(context).await;
        }

        getFile(context) [super::send_files]

    }
    extra_code->(..{

    }),
    middleware-> (context {
        println!("middleware function invoked");

        if let Some(ref holder) = context.holder {
            if holder.user.is_some() {
                println!("user is authenticated");
            }
        }

        if 1 == 1  { return server::MiddlewareResult::Pass }

        response!(context -> "invalid middleware passing");
        server::MiddlewareResult::Stop
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








```