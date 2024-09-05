# Water Http Framework is the most easy and fast web framework that built on both http1.1 and http2 and it`s supporting them automatically [no extra implementations required]



# so lets talk about installing firstly

- you need to add water_http crate,
and we recommend you to use the terminal or CMD in windows
```shell
 cargo add water_http
```
or you could use cargo.toml file and add the latest version of water_http
```toml
water_http="[latest_version_number]"
```

- then you need to add tokio
```shell
cargo add tokio --features=full
```
- (optional)  if you wants to use json structs just add serde and serde_json crates
```shell
cargo serde serde_json
```

# Features 
- auto handling for h2 or h1.1 under the hood
- very fast 
- very easy to use
- auto supporting for most used web encoding (zstd,brotli,deflate,gzip)
- provide simple routing structure with many patterns of write
- naming routes and redirecting to these routes by their names using one single method 
- support tls or ssl implementation for secure connections without any 
 confusing
- provide easy approaches for saving and downloading files from and to the server
- support videos streaming for web pages videos
- easy built in middlewares and prefixes for your controllers 
  to build easy and clean code with high efficiency results
- multi pattern ways to generate your code and very easy approaches
- support controlling low actions like block custom ip addresses from connecting your server or strict your service to custom ip addresses
- thresholding actions like the threshold the maximum size of using compressing algorithms when sending response back considering clients encoding support 
# Now Let`s implement Rust Code

- first implementation
```rust
use std::collections::HashMap;

use water_http::{WaterController, DataHolderInitializer, RunServer, WaterServerConfigurations};
/// notice that this data holder could be used to parse data from middleware to another or 
/// from controller to controller , so you could choose what ever you want from data type ,
/// or you could make it u8 to have single byte if you don`t use it
DataHolderInitializer!(HashMap<String,String>);
#[tokio::main]
async fn main() {
    let config = WaterServerConfigurations::bind("127.0.0.1",8084);
    RunServer!(
        config ,
        MainController::build()
    );
}


WaterController! {
    holder -> crate::_WaterContextDataHolder,
    name -> MainController,
    functions -> {
       GET => / => any_thing_you_want_name(context)async{
          let _ =   context.send_str_data("Hello ,World!",true).await;
       }
    },
}
```

- second one
```rust
use std::collections::HashMap;

use water_http::{WaterController, DataHolderInitializer, RunServer, WaterServerConfigurations};
/// notice that this data holder could be used to parse data from middleware to another or 
/// from controller to controller , so you could choose what ever you want from data type ,
/// or you could make it u8 to have single byte if you don`t use it
DataHolderInitializer!(HashMap<String,String>);
#[tokio::main]
async fn main() {
    /// it will listen to port 80 
    let config = WaterServerConfigurations::default();
    RunServer!(
        config,
        MainController::build()
    );
}


WaterController! {
    holder -> crate::_WaterContextDataHolder,
    name -> MainController,
    functions -> {
       GET => / => any_thing_you_want_name(context)async{
          let _ =   context.send_str_data("Hello ,World!",true).await;
       }
    },
}

```

- third one
```rust
use std::collections::HashMap;

use water_http::{WaterController, DataHolderInitializer, RunServer};
/// notice that this data holder could be used to parse data from middleware to another or 
/// from controller to controller , so you could choose what ever you want from data type ,
/// or you could make it u8 to have single byte if you don`t use it
DataHolderInitializer!(HashMap<String,String>);
#[tokio::main]
async fn main() {
    RunServer!(MainController::build());
}


WaterController! {
    holder -> crate::_WaterContextDataHolder,
    name -> MainController,
    functions -> {
       GET => / => any_thing_you_want_name(context)async{
          let _ =   context.send_str_data("Hello ,World!",true).await;
       }
    },
}
```
- forth 
```rust
use std::collections::HashMap;

use water_http::{WaterController, DataHolderInitializer, RunServer,MiddlewareBuilder};
// notice that this data holder could be used to parse data from middleware to another or
// from controller to controller , so you could choose what ever you want from data type ,
// or you could make it u8 to have single byte if you don`t use it
DataHolderInitializer!(HashMap<String,String>);
#[tokio::main]
async fn main() {
    RunServer!(MainController::build());
}


WaterController! {
    holder -> crate::_WaterContextDataHolder,
    name -> MainController,
    functions -> {
       GET => / => any_thing_you_want_name(context)async{
          let _ =   context.send_str_data("Hello ,World!",true).await;
       }
    },
    children->vec![
        super::SecondController::build(),
        super::ThirdOne::build(),
    ],
}


// notice that when you set new water controller you are creating new mod ,  so you need to either use extra_code
// to add your wanted code or use [super::*] or [crate::*]
WaterController!{
    extra_code-> {
        use water_http::structure::MiddlewareResult;

    },
    holder -> crate::_WaterContextDataHolder,
    name -> SecondController,
    functions -> {

        GET => test_second => test_second_function(context)async{
         let _ =   context.send_str_data("Second Response",true).await;
        }


    },
    prefix -> "second controller",

    // setting middleware algorithm
    middleware -> super::MiddlewareBuilder!(
        context => async {
            let authorization = context.get_from_headers_as_string("Authorization");
            match authorization {

            Some(authorization) => {
                    if authorization == "some_thing"{
                        context.data_holder.as_mut()
                        .unwrap()
                        .insert(
                            "user_id".to_string(),
                            "whatever".to_string()
                        );
                    }
                    MiddlewareResult::Pass
                }
            None => MiddlewareResult::Stop
            }
        }
    ),
}


// notice that when you set new water controller you are creating new mod ,  so you need to either use extra_code
// to add your wanted code or use [super::*] or [crate::*]
WaterController!{
    extra_code-> {},
    holder -> crate::_WaterContextDataHolder,
    name -> ThirdOne,
    functions -> {
        GET => test_second => any_thing_you_want_name(context)async{
         let _ =   context.send_str_data("Third Response",true).await;
        }
    },
    prefix->"third",
}
```


and there is a lot of features like downloading files
and setting public directory and streaming videos 
and sending many files as response and sending customs responses
and also setting custom ip addresses that would be only them who can connect the server
or restricting ip addresses from connecting the server
and also there is cli application and very easy frontend 
building tools are coming soon

```rust
 use water_http::WaterServerConfigurations;
 let mut config = WaterServerConfigurations::default();
    config.public_files_path = String::from("./public");
    config.restricted_ips = Some(WaterIpAddressesRestriction::OnlyAllowedIps(vec![
        "127.0.0.1".to_string()
    ]));
    config.set_tls_certificate(
        "./ssl/certificate.crt",
        "./ssl/private.key",
        None,
    );
    // for prevent downloading public files from public directory 
    config.do_not_even_check_public_resources = true;
```

```rust
use std::collections::HashMap;

use water_http::{WaterController, DataHolderInitializer, RunServer,get_route_by_name};
// notice that this data holder could be used to parse data from middleware to another or
// from controller to controller , so you could choose what ever you want from data type ,
// or you could make it u8 to have single byte if you don`t use it
DataHolderInitializer!(HashMap<String,String>);
#[tokio::main]
async fn main() {

    RunServer!(MainController::build());
}
WaterController! {
    holder -> crate::_WaterContextDataHolder,
    name -> MainController,
      functions -> {

        GET => test_second => test_second_function(context) async {
         let _ =   context.send_str_data("Second Response",true).await;
        }

        GET => test/newpath => tn(context) async {
            // redirecting to ---> our_new_path
            let _route_url = super::get_route_by_name("our_new_path",None);
            println!("{:?}",_route_url);
            // if you want to directly redirect url by using route name
            let _ = context.redirect_by_route_name("our_new_path",None).await;


            // redirecting to ---> test_v
            let test_v1_atts = [("id","your_custom_id")];
            let _ = context.redirect_by_route_name("test_v",Some(&test_v1_atts));

        }



        GET_test_v => v1/{id} => test_v_function_name(context) async {
            let res = format!("id =  {id}");
            let _ = context.send_string_data(res,true).await;
        }



        GET_our_new_path => hello => h(context) async {
         let _ =   context.send_str_data("test route response",true).await;
        }


    },
}
```