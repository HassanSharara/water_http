
#![allow(non_snake_case)]


/// Initiating Controller Root is very important to detect the max important requirements for
/// building controller struct
/// you need to know that `headers_length` means that how many headers could the framework read
/// at single request and the reason why we initiate something like that is to provide
/// a good structure for framework and allocating memory in stack instead of heap
/// to provide fast operation which is very importing when dealing with
/// quit high load of traffic and also for security and protecting against
/// headers  attackers
/// and the query_length is also the same
/// but it`s works on the incoming request path query and for examples
/// [.com/post?name=hello&description=desc]
#[macro_export]
macro_rules! InitControllersRoot {

    {
    /// defining the name of static Controller Root and it`s should be uppercase
    name: $name:ident ,
    holder_type:$holder:ty,
     } => {
       InitControllersRoot! {
           name:$name,
           holder_type:$holder,
           headers_length:16,
           queries_length:16
       }
    };
    {
    /// defining the name of static Controller Root and it`s should be uppercase
    name: $name:ident ,
    holder_type:$holder:ty,
    headers_length:$hl:literal,
    queries_length:$ql:literal
     } => {
        pub static mut $name:Option<water_http::server::CapsuleWaterController<$holder,$hl,$ql>> = None;
    };
}



/// for creating route from given names of each route
/// and also matching provided keys and values given
#[macro_export]
macro_rules! route {
    ($key:expr) => {
        {
           water_http::server::___get_from_all_routes($key,None)
        }
    };
    ($key:expr,[$($k:expr => $value:expr),*]) => {
        {
            let mut map:std::collections::HashMap<&str,&str> = std::collections::HashMap::new();
            $(map.insert($k,$value);)*
            water_http::server::___get_from_all_routes($key,Some(map))
        }
    };
}





/// for running server in appropriate way,
/// and it takes 3 arguments
/// - the first one is config `ServerConfigurations`
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

#[doc(hidden)]
#[macro_export]
macro_rules! FunctionsMacroBuilderTow {
    // ----- Public entrypoint -----
    (
        @entry
        functions -> { $($items:tt)* }
    ) => {
        water_http::FunctionsMacroBuilderTow! (
            @parse [],
            $($items)*

        );
    };


    // first option
    (
        @parse [ $($acc:tt)* ],
        $method:ident => $($path:tt)/+ => $fn_name:ident ( $context:ident ) $async:tt { $($body:tt)* } $($rest:tt)*
    ) => {
        water_http::FunctionsMacroBuilderTow! (
            @parse [
                $($acc)*
                $method => $($path)/+ => $fn_name($context) $async { $($body)* },
            ],
            $($rest)*
        );
    };

      // first option
    (
        @parse [ $($acc:tt)* ],
        $method:ident -> $($path:tt)/+ -> $fn_name:ident ( $context:ident ) $async:tt { $($body:tt)* } $($rest:tt)*
    ) => {
        water_http::FunctionsMacroBuilderTow! (
            @parse [
                $($acc)*
                $method => $($path)/+ => $fn_name($context) $async { $($body)* },
            ],
            $($rest)*
        );
    };


     // second option
    (
        @parse [ $($acc:tt)* ],
        $method:ident => $($path:tt)/+ => $fn_name:ident ( $context:ident ) $async:tt [$fn_path:path] $($rest:tt)*
    ) => {
        water_http::FunctionsMacroBuilderTow! (
            @parse [
                $($acc)*
                $method => $($path)/+ => $fn_name($context) $async {
                    $fn_path($context).await;
                },
            ],
            $($rest)*
        );
    };

     // second option
    (
        @parse [ $($acc:tt)* ],
        $method:ident -> $($path:tt)/+ -> $fn_name:ident ( $context:ident ) $async:tt [$fn_path:path] $($rest:tt)*
    ) => {
        water_http::FunctionsMacroBuilderTow! (
            @parse [
                $($acc)*
                $method => $($path)/+ => $fn_name($context) $async {
                    $fn_path($context).await;
                },
            ],
            $($rest)*
        );
    };
       // third option
    (
        @parse [ $($acc:tt)* ],
        $method:ident => $($path:tt)/+ => $fn_name:ident ( $context:ident )  [$fn_path:path] $($rest:tt)*
    ) => {
        water_http::FunctionsMacroBuilderTow! (
            @parse [
                $($acc)*
                $method => $($path)/+ => $fn_name($context) async {
                    $fn_path($context).await;
                },
            ],
            $($rest)*
        );
    };

          // third option
    (
        @parse [ $($acc:tt)* ],
        $method:ident -> $($path:tt)/+ -> $fn_name:ident ( $context:ident )  [$fn_path:path] $($rest:tt)*
    ) => {
        water_http::FunctionsMacroBuilderTow! (
            @parse [
                $($acc)*
                $method => $($path)/+ => $fn_name($context) async {
                    $fn_path($context).await;
                },
            ],
            $($rest)*
        );
    };

    // Case 2: method => path => fn(context) { .. } , rest...  (implicit async)
    (
        @parse [ $($acc:tt)* ],
        $method:ident => $($path:tt)/+ => $fn_name:ident ( $context:ident ) { $($body:tt)* }  $($rest:tt)*
    ) => {
        water_http::FunctionsMacroBuilderTow!( @parse [
                $($acc)*
                $method => $($path)/+ => $fn_name($context) async { $($body)* },
            ],
            $($rest)*
        );
    };


    // Case 2: method => path => fn(context) { .. } , rest...  (implicit async)
    (
        @parse [ $($acc:tt)* ],
        $method:ident -> $($path:tt)/+ -> $fn_name:ident ( $context:ident ) { $($body:tt)* }  $($rest:tt)*
    ) => {
        water_http::FunctionsMacroBuilderTow!( @parse [
                $($acc)*
                $method => $($path)/+ => $fn_name($context) async { $($body)* },
            ],
            $($rest)*
        );
    };



    // case 3 without async path and fn name only
    // option 1
       (
        @parse [ $($acc:tt)* ],
         $method:ident => $fn_name:ident ( $context:ident ) { $($body:tt)* }  $($rest:tt)*
    ) => {
        water_http::FunctionsMacroBuilderTow!( @parse [
                $($acc)*
                $method => $fn_name => $fn_name($context) async { $($body)* },
            ],
            $($rest)*
        );
    };



    // case 3 without async path and fn name only
    // option 1
       (
        @parse [ $($acc:tt)* ],
         $method:ident -> $fn_name:ident ( $context:ident ) { $($body:tt)* }  $($rest:tt)*
    ) => {
        water_http::FunctionsMacroBuilderTow!( @parse [
                $($acc)*
                $method => $fn_name => $fn_name($context) async { $($body)* },
            ],
            $($rest)*
        );
    };

    (
        @parse [ $($acc:tt)* ],
         $method:ident => $fn_name:ident ( $context:ident ) [$fn_path:path]  $($rest:tt)*
    ) => {
        water_http::FunctionsMacroBuilderTow!( @parse [
                $($acc)*
                $method => $fn_name => $fn_name($context) async { $fn_path($context).await },
            ],
            $($rest)*
        );
    };
    (
        @parse [ $($acc:tt)* ],
         $method:ident -> $fn_name:ident ( $context:ident ) [$fn_path:path]  $($rest:tt)*
    ) => {
        water_http::FunctionsMacroBuilderTow!( @parse [
                $($acc)*
                $method => $fn_name => $fn_name($context) async { $fn_path($context).await },
            ],
            $($rest)*
        );
    };


    (
        @parse [ $($acc:tt)* ],
         $method:ident => $fn_name:ident ( $context:ident ) $async:tt [$fn_path:path] $($rest:tt)*
    ) => {
        water_http::FunctionsMacroBuilderTow!( @parse [
                $($acc)*
                $method => $fn_name => $fn_name($context) $async { $fn_path($context).await },
            ],
            $($rest)*
        );
    };
      (
        @parse [ $($acc:tt)* ],
         $method:ident -> $fn_name:ident ( $context:ident ) $async:tt [$fn_path:path] $($rest:tt)*
    ) => {
        water_http::FunctionsMacroBuilderTow!( @parse [
                $($acc)*
                $method => $fn_name => $fn_name($context) $async { $fn_path($context).await },
            ],
            $($rest)*
        );
    };

    (
        @parse [ $($acc:tt)* ],
         $method:ident => $fn_name:ident ( $context:ident ) $async:tt { $($body:tt)* }  $($rest:tt)*
    ) => {
        water_http::FunctionsMacroBuilderTow!( @parse [
                $($acc)*
                $method => $fn_name => $fn_name($context) $async { $($body)* },
            ],
            $($rest)*
        );
    };
    (
        @parse [ $($acc:tt)* ],
         $method:ident -> $fn_name:ident ( $context:ident ) $async:tt { $($body:tt)* }  $($rest:tt)*
    ) => {
        water_http::FunctionsMacroBuilderTow!( @parse [
                $($acc)*
                $method => $fn_name => $fn_name($context) $async { $($body)* },
            ],
            $($rest)*
        );
    };

       (
        @parse [ $($acc:tt)* ],
         $($path:tt)/+ => $fn_name:ident ( $context:ident ) { $($body:tt)* }  $($rest:tt)*
    ) => {
        water_http::FunctionsMacroBuilderTow!( @parse [
                $($acc)*
                GET => $($path)/+ => $fn_name($context) async { $($body)* },
            ],
            $($rest)*
        );
    };

       (
        @parse [ $($acc:tt)* ],
         $($path:tt)/+ -> $fn_name:ident ( $context:ident ) { $($body:tt)* }  $($rest:tt)*
    ) => {
        water_http::FunctionsMacroBuilderTow!( @parse [
                $($acc)*
                GET => $($path)/+ => $fn_name($context) async { $($body)* },
            ],
            $($rest)*
        );
    };

    // case 3 without async path and fn name only
    // option 2
       (
        @parse [ $($acc:tt)* ],
         $($path:tt)/+ => $fn_name:ident ( $context:ident ) [$fn_path:path] $($rest:tt)*
    ) => {
        water_http::FunctionsMacroBuilderTow!( @parse [
                $($acc)*
                GET => $($path)/+ => $fn_name($context) async {
                    $fn_path($context).await;
                },
            ],
            $($rest)*
        );
    };

    // case 3 without async path and fn name only
    // option 2
       (
        @parse [ $($acc:tt)* ],
         $($path:tt)/+ -> $fn_name:ident ( $context:ident ) [$fn_path:path] $($rest:tt)*
    ) => {
        water_http::FunctionsMacroBuilderTow!( @parse [
                $($acc)*
                GET => $($path)/+ => $fn_name($context) async {
                    $fn_path($context).await;
                },
            ],
            $($rest)*
        );
    };


    // case 3 with async path and fn name only
    // option 1
       (
        @parse [ $($acc:tt)* ],
         $($path:tt)/+ => $fn_name:ident ( $context:ident ) $async:tt { $($body:tt)* }  $($rest:tt)*
    ) => {

        water_http::FunctionsMacroBuilderTow!( @parse [
                $($acc)*
                GET => $($path)/+ => $fn_name($context) $async { $($body)* },
            ],
            $($rest)*
        );
    };

    // case 3 with async path and fn name only
    // option 1
       (
        @parse [ $($acc:tt)* ],
         $($path:tt)/+ -> $fn_name:ident ( $context:ident ) $async:tt { $($body:tt)* }  $($rest:tt)*
    ) => {

        water_http::FunctionsMacroBuilderTow!( @parse [
                $($acc)*
                GET => $($path)/+ => $fn_name($context) $async { $($body)* },
            ],
            $($rest)*
        );
    };
      // case 3 with async path and fn name only
    // option 2
       (
        @parse [ $($acc:tt)* ],
         $($path:tt)/+ => $fn_name:ident ( $context:ident ) $async:tt [$fn_path:path]  $($rest:tt)*
    ) => {
        water_http::FunctionsMacroBuilderTow!( @parse [
                $($acc)*
                GET => $($path)/+ => $fn_name($context) $async { $fn_path($context).await; },
            ],
            $($rest)*
        );
    };
   // case 3 with async path and fn name only
    // option 2
       (
        @parse [ $($acc:tt)* ],
         $($path:tt)/+ -> $fn_name:ident ( $context:ident ) $async:tt [$fn_path:path]  $($rest:tt)*
    ) => {
        water_http::FunctionsMacroBuilderTow!( @parse [
                $($acc)*
                GET => $($path)/+ => $fn_name($context) $async { $fn_path($context).await; },
            ],
            $($rest)*
        );
    };

       // case fn name only without async
      // option 1
       (
        @parse [ $($acc:tt)* ],
         $fn_name:ident ( $context:ident ){ $($body:tt)* }  $($rest:tt)*
    ) => {
        water_http::FunctionsMacroBuilderTow!( @parse [
                $($acc)*
                GET => $fn_name => $fn_name($context) async { $($body)* },
            ],
            $($rest)*
        );
    };
    // case fn name only without async
      // option 2
       (
        @parse [ $($acc:tt)* ],
         $fn_name:ident ( $context:ident )[$fn_path:path]  $($rest:tt)*
    ) => {
        water_http::FunctionsMacroBuilderTow!( @parse [
                $($acc)*
                GET => $fn_name => $fn_name($context) async {
                    $fn_path($context).await;
                },
            ],
            $($rest)*
        );
    };
     // case 5 with fn name only
    // option 1
       (
        @parse [ $($acc:tt)* ],
         $fn_name:ident ( $context:ident ) $async:tt { $($body:tt)* }  $($rest:tt)*
    ) => {
        water_http::FunctionsMacroBuilderTow!( @parse [
                $($acc)*
                GET => $fn_name => $fn_name($context) $async { $($body)* },
            ],
            $($rest)*
        );
    };
     // case 5 with fn name only
    // option 2
       (
        @parse [ $($acc:tt)* ],
         $fn_name:ident ( $context:ident ) $async:tt [$fn_path:path]  $($rest:tt)*
    ) => {
        water_http::FunctionsMacroBuilderTow!( @parse [
                $($acc)*
                GET => $fn_name => $fn_name($context) $async { $fn_path($context).await; },
            ],
            $($rest)*
        );
    };

    // case 6
       (
        @parse [ $($acc:tt)* ],
         $method:ident $fn_name:ident ( $context:ident ) $async:tt { $($body:tt)* }  $($rest:tt)*
    ) => {
        water_http::FunctionsMacroBuilderTow!( @parse [
                $($acc)*
                $method => / => $fn_name($context) $async { $($body)* },
            ],
            $($rest)*
        );
    };
    // case 6
       (
        @parse [ $($acc:tt)* ],
         $method:ident $fn_name:ident ( $context:ident ) $async:tt [$fn_path:path] $($rest:tt)*
    ) => {
        water_http::FunctionsMacroBuilderTow!( @parse [
                $($acc)*
                $method => / => $fn_name($context) $async { $fn_path($context).await; },
            ],
            $($rest)*
        );
    };

    // case 7
    //option 1
       (
        @parse [ $($acc:tt)* ],
         $method:ident $fn_name:ident ( $context:ident )  { $($body:tt)* }  $($rest:tt)*
    ) => {
        water_http::FunctionsMacroBuilderTow!( @parse [
                $($acc)*
                $method => / => $fn_name($context) async { $($body)* },
            ],
            $($rest)*
        );
    };

    // case 7
    // option 2
       (
        @parse [ $($acc:tt)* ],
         $method:ident $fn_name:ident ( $context:ident )  [$fn_path:path] $($rest:tt)*
    ) => {
        water_http::FunctionsMacroBuilderTow!( @parse [
                $($acc)*
                $method => / => $fn_name($context) async { $fn_path($context).await },
            ],
            $($rest)*
        );
    };

     // option 1
      (
        @parse [ $($acc:tt)* ],
         $($path:tt)/+ $fn_name:ident ( $context:ident )  { $($body:tt)* }  $($rest:tt)*
    ) => {
        water_http::FunctionsMacroBuilderTow!( @parse [
                $($acc)*
                GET => $($path)/+ => $fn_name($context) async { $($body)* },
            ],
            $($rest)*
        );
    };

        // option 2
      (
        @parse [ $($acc:tt)* ],
         $($path:tt)/+ $fn_name:ident ( $context:ident )  [$fn_path:path]  $($rest:tt)*
    ) => {
        water_http::FunctionsMacroBuilderTow!( @parse [
                $($acc)*
                GET => $($path)/+ => $fn_name($context) async { $fn_path($context).await },
            ],
            $($rest)*
        );
    };
   // option 1
     (
        @parse [ $($acc:tt)* ],
         $($path:tt)/+ $fn_name:ident ( $context:ident )  $async:tt { $($body:tt)* }  $($rest:tt)*
    ) => {
        water_http::FunctionsMacroBuilderTow!( @parse [
                $($acc)*
                GET => $($path)/+ => $fn_name($context) $async { $($body)* },
            ],
            $($rest)*
        );
    };

    // option 2

     (
        @parse [ $($acc:tt)* ],
         $($path:tt)/+ $fn_name:ident ( $context:ident )  $async:tt [$fn_path:path] $($rest:tt)*
    ) => {
        water_http::FunctionsMacroBuilderTow!( @parse [
                $($acc)*
                GET => $($path)/+ => $fn_name($context) $async { $fn_path($context).await },
            ],
            $($rest)*
        );
    };

    // option 1
     (
        @parse [ $($acc:tt)* ],
         $path:literal $fn_name:ident ( $context:ident )  { $($body:tt)* }  $($rest:tt)*
    ) => {
        water_http::FunctionsMacroBuilderTow!( @parse [
                $($acc)*
                GET => $path => $fn_name($context) async { $($body)* },
            ],
            $($rest)*
        );
    };

    // option 2
     (
        @parse [ $($acc:tt)* ],
         $path:literal $fn_name:ident ( $context:ident )  [$fn_path:path]  $($rest:tt)*
    ) => {
        water_http::FunctionsMacroBuilderTow!( @parse [
                $($acc)*
                GET => $path => $fn_name($context) async { $fn_path($context).await },
            ],
            $($rest)*
        );
    };

    // option 1
     (
        @parse [ $($acc:tt)* ],
         $path:literal $fn_name:ident ( $context:ident ) $async:tt { $($body:tt)* }  $($rest:tt)*
    ) => {
        water_http::FunctionsMacroBuilderTow!( @parse [
                $($acc)*
                GET => $path => $fn_name($context) $async { $($body)* },
            ],
            $($rest)*
        );
    };

      // option 2
     (
        @parse [ $($acc:tt)* ],
         $path:literal $fn_name:ident ( $context:ident ) $async:tt [$fn_path:path]   $($rest:tt)*
    ) => {
        water_http::FunctionsMacroBuilderTow!( @parse [
                $($acc)*
                GET => $path => $fn_name($context) $async { $fn_path($context).await },
            ],
            $($rest)*
        );
    };
    // case 8

   // option 1
    (
        @parse [ $($acc:tt)* ],
         $method:ident $($path:tt)/+ $fn_name:ident ( $context:ident ) $async:tt { $($body:tt)* }  $($rest:tt)*
    ) => {
        water_http::FunctionsMacroBuilderTow!( @parse [
                $($acc)*
                $method => $($path)/+ => $fn_name($context) $async { $($body)* },
            ],
            $($rest)*
        );
    };

    // option 2
    (
        @parse [ $($acc:tt)* ],
         $method:ident $($path:tt)/+ $fn_name:ident ( $context:ident ) $async:tt [$fn_path:path]  $($rest:tt)*
    ) => {
        water_http::FunctionsMacroBuilderTow!( @parse [
                $($acc)*
                $method => $($path)/+ => $fn_name($context) $async { $fn_path($context).await },
            ],
            $($rest)*
        );
    };

    // case 9
    // option 1
       (
        @parse [ $($acc:tt)* ],
         $method:ident $($path:tt)/+ $fn_name:ident ( $context:ident )  { $($body:tt)* }  $($rest:tt)*
    ) => {
        water_http::FunctionsMacroBuilderTow!( @parse [
                $($acc)*
                $method => $($path)/+ => $fn_name($context) async { $($body)* },
            ],
            $($rest)*
        );
    };

      // case 9
      // option 2
       (
        @parse [ $($acc:tt)* ],
         $method:ident $($path:tt)/+ $fn_name:ident ( $context:ident )  [$fn_path:path]  $($rest:tt)*
    ) => {
        water_http::FunctionsMacroBuilderTow!( @parse [
                $($acc)*
                $method => $($path)/+ => $fn_name($context) async { $fn_path($context).await },
            ],
            $($rest)*
        );
    };

    // option 1
      (
        @parse [ $($acc:tt)* ],
         $method:ident $path:tt $fn_name:ident ( $context:ident ) $async:tt { $($body:tt)* }  $($rest:tt)*
    ) => {
        water_http::FunctionsMacroBuilderTow!( @parse [
                $($acc)*
                $method => $path => $fn_name($context) $async { $($body)* },
            ],
            $($rest)*
        );
    };

      // option 2
      (
        @parse [ $($acc:tt)* ],
         $method:ident $path:tt $fn_name:ident ( $context:ident ) $async:tt [$fn_path:path] $($rest:tt)*
    ) => {
        water_http::FunctionsMacroBuilderTow!( @parse [
                $($acc)*
                $method => $path => $fn_name($context) $async { $fn_path($context).await },
            ],
            $($rest)*
        );
    };



    // option 1
       (
        @parse [ $($acc:tt)* ],
         $method:ident $path:tt $fn_name:ident ( $context:ident )  { $($body:tt)* }  $($rest:tt)*
    ) => {
        water_http::FunctionsMacroBuilderTow!( @parse [
                $($acc)*
                $method => $path => $fn_name($context) async { $($body)* },
            ],
            $($rest)*
        );
    };

    // option 2
       (
        @parse [ $($acc:tt)* ],
         $method:ident $path:tt $fn_name:ident ( $context:ident )  [$fn_path:path]  $($rest:tt)*
    ) => {
        water_http::FunctionsMacroBuilderTow!( @parse [
                $($acc)*
                $method => $path => $fn_name($context) async { $fn_path($context).await },
            ],
            $($rest)*
        );
    };


    // case 10
    // option 1
       (
        @parse [ $($acc:tt)* ],
        #[$method:ident,$($path:tt)/+]
        $async:tt $fn_name:ident($context:ident) { $($body:tt)* }  $($rest:tt)*
    ) => {
        water_http::FunctionsMacroBuilderTow!( @parse [
                $($acc)*
                $method => $($path)/+ => $fn_name($context) $async { $($body)* },
            ],
            $($rest)*
        );
    };
      // case 10
    // option 1
       (
        @parse [ $($acc:tt)* ],
        #[$method:ident,$($path:tt)/+]
        $fn_name:ident($context:ident) $async:tt { $($body:tt)* }  $($rest:tt)*
    ) => {
        water_http::FunctionsMacroBuilderTow!( @parse [
                $($acc)*
                $method => $($path)/+ => $fn_name($context) $async { $($body)* },
            ],
            $($rest)*
        );
    };

    // case 10
    // option 2
       (
        @parse [ $($acc:tt)* ],
        #[$method:ident,$($path:tt)/+]
        $async:tt $fn_name:ident($context:ident) [$fn_path:path]  $($rest:tt)*
    ) => {
        water_http::FunctionsMacroBuilderTow!( @parse [
                $($acc)*
                $method => $($path)/+ => $fn_name($context) $async {
                    $fn_path($context).await
                },
            ],
            $($rest)*
        );
    };

    // case 10
    // option 2
       (
        @parse [ $($acc:tt)* ],
        #[$method:ident,$($path:tt)/+]
        $fn_name:ident($context:ident)$async:tt  [$fn_path:path]  $($rest:tt)*
    ) => {
        water_http::FunctionsMacroBuilderTow!( @parse [
                $($acc)*
                $method => $($path)/+ => $fn_name($context) $async {
                    $fn_path($context).await
                },
            ],
            $($rest)*
        );
    };



    // case 11
    // option 2
       (
        @parse [ $($acc:tt)* ],
        #[$method:ident,$($path:tt)/+]
        $fn_name:ident($context:ident) { $($body:tt)* }  $($rest:tt)*
    ) => {
        water_http::FunctionsMacroBuilderTow!( @parse [
                $($acc)*
                $method => $($path)/+ => $fn_name($context) async { $($body)* },
            ],
            $($rest)*
        );
    };


    // case 11
    // option 2
       (
        @parse [ $($acc:tt)* ],
        #[$method:ident,$($path:tt)/+]
        $fn_name:ident($context:ident) [$fn_path:path]  $($rest:tt)*
    ) => {
        water_http::FunctionsMacroBuilderTow!( @parse [
                $($acc)*
                $method => $($path)/+ => $fn_name($context) async {
                    $fn_path($context).await
                },
            ],
            $($rest)*
        );
    };



    // case 12
    // option 1
       (
        @parse [ $($acc:tt)* ],
        #[$method:ident,$($path:tt)/+]
        $async:tt fn $fn_name:ident($context:ident) { $($body:tt)* }  $($rest:tt)*
    ) => {
        water_http::FunctionsMacroBuilderTow!( @parse [
                $($acc)*
                $method => $($path)/+ => $fn_name($context) $async { $($body)* },
            ],
            $($rest)*
        );
    };


    // case 12
    // option 2
       (
        @parse [ $($acc:tt)* ],
        #[$method:ident,$($path:tt)/+]
        $async:tt fn $fn_name:ident($context:ident) [$fn_path:path]  $($rest:tt)*
    ) => {
        water_http::FunctionsMacroBuilderTow!( @parse [
                $($acc)*
                $method => $($path)/+ => $fn_name($context) $async {
                    $fn_path($context).await
                },
            ],
            $($rest)*
        );
    };


    // case 13
    // option 1
       (
        @parse [ $($acc:tt)* ],
        #[$method:ident,$($path:tt)/+]
        fn $fn_name:ident($context:ident) { $($body:tt)* }  $($rest:tt)*
    ) => {
        water_http::FunctionsMacroBuilderTow!( @parse [
                $($acc)*
                $method => $($path)/+ => $fn_name($context) async { $($body)* },
            ],
            $($rest)*
        );
    };


    // case 13
    // option 2
       (
        @parse [ $($acc:tt)* ],
        #[$method:ident,$($path:tt)/+]
        fn $fn_name:ident($context:ident) [$fn_path:path]  $($rest:tt)*
    ) => {
        water_http::FunctionsMacroBuilderTow!( @parse [
                $($acc)*
                $method => $($path)/+ => $fn_name($context) async {
                    $fn_path($context).await
                },
            ],
            $($rest)*
        );
    };

    // ----- Terminal: no more tokens -----
    (
        @parse [ $($acc:tt)* ],
    ) => {
      // fn yess{ let a = stringify!($($acc)*);}

        FunctionsMacroBuilder!(
          functions->{
              $($acc)*
          }
      );
    };
}


/// constructing functions builder
#[doc(hidden)]
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
          },
         )*
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
                     stringify!($method).replace('"',"").replace(" ",""),
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
#[doc(hidden)]
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
#[doc(hidden)]
#[macro_export]
macro_rules! CheckExtraCode {

    (extra_code ->  .. {$($b:tt)*} ) => {
        $($b)*
    };
   ($key:tt -> $($tokens:tt)* ) => {
    };
}

/// for creating single water controller or capsule for encapsulating objects handlers and routes
/// creating Water Controller is very easy
/// default creating
/// ```rust
/// use water_http::WaterController;
/// WaterController! {
///  holder -> u8,
///  name -> WebMainController,
///  functions -> {
///   GET => / => main(context){
///    let mut sender = context.sender();
///    if let Ok(_) = sender.send_str("hello from server").await {
///
///   }
///  }
/// }
/// }
/// ```
#[macro_export]
macro_rules! WaterController {
    {
     holder -> $holder:path,
     name -> $name:ident,
     functions -> {$($function_tokens:tt)*}

     $($key:tt -> ($($value:tt)*)),*
    } => {
        #[allow(non_snake_case)]
        pub mod $name {

            use water_http::http::{HttpSenderTrait,request::{HttpGetterTrait,IBodyChunks,IBody,ParsingBodyResults,ParsingBodyMechanism}};
            use water_http::server::HttpContext;
            use water_http::*;
            pub type Holder = $holder;

            water_http::FunctionsMacroBuilderTow!(
                @entry
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


    {
     holder -> $holder:path,
     name -> $name:ident,
     functions -> {$($function_tokens:tt)*} $separator:tt
     $($key:tt -> ($($value:tt)*)),*
    } => {
       water_http::WaterController!(
          holder -> $holder,
          name -> $name,
          functions -> { $($function_tokens)* }
           $($key -> ($($value)*)),*
       );
    };

}




/// for setting path from another macro
/// it`s for another macros call so do not worry about it ,
/// we just had to make it public for re calling it from another macros

#[doc(hidden)]
#[macro_export]
macro_rules! path_setter {
    [$context_name:ident () {$path_item:tt} ]=>{
        let $path_item = $context_name.get_from_path_params(stringify!($path_item)).unwrap();
    };
     [$context_name:ident () $path_item:tt]=>{
    };


    ( $context_name:ident -> $($p_item:tt)/+) => {
        $(
            water_http::path_setter![$context_name () $p_item ];
        )+
    };
}


#[doc(hidden)]
#[macro_export]
macro_rules! response {
    ($context:ident -> $res:expr) => {
        _= $context.send_str($res).await;
    };
    ($context:ident json -> $res:expr) => {
        let mut sender = $context.sender();
            _= sender.send_json($res).await;
    };

    ($context:ident file -> $res:expr) => {
        let mut sender = $context.sender();
            let sending_result= sender.send_file(water_http::http::FileRSender::new($res)).await;
           if !sending_result.is_success() {
            _= $context.send_status_code(water_http::http::status_code::HttpStatusCode::NOT_FOUND);
        }
    };
    ($context:ident file -> $res:expr , $($function_tokens:tt)*) => {
        let mut __sender = $context.sender();
        let mut __f = water_http::http::FileRSender::new($res);
        __f.set_edit_each_chunk( $($function_tokens)*);
            let sending_result= __sender.send_file(__f).await;
           if !sending_result.is_success() {
            _= __sender.send_status_code(water_http::http::status_code::HttpStatusCode::INTERNAL_SERVER_ERROR);
        }
    };

     ($context:ident string -> $res:ident) => {
            _= $context.send_string_slice(&$res).await;
    };
     ($context:ident string -> &$res:ident) => {
            _= $context.send_string_slice(&$res).await;
    };
     ($context:ident string -> $value:expr ) => {
            _= $context.send_string_slice(format!($value).as_str()).await;
    };
}


#[macro_export]
macro_rules! functions_builder {
    {
        $($tokens:tt)*
    } => {
        $crate::hidden_functions_builder!(@parsed[], $($tokens)*);
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! hidden_functions_builder {
    (
        @parsed[$($d:tt)*],
        $fn_name:ident($context:ident $(, $parameter_name:ident : $typ:ty)* ) $async:tt
        ($($param:ident),*) {

            $($body_tokens:tt)*
        }
        $($res:tt)*
    ) => {
        $crate::hidden_functions_builder! (
            @parsed[
                $($d)*
            ],
            pub fn $fn_name($context$(,$parameter_name:$typ)*) $async {
                $(let $param = $context.get_from_path_params(stringify!($param)).unwrap();)*
                    $($body_tokens)*
            }
            $($res)*
        );
    };


     (
        @parsed[$($d:tt)*],
        $pub:tt $fn:tt $fn_name:ident($context:ident $(, $parameter_name:ident : $typ:ty)* ) $async:tt
        ($($param:ident),*) {

            $($body_tokens:tt)*
        }
        $($res:tt)*
    ) => {
        $crate::hidden_functions_builder! (
            @parsed[
                $($d)*
            ],
            $pub $fn $fn_name($context$(,$parameter_name:$typ)*) $async {
                $(let $param = $context.get_from_path_params(stringify!($param)).unwrap();)*
                    $($body_tokens)*
            }
            $($res)*
        );
    };

       (
        @parsed[$($d:tt)*],
        pub $fn:tt $fn_name:ident($context:ident $(, $parameter_name:ident : $typ:ty)* )
        ($($param:ident),*) {

            $($body_tokens:tt)*
        }
        $($res:tt)*
    ) => {
        $crate::hidden_functions_builder! (
            @parsed[
                $($d)*
            ],
            pub $fn $fn_name($context$(,$parameter_name:$typ)*) async {
                $(let $param = $context.get_from_path_params(stringify!($param)).unwrap();)*
                    $($body_tokens)*
            }
            $($res)*
        );
    };


     (
        @parsed[$($d:tt)*],
        $pub:tt $async:tt $fn:tt $fn_name:ident($context:ident $(, $parameter_name:ident : $typ:ty)* )
        ($($param:ident),*) {

            $($body_tokens:tt)*
        }
        $($res:tt)*
    ) => {
        $crate::hidden_functions_builder! (
            @parsed[
                $($d)*
            ],
            $pub $fn $fn_name($context$(,$parameter_name:$typ)*) $async {
                $(let $param = $context.get_from_path_params(stringify!($param)).unwrap();)*
                    $($body_tokens)*
            }
            $($res)*
        );
    };

     (
        @parsed[$($d:tt)*],
        $pub:tt $async:tt $fn:tt $fn_name:ident($context:ident $(, $parameter_name:ident : $typ:ty)* )
        {

            $($body_tokens:tt)*
        }
        $($res:tt)*
    ) => {
        $crate::hidden_functions_builder! (
            @parsed[
                $($d)*
            ],
            $pub $fn $fn_name($context$(,$parameter_name:$typ)*) $async {
                    $($body_tokens)*
            }
            $($res)*
        );
    };

      (
        @parsed[$($d:tt)*],
         $async:tt $fn:tt $fn_name:ident($context:ident $(, $parameter_name:ident : $typ:ty)* )
        ($($param:ident),*) {

            $($body_tokens:tt)*
        }
        $($res:tt)*
    ) => {
        $crate::hidden_functions_builder! (
            @parsed[
                $($d)*
            ],
            pub $fn $fn_name($context$(,$parameter_name:$typ)*) $async {
                $(let $param = $context.get_from_path_params(stringify!($param)).unwrap();)*
                    $($body_tokens)*
            }
            $($res)*
        );
    };

        (
        @parsed[$($d:tt)*],
         $async:tt $fn:tt $fn_name:ident($context:ident $(, $parameter_name:ident : $typ:ty)* )
        {

            $($body_tokens:tt)*
        }
        $($res:tt)*
    ) => {
        $crate::hidden_functions_builder! (
            @parsed[
                $($d)*
            ],
            pub $fn $fn_name($context$(,$parameter_name:$typ)*) $async {
                    $($body_tokens)*
            }
            $($res)*
        );
    };

          (
        @parsed[$($d:tt)*],
        $fn:tt $fn_name:ident($context:ident $(, $parameter_name:ident : $typ:ty)* )
        ($($param:ident),*) {

            $($body_tokens:tt)*
        }
        $($res:tt)*
    ) => {
        $crate::hidden_functions_builder! (
            @parsed[
                $($d)*
            ],
            pub $fn $fn_name($context$(,$parameter_name:$typ)*) async {
                $(let $param = $context.get_from_path_params(stringify!($param)).unwrap();)*
                    $($body_tokens)*
            }
            $($res)*
        );
    };

           (
        @parsed[$($d:tt)*],
        $fn:tt $fn_name:ident($context:ident $(, $parameter_name:ident : $typ:ty)* )
       {

            $($body_tokens:tt)*
        }
        $($res:tt)*
    ) => {
        $crate::hidden_functions_builder! (
            @parsed[
                $($d)*
            ],
            pub $fn $fn_name($context$(,$parameter_name:$typ)*) async {
                    $($body_tokens)*
            }
            $($res)*
        );
    };

    // for building results
      (
        @parsed[$($d:tt)*],
        $pub:tt $fn:tt $fn_name:ident($context:ident $(, $parameter_name:ident : $typ:ty)* ) $async:tt {
            $($body_tokens:tt)*
        }
        $($res:tt)*
    ) => {
        $crate::hidden_functions_builder! (
            @parsed[
                $($d)*
                $pub $async $fn $fn_name<
                    'context,
                    MainHolderType: Send + 'static,
                    const header_length: usize,
                    const query_length: usize
                >(
                    $context: &mut $crate::server::HttpContext<
                        'context, MainHolderType, header_length, query_length
                    >
                    $(, $parameter_name: $typ)*
                ) {
                    $($body_tokens)*
                }
            ],
            $($res)*
        );
    };


    (@parsed[$($d:tt)*],) => {
        $($d)*
    };
}
