

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
 water_http use tokio runtime for multithreading tasks
 and there is a version that use coroutines which handle 
 more requests and more efficiently but every thing comes with pay
 so the cons is that using memory management is complex and sometimes it
 leads to system block