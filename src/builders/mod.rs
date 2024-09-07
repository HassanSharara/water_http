
#![allow(non_snake_case)]


/// for building middleware inside each capsule with very easy implementation
/// you could use this
/// MiddlewareBuilder![ context async {} ]
/// or MiddlewareBuilder![ (context) async {} ]
/// or MiddlewareBuilder![ (context) => async {} ]
/// or MiddlewareBuilder![ context => async {} ]
#[macro_export]
macro_rules! MiddlewareBuilder {
    [ $context:ident  $async:tt $bb:block ]=>{
            water_http::MiddlewareBuilder!($context=> $async $bb)
    };
    [ ($context:ident) $async:tt  $bb:block ]=>{
            water_http::MiddlewareBuilder!($context=>$async $bb)
    };
    [ ($context:ident) => $async:tt $bb:block ]=>{
            water_http::MiddlewareBuilder!($context=>$async $bb)
    };
    ( $context:ident => $async:tt $bb:block )=>{
            |$context : &mut ___CONTEXT| Box::pin( $async move $bb)
    };
}


/// it`s for building controllers
/// , and you could use it with multi patterns
/// like
///   WaterController! {
///    holder -> path_to_your_holder ,
///    name -> name_of_your_controller,
///    functions {
///    and these could be
///     path => any_fn_name(context) async {}
///   or GET => path => any_fn_name(context) async {}
///   or  "path" => any_fn_name(context) async {}
///   or  GET => "path" => any_fn_name(context) async {}
///   or #[route(GET,path)]
///    pub async fn fn_name(context) {
///    }

///   or #[route(path)]
///    pub async fn fn_name(context) {
///    }
///
///  },
///
/// }
///
/// after function attribute we could provide prefix -> "any_prefix",
/// or we could also provide middleware -> MiddlewareBuilder! ++
/// or apply_parents_middlewares->true,
/// # notice that this will create new mod with the name that you provide
/// also if you want to use a code inside this mod
/// you could give extra_code attribute before holder attribute
#[macro_export]
macro_rules! WaterController {

    // pattern
    {
         holder -> $holder_type:path ,
         name -> $name:ident,
         functions ->  {
            $(
           $method:ident => $($path:tt)/+ => $fn_name:ident($para:ident) $async:tt  {
            $($body_tokens:tt)*
        }
        )*
        },
        $($attributes:ident -> $data:expr ,)*} =>  {
        water_http::WaterController! {
            holder -> $holder_type,
            name -> $name,
            functions -> {
                $(
                 #[route($method,$($path)/+)]
                pub $async fn $fn_name($para)  {
                     $($body_tokens)*
                 })*
            },
            $($attributes -> $data ,)*
        }
    };




    // pattern
    {
        extra_code-> {$($code:tt)*},
        holder -> $holder_type:path ,
        name -> $name:ident,
        functions ->  {
            $(
        $method:ident => $($path:tt)/+ => $fn_name:ident($function_para:ident) $async:tt  {
            $($body_tokens:tt)*
        }
        )*
        },
        $($attributes:ident -> $data:expr ,)*} =>  {
        water_http::WaterController! {
            extra_code -> {
                $($code)*
            },
            holder -> $holder_type,
            name -> $name,
            functions -> {
                $(
                 #[route($method,$($path)/+)]
                pub $async fn $fn_name($function_para)  {
                     $($body_tokens)*
                 })*
            },
            $($attributes -> $data ,)*
        }
    };



       // pattern
        {
        holder -> $holder_type:path ,
        name -> $name:ident,
        functions ->  {
            $(
         $($path:tt)/+ => $fn_name:ident($function_para:ident) $async:tt  {
            $($body_tokens:tt)*
        }
        )*
        },
        $($attributes:ident -> $data:expr ,)*} =>  {
        water_http::WaterController! {
            holder -> $holder_type,
            name -> $name,
            functions -> {
                $(
                 #[route($($path)/+)]
                pub $async fn $fn_name($function_para)  {
                     $($body_tokens)*
                 })*
            },
            $($attributes -> $data ,)*
        }
    };

      // pattern
        {
        extra_code-> {$($code:tt)*},
        holder -> $holder_type:path ,
        name -> $name:ident,
        functions ->  {
            $(
         $($path:tt)/+ => $fn_name:ident($function_para:ident) $async:tt  {
            $($body_tokens:tt)*
        }
        )*
        },
        $($attributes:ident -> $data:expr ,)*} =>  {
        water_http::WaterController! {
            extra_code -> {
                $($code)*
            },
            holder -> $holder_type,
            name -> $name,
            functions -> {
                $(
                 #[route($($path)/+)]
                pub $async fn $fn_name($function_para)  {
                     $($body_tokens)*
                 })*
            },
            $($attributes -> $data ,)*
        }
    };



     // pattern
       {
        holder -> $holder_type:path ,
        name -> $name:ident,
        functions ->  {
            $(
            #[route($($path:tt)/+)]
            pub $async:tt fn $fn_name:ident($para:ident) {
                $($body_tokens:tt)*
            }
          )*
        },
        $($attributes:ident -> $data:expr ,)*} =>  {
        water_http::WaterController! {
            holder -> $holder_type,
            name -> $name,
            functions -> {
                $(
                 #[route(GET,$($path)/+)]
                pub $async fn $fn_name($para)  {
                     $($body_tokens)*
                 })*
            },
            $($attributes -> $data ,)*
        }
    };



     // pattern
       {
        extra_code-> {$($code:tt)*},
        holder -> $holder_type:path ,
        name -> $name:ident,
        functions ->  {
            $(
            #[route($($path:tt)/+)]
            pub $async:tt fn $fn_name:ident($function_para:ident) {
                $($body_tokens:tt)*
            }
          )*
        },
        $($attributes:ident -> $data:expr ,)*} =>  {
        water_http::WaterController! {
            extra_code-> {$($code)*},
            holder -> $holder_type,
            name -> $name,
            functions -> {
                $(
                 #[route(GET,$($path)/+)]
                pub $async fn $fn_name($function_para)  {
                     $($body_tokens)*
                 })*
            },
            $($attributes -> $data ,)*
        }
    };





     //pattern
        {
        holder -> $holder_type:path ,
        name -> $name:ident,
        functions ->  {
            $(
            #[route($method:ident,$($path:tt)/+)]
            pub $async:tt fn $fn_name:ident($function_para:ident) {
                $($body_tokens:tt)*
            }
          )*
        },
        $($attributes:ident -> $data:expr ,)*

      } => {
            water_http::WaterController! {
                extra_code ->{},
                holder -> $holder_type,
                name -> $name,
               functions -> {
                $(
                 #[route($method,$($path)/+)]
                pub $async fn $fn_name($function_para)  {
                     $($body_tokens)*
                 })*
              },
             $($attributes -> $data ,)*

            }
        };

     //result pattern
      {
        extra_code -> {
            $($code:tt)*
        },
        holder -> $holder_type:path ,
        name -> $name:ident,
        functions ->  {
            $(
            #[route($method:ident,$($path:tt)/+)]
            pub $async:tt fn $fn_name:ident($para:ident) {
                $($body_tokens:tt)*
            }
          )*
        },
        $($attributes:ident -> $data:expr ,)*

      } =>  {

         #[allow(non_snake_case)]
          pub mod $name {

               #![allow(non_snake_case)]
               use std::fmt::format;
               pub type ___CONTEXTHOLDER = $holder_type;
               pub type ___CONTEXT = water_http::framework_http::HttpContext<___CONTEXTHOLDER>;
               pub type __WaterCapsuleController = water_http::WaterCapsuleController<___CONTEXTHOLDER>;

               $($code)*

               $(
               pub async fn $fn_name($para:&mut ___CONTEXT)   {
                water_http::path_setter!($para->$($path)/+);
                   $($body_tokens)*
               }
               )*

               pub fn build() -> __WaterCapsuleController{
                let mut  controller = __WaterCapsuleController::new();
                controller.functions = vec![
                     $(
                     (
                  stringify!($method).replace(" ","").replace('\"',""),
                  stringify!($($path)/+).replace(" ","").replace('\"',""),
                  | context | Box::pin(
                      $async move {
                          let _ = $fn_name(context).await;
                      }
                  )
                 ),)*
                 ];
                 $(
                  controller.$attributes = water_http::framework_att_setter!($attributes->$data);
                 )*
                 controller
            }
          }
    };




}

/// for re assigning attributes given by [WaterController]
#[macro_export]
macro_rules! framework_att_setter {
    (prefix -> $data:expr) => {
      String::from($data).into()
    };



   ( middleware -> $data:expr) => {
      Some($data)
    };
    ($attr:ident->$data:expr) => {
       $data
    };
}

/// for setting path from another macro
/// it`s for another macros call so do not worry about it ,
/// we just had to make it public for re calling it from another macros
#[macro_export]
macro_rules! path_setter {
    [$context_name:ident () {$path_item:tt} ]=>{
        let $path_item = &$context_name.path_params_map.get(stringify!($path_item)).unwrap();
    };
     [$context_name:ident () $path_item:tt]=>{
    };
    ( $context_name:ident -> $($p_item:tt)/+) => {
        $(
            water_http::path_setter![$context_name () $p_item ];
        )+
    };
}