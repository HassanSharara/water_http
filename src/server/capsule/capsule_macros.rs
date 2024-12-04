
#![allow(non_snake_case)]


use crate::server::HttpContext;
/// Initiating Controller Root is very important to detect the max important requirements for
/// building controller struct
/// you need to know that [headers_length] means that how many headers could the framework read
/// at single request and the reason why we initiate something like that is to provide
/// a good structure for framework and allocating memory in stack instead of heap
/// to provide fast operation which is very importing when dealing with
/// quit high load of traffic and also for security and protecting against
/// headers  attackers
/// and the [query_length] is also the same
/// but it`s works on the incoming request path query and for example
/// [.com/post?name=hello&description=desc]
#[macro_export]
macro_rules! InitControllerRoot {

    {
    /// defining the name of static Controller Root and it`s should be uppercase
    name: $name:ident ,
    holder_type:$holder:ty,
     } => {
       InitControllerRoot! {
           name:$name,
           holder_type:$holder,
           headers_length:16,
           query_length:16
       }
    };
    {
    /// defining the name of static Controller Root and it`s should be uppercase
    name: $name:ident ,
    holder_type:$holder:ty,
    headers_length:$hl:literal,
    query_length:$ql:literal
     } => {
        pub static mut $name:Option<water_http::server::CapsuleWaterController<$holder,$hl,$ql>> = None;

    };
}



/// for running server in appropriate way,
/// and it takes 3 arguments
/// - the first one is config [`water_http::server::ServerConfigurations`]
/// - the second one is Root which is a defining of holder of entry controller
/// - the third one is the entry point of the server which is the start point controller
/// # Note :
///  you really need to make sure to not interrupt or change values in root holder or the second arguments
/// it`s may lead to very unpredicted behavior cause it designed to provide a higher speed not for editing specially during
/// multithreading and multiprocessing operations by the framework
#[macro_export]
macro_rules! RunServer {
    (
        $config:expr,
        $controller:expr,
        $entry:ident
    ) => {
        unsafe {
            let co = $entry::build();
            $controller = Some(co);
           water_http::server::run_server(
            $config,
            $controller.as_mut().unwrap()
           ).await;
        };

    };
}


/// constructing functions builder
#[macro_export]
macro_rules! FunctionsMacroBuilder {
      (
     functions -> {
         $(
           #[route($method:ident,$($path:tt)/+)]
           fn $fn_name:ident($context:ident) {
              $($function_body_tokens:tt)*
          }
         ),*
     }
    ) =>{
            water_http::FunctionsMacroBuilder!(
             functions -> {
                 $(
                 $method => $($path)/+ => $fn_name($context) $async {
                     $($function_body_tokens)*
                 }
                 ),*
             }
         );
     };

      (
     functions -> {
         $(
           #[route($method:ident,$($path:tt)/+)]
           pub async  $fn_name:ident($context:ident) {
              $($function_body_tokens:tt)*
          }
         ),*
     }
    ) =>{
            water_http::FunctionsMacroBuilder!(
             functions -> {
                 $(
                 $method => $($path)/+ => $fn_name($context) $async {
                     $($function_body_tokens)*
                 }
                 ),*
             }
         );
     };

     (
     functions -> {
         $(
           #[route($method:ident,$($path:tt)/+)]
           pub async fn $fn_name:ident($context:ident) {
              $($function_body_tokens:tt)*
          }
         ),*
     }
    ) =>{
            water_http::FunctionsMacroBuilder!(
             functions -> {
                 $(
                 $method => $($path)/+ => $fn_name($context) $async {
                     $($function_body_tokens)*
                 }
                 ),*
             }
         );
     };

     (
     functions -> {
         $(
           #[route($method:ident,$($path:tt)/+)]
           $async:tt $fn_name:ident($context:ident) {
              $($function_body_tokens:tt)*
          }
         ),*
     }
    ) => {
          water_http::FunctionsMacroBuilder!(
             functions -> {
                 $(
                 $method => $($path)/+ => $fn_name($context) $async {
                     $($function_body_tokens)*
                 }
                 ),*
             }
         );
    };
    (
     functions -> {
         $(
           #[route($method:ident,$($path:tt)/+)]
           $fn_name:ident($context:ident) {
              $($function_body_tokens:tt)*
          }
         ),*
     }
    ) => {
          water_http::FunctionsMacroBuilder!(
             functions -> {
                 $(
                 $method => $($path)/+ => $fn_name($context) async {
                     $($function_body_tokens)*
                 }
                 ),*
             }
         );
    };


         (
     functions -> {
         $(
            $($path:tt)/+ => $fn_name:ident($context:ident) {
              $($function_body_tokens:tt)*
          }
         ),*
     }
    ) => {
         water_http::FunctionsMacroBuilder!(
             functions -> {
                 $(
                 GET => $($path)/+ => $fn_name($context) async {
                     $($function_body_tokens)*
                 }
                 ),*
             }
         );
    };


         (
     functions -> {
         $(
            $($path:tt)/+ => $fn_name:ident($context:ident) $async:tt {
              $($function_body_tokens:tt)*
          }
         ),*
     }
    ) => {
         water_http::FunctionsMacroBuilder!(
             functions -> {
                 $(
                 GET => $($path)/+ => $fn_name($context) $async {
                     $($function_body_tokens)*
                 }
                 ),*
             }
         );
    };

     (
     functions -> {
         $(
           $method:ident => $($path:tt)/+ => $fn_name:ident($context:ident) {
              $($function_body_tokens:tt)*
          }
         ),*
     }
    ) => {
         water_http::FunctionsMacroBuilder!(
             functions -> {
                 $(
                 $method => $($path)/+ => $fn_name($context) async {
                     $($function_body_tokens)*
                 }
                 ),*
             }
         );
    };



    // for building
    (
     functions -> {
         $(
           $method:ident => $($path:tt)/+ => $fn_name:ident($context:ident) $async:tt {
              $($function_body_tokens:tt)*
          }
         ),*
     }
    ) => {

        $(pub $async fn $fn_name<'context,CONTEXT_HOLDER:Send + 'static,const header_length:usize,const query_length:usize>
        ($context:&mut water_http::server::HttpContext<'context,CONTEXT_HOLDER,header_length,query_length>) {
            water_http::path_setter!($context->$($path)/+);
            $($function_body_tokens)*
        }
        )*



      pub fn build<const header_length:usize,const query_length:usize>()->
        water_http::server::CapsuleWaterController<Holder,header_length,query_length>{
          let mut controller  = water_http::server::CapsuleWaterController::new();
            $(
             controller.push_handler(
                 (
                     stringify!($method).replace('"',"").replace(" ","").to_uppercase(),
                     stringify!($($path)/+).replace('"',"").replace(" ","").replace("//","/"),
                     | context | Box::pin( async move {
                         $fn_name(context).await;
                     })
                 )
             );
            )*
            check_up_controller(&mut controller);
            controller
       }
    };


}

/// generating internal needed code
#[macro_export]
macro_rules! CheckupAutoGenerator {
    ( $controller:path >> prefix->$value:expr) => {
        $controller.prefix = Some($value);
    };

    ( $controller:path >> apply_parents_middlewares->$value:expr) => {
        $controller.apply_parents_middlewares = $value;
    };

    ( $controller:path >> middleware-> $context:ident $block:block) => {
        $controller.middleware = Some(
            |$context :&mut HttpContext<Holder,header_length,query_length>| Box::pin( async move  $block)
        );
    };

    ($controller:path >> children-> [$($child:ident),*] )=>{

        $(
        $controller.push_controller(super::$child::build());
        )*
    };

    ($controller:path >> extra_code-> .. {$($tokens:tt)* }) => {}


}
/// for checking if extra code need to be build
#[macro_export]
macro_rules! CheckExtraCode {

    (extra_code ->  .. {$($b:tt)*} ) => {
        $($b)*
    };
   ($key:tt -> $($tokens:tt)* ) => {
    };
}
#[macro_export]
macro_rules! WaterController {
    {
     holder -> $holder:path,
     name -> $name:ident,
     functions -> {$($function_tokens:tt)*}
     $($key:tt -> ($($value:tt)*)),*
    } => {
        pub mod $name {

            use water_http::http::HttpSenderTrait;
            use  water_http::server::HttpContext;
            pub type Holder = $holder;

            water_http::FunctionsMacroBuilder!(
                functions -> {$($function_tokens)*}
            );

            $(
           water_http::CheckExtraCode!(
               $key -> $($value)*
           );
           )*

            fn check_up_controller
            <const header_length:usize,const query_length:usize>(controller:&mut water_http::server::CapsuleWaterController<Holder,header_length,query_length>){
                $(
                water_http::CheckupAutoGenerator!(
                    controller >> $key -> $($value)*
                );
                )*
            }


        }
    };
}




/// for setting path from another macro
/// it`s for another macros call so do not worry about it ,
/// we just had to make it public for re calling it from another macros
#[macro_export]
macro_rules! path_setter {
    [$context_name:ident () {$path_item:tt} ]=>{
        let $path_item = &$context_name.get_from_generic_path_params(stringify!($path_item)).unwrap();
    };
     [$context_name:ident () $path_item:tt]=>{
    };


    ( $context_name:ident -> $($p_item:tt)/+) => {
        $(
            water_http::path_setter![$context_name () $p_item ];
        )+
    };
}





