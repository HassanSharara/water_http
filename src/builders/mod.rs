
#![allow(non_snake_case)]


#[macro_export]
macro_rules! MiddlewareBuilder {
    [ $context:ident  $async:tt $bb:block ]=>{
            knife_web_framework_lib::MiddlewareBuilder!($context=> $async $bb)
    };
    [ ($context:ident) $async:tt  $bb:block ]=>{
            knife_web_framework_lib::MiddlewareBuilder!($context=>$async $bb)
    };
    [ ($context:ident) => $async:tt $bb:block ]=>{
            knife_web_framework_lib::MiddlewareBuilder!($context=>$async $bb)
    };
    ( $context:ident => $async:tt $bb:block )=>{
            |$context : &mut ___CONTEXT| Box::pin( $async move $bb)
    };
}
#[macro_export]
macro_rules! FrameWorkControllerBuilder {
    {
        holder -> $holder_type:path ,
        name -> $name:ident,
        functions ->  {
            $(
        $method:ident => $($path:tt)/+ => $fn_name:ident($function_para:ident) async  {
            $($body_tokens:tt)*
        }
        )*
        },
        $($attributes:ident -> $data:expr ,)*} =>  {
        knife_web_framework_lib::FrameWorkControllerBuilder! {
            holder -> $holder_type,
            name -> $name,
            functions -> {
                $(
                 #[route($method,$($path)/+)]
                pub async fn $fn_name($function_para)  {
                     $($body_tokens)*
                 })*
            },
            $($attributes -> $data ,)*
        }
    };



        {
        holder -> $holder_type:path ,
        name -> $name:ident,
        functions ->  {
            $(
         $($path:tt)/+ => $fn_name:ident($function_para:ident) async  {
            $($body_tokens:tt)*
        }
        )*
        },
        $($attributes:ident -> $data:expr ,)*} =>  {
        knife_web_framework_lib::FrameWorkControllerBuilder! {
            holder -> $holder_type,
            name -> $name,
            functions -> {
                $(
                 #[route($($path)/+)]
                pub async fn $fn_name($function_para)  {
                     $($body_tokens)*
                 })*
            },
            $($attributes -> $data ,)*
        }
    };




       {
        holder -> $holder_type:path ,
        name -> $name:ident,
        functions ->  {
            $(
            #[route($($path:tt)/+)]
            pub async fn $fn_name:ident($function_para:ident) {
                $($body_tokens:tt)*
            }
          )*
        },
        $($attributes:ident -> $data:expr ,)*} =>  {
        knife_web_framework_lib::FrameWorkControllerBuilder! {
            holder -> $holder_type,
            name -> $name,
            functions -> {
                $(
                 #[route(GET,$($path)/+)]
                pub async fn $fn_name($function_para)  {
                     $($body_tokens)*
                 })*
            },
            $($attributes -> $data ,)*
        }
    };





      {
        holder -> $holder_type:path ,
        name -> $name:ident,
        functions ->  {
            $(
            #[route($method:ident,$($path:tt)/+)]
            pub async fn $fn_name:ident($para:ident) {
                $($body_tokens:tt)*
            }
          )*
        },
        $($attributes:ident -> $data:expr ,)*

      } =>  {

         #[allow(non_snake_case)]
          pub mod $name {
               #![allow(non_snake_case)]
               pub type ___CONTEXTHOLDER = $holder_type;
               pub type ___CONTEXT = knife_web_framework_lib::framework_http::HttpContext<___CONTEXTHOLDER>;
               pub type __HttpContextRController = knife_web_framework_lib::structure::HttpContextRController<___CONTEXTHOLDER>;

               $(
               #[pms::route($method,$($path)/+)]
               pub async fn $fn_name($para:&mut ___CONTEXT)   {
                knife_web_framework_lib::path_setter!($para->$($path)/+);
                   $($body_tokens)*
               }
               )*
               pub fn build() -> __HttpContextRController{

                let controller = __HttpContextRController{
                 $(
                 $attributes:knife_web_framework_lib::framework_att_setter!($attributes->$data),
                 )*
                 functions:vec![
                   $(
                  (stringify!($method).replace(" ","").replace('\"',"").to_uppercase(),
                  stringify!($($path)/+).replace(" ","").replace('\"',""),
                  |context| Box::pin(
                      async move {
                          let _ = $fn_name(context).await;
                      }
                  )),
                   )*
                 ],
                 ..__HttpContextRController::new()
              };
            controller
            }
          }
    };

}

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
#[macro_export]
macro_rules! path_setter {
    [$context_name:ident () {$path_item:tt} ]=>{
        let $path_item = &$context_name.path_params_map.get(stringify!($path_item)).unwrap();
    };
     [$context_name:ident () $path_item:tt]=>{
    };
    ( $context_name:ident -> $($p_item:tt)/+) => {
        $(
            knife_web_framework_lib::path_setter![$context_name () $p_item ];
        )+
    };
}