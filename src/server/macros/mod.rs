#[cfg(not(feature = "thread_shared_struct"))]
#[macro_export]
macro_rules! start_server {
    {
        holder -> $t:ty,
        name -> $name:ident,
        ip -> $ip:expr,
        port -> $port:expr,
        $( $tokens:tt )*
    } => {
        use water_http::InitControllersRoot;
        type MainHolderType = $t;

        InitControllersRoot!{
            name: MAIN_ROOT,
            holder_type: crate::MainHolderType,
        }

        fn start_fast_server() {
            let config = water_http::server::ServerConfigurations::bind($ip, $port);
            water_http::RunServer!(
                config,
                MAIN_ROOT,
                $name
            );
        }

        water_http::WaterController!{
            holder -> crate::MainHolderType,
            name -> $name,
            $( $tokens )*
        }
    };
}

#[cfg(feature = "thread_shared_struct")]
#[macro_export]
macro_rules! start_server {
    {
        holder -> $t:ty,
        name -> $name:ident,
        ip -> $ip:expr,
        port -> $port:expr,
        shared_struct -> $async:tt $build_shared_struct_function:ident ()-> $shared_type:ty $bloc:block,
        $( $tokens:tt )*
    } => {
        use water_http::InitControllersRoot;
        type MainHolderType = $t;

        InitControllersRoot!{
            name: MAIN_ROOT,
            holder_type: crate::MainHolderType,
            shared_type: $shared_type,
        }

        pub fn $build_shared_struct_function ()->std::pin::Pin<std::boxed::Box<dyn std::future::Future<Output=$shared_type>>> {
            std::boxed::Box::pin($async $bloc)
        }

        fn start_fast_server() {
            let config = water_http::server::ServerConfigurations::bind($ip, $port);
            water_http::RunServer!(
                config,
                MAIN_ROOT,
                $name,
                $build_shared_struct_function
            );
        }

        // Sequences correctly: holder -> shared -> name -> functions
        water_http::WaterController!{
            holder -> crate::MainHolderType,
            shared -> $shared_type,
            name -> $name,
            $( $tokens )*
        }
    };
}

// =============================================================================
// FAST_BUILD: WITHOUT FEATURE "thread_shared_struct" (Your Existing Code)
// =============================================================================
#[cfg(not(feature = "thread_shared_struct"))]
#[macro_export]
macro_rules! fast_build {
    {
        holder -> $t:ty,
        name -> $name:ident,
        ip -> $ip:expr,
        port -> $port:expr,
        $( $tokens:tt )*
    } => {
        water_http::start_server! {
            holder -> $t,
            name -> $name,
            ip -> $ip,
            port -> $port,
            $( $tokens )*
        }
    };

    {
        holder -> $t:ty,
        name -> $name:ident,
        ip -> $ip:expr,
        $( $tokens:tt )*
    } => {
        water_http::start_server! {
            holder -> $t,
            name -> $name,
            ip -> $ip,
            port -> 8084,
            $( $tokens )*
        }
    };

    {
        holder -> $t:ty,
        name -> $name:ident,
        port -> $port:expr,
        $( $tokens:tt )*
    } => {
        water_http::start_server! {
            holder -> $t,
            name -> $name,
            ip -> "0.0.0.0",
            port -> $port,
            $( $tokens )*
        }
    };

    {
        holder -> $t:ty,
        ip -> $ip:expr,
        port -> $port:expr,
        $( $tokens:tt )*
    } => {
        water_http::start_server! {
            holder -> $t,
            name -> MainController,
            ip -> $ip,
            port -> $port,
            $( $tokens )*
        }
    };

    {
        name -> $name:ident,
        ip -> $ip:expr,
        port -> $port:expr,
        $( $tokens:tt )*
    } => {
        water_http::start_server! {
            holder -> u8,
            name -> $name,
            ip -> $ip,
            port -> $port,
            $( $tokens )*
        }
    };

    {
        holder -> $t:ty,
        name -> $name:ident,
        $( $tokens:tt )*
    } => {
        water_http::start_server! {
            holder -> $t,
            name -> $name,
            ip -> "0.0.0.0",
            port -> 8084,
            $( $tokens )*
        }
    };

    {
        holder -> $t:ty,
        ip -> $ip:expr,
        $( $tokens:tt )*
    } => {
        water_http::start_server! {
            holder -> $t,
            name -> MainController,
            ip -> $ip,
            port -> 8084,
            $( $tokens )*
        }
    };

    {
        holder -> $t:ty,
        port -> $port:expr,
        $( $tokens:tt )*
    } => {
        water_http::start_server! {
            holder -> $t,
            name -> MainController,
            ip -> "0.0.0.0",
            port -> $port,
            $( $tokens )*
        }
    };

    {
        name -> $name:ident,
        ip -> $ip:expr,
        $( $tokens:tt )*
    } => {
        water_http::start_server! {
            holder -> u8,
            name -> $name,
            ip -> $ip,
            port -> 8084,
            $( $tokens )*
        }
    };

    {
        name -> $name:ident,
        port -> $port:expr,
        $( $tokens:tt )*
    } => {
        water_http::start_server! {
            holder -> u8,
            name -> $name,
            ip -> "0.0.0.0",
            port -> $port,
            $( $tokens )*
        }
    };

    {
        ip -> $ip:expr,
        port -> $port:expr,
        $( $tokens:tt )*
    } => {
        water_http::start_server! {
            holder -> u8,
            name -> MainController,
            ip -> $ip,
            port -> $port,
            $( $tokens )*
        }
    };

    {
        holder -> $t:ty,
        $( $tokens:tt )*
    } => {
        water_http::start_server! {
            holder -> $t,
            name -> MainController,
            ip -> "0.0.0.0",
            port -> 8084,
            $( $tokens )*
        }
    };

    {
        name -> $name:ident,
        $( $tokens:tt )*
    } => {
        water_http::start_server! {
            holder -> u8,
            name -> $name,
            ip -> "0.0.0.0",
            port -> 8084,
            $( $tokens )*
        }
    };

    {
        ip -> $ip:expr,
        $( $tokens:tt )*
    } => {
        water_http::start_server! {
            holder -> u8,
            name -> MainController,
            ip -> $ip,
            port -> 8084,
            $( $tokens )*
        }
    };

    {
        port -> $port:expr,
        $( $tokens:tt )*
    } => {
        water_http::start_server! {
            holder -> u8,
            name -> MainController,
            ip -> "0.0.0.0",
            port -> $port,
            $( $tokens )*
        }
    };

    {
        $( $tokens:tt )*
    } => {
        water_http::start_server! {
            holder -> u8,
            name -> MainController,
            ip -> "0.0.0.0",
            port -> 8084,
            $( $tokens )*
        }
    };
}

// =============================================================================
// FAST_BUILD: WITH FEATURE "thread_shared_struct" (All Possibilities Handled)
// =============================================================================
#[cfg(feature = "thread_shared_struct")]
#[macro_export]
macro_rules! fast_build {
    // =========================================================================
    // 4 PROPERTIES + SHARED_STRUCT
    // =========================================================================
    {
        holder -> $t:ty,
        name -> $name:ident,
        ip -> $ip:expr,
        port -> $port:expr,
        shared_struct -> $async:tt $build_fn:ident ()-> $shared_type:ty $bloc:block,
        $( $tokens:tt )*
    } => {
        water_http::start_server! {
            holder -> $t,
            name -> $name,
            ip -> $ip,
            port -> $port,
            shared_struct -> $async $build_fn ()-> $shared_type $bloc,
            $( $tokens )*
        }
    };

    // =========================================================================
    // 3 PROPERTIES + SHARED_STRUCT
    // =========================================================================
    {
        holder -> $t:ty,
        name -> $name:ident,
        ip -> $ip:expr,
        shared_struct -> $async:tt $build_fn:ident ()-> $shared_type:ty $bloc:block,
        $( $tokens:tt )*
    } => {
        water_http::start_server! {
            holder -> $t,
            name -> $name,
            ip -> $ip,
            port -> 8084,
            shared_struct -> $async $build_fn ()-> $shared_type $bloc,
            $( $tokens )*
        }
    };

    {
        holder -> $t:ty,
        name -> $name:ident,
        port -> $port:expr,
        shared_struct -> $async:tt $build_fn:ident ()-> $shared_type:ty $bloc:block,
        $( $tokens:tt )*
    } => {
        water_http::start_server! {
            holder -> $t,
            name -> $name,
            ip -> "0.0.0.0",
            port -> $port,
            shared_struct -> $async $build_fn ()-> $shared_type $bloc,
            $( $tokens )*
        }
    };

    {
        holder -> $t:ty,
        ip -> $ip:expr,
        port -> $port:expr,
        shared_struct -> $async:tt $build_fn:ident ()-> $shared_type:ty $bloc:block,
        $( $tokens:tt )*
    } => {
        water_http::start_server! {
            holder -> $t,
            name -> MainController,
            ip -> $ip,
            port -> $port,
            shared_struct -> $async $build_fn ()-> $shared_type $bloc,
            $( $tokens )*
        }
    };

    {
        name -> $name:ident,
        ip -> $ip:expr,
        port -> $port:expr,
        shared_struct -> $async:tt $build_fn:ident ()-> $shared_type:ty $bloc:block,
        $( $tokens:tt )*
    } => {
        water_http::start_server! {
            holder -> u8,
            name -> $name,
            ip -> $ip,
            port -> $port,
            shared_struct -> $async $build_fn ()-> $shared_type $bloc,
            $( $tokens )*
        }
    };

    // =========================================================================
    // 2 PROPERTIES + SHARED_STRUCT
    // =========================================================================
    {
        holder -> $t:ty,
        name -> $name:ident,
        shared_struct -> $async:tt $build_fn:ident ()-> $shared_type:ty $bloc:block,
        $( $tokens:tt )*
    } => {
        water_http::start_server! {
            holder -> $t,
            name -> $name,
            ip -> "0.0.0.0",
            port -> 8084,
            shared_struct -> $async $build_fn ()-> $shared_type $bloc,
            $( $tokens )*
        }
    };

    {
        holder -> $t:ty,
        ip -> $ip:expr,
        shared_struct -> $async:tt $build_fn:ident ()-> $shared_type:ty $bloc:block,
        $( $tokens:tt )*
    } => {
        water_http::start_server! {
            holder -> $t,
            name -> MainController,
            ip -> $ip,
            port -> 8084,
            shared_struct -> $async $build_fn ()-> $shared_type $bloc,
            $( $tokens )*
        }
    };

    {
        holder -> $t:ty,
        port -> $port:expr,
        shared_struct -> $async:tt $build_fn:ident ()-> $shared_type:ty $bloc:block,
        $( $tokens:tt )*
    } => {
        water_http::start_server! {
            holder -> $t,
            name -> MainController,
            ip -> "0.0.0.0",
            port -> $port,
            shared_struct -> $async $build_fn ()-> $shared_type $bloc,
            $( $tokens )*
        }
    };

    {
        name -> $name:ident,
        ip -> $ip:expr,
        shared_struct -> $async:tt $build_fn:ident ()-> $shared_type:ty $bloc:block,
        $( $tokens:tt )*
    } => {
        water_http::start_server! {
            holder -> u8,
            name -> $name,
            ip -> $ip,
            port -> 8084,
            shared_struct -> $async $build_fn ()-> $shared_type $bloc,
            $( $tokens )*
        }
    };

    {
        name -> $name:ident,
        port -> $port:expr,
        shared_struct -> $async:tt $build_fn:ident ()-> $shared_type:ty $bloc:block,
        $( $tokens:tt )*
    } => {
        water_http::start_server! {
            holder -> u8,
            name -> $name,
            ip -> "0.0.0.0",
            port -> $port,
            shared_struct -> $async $build_fn ()-> $shared_type $bloc,
            $( $tokens )*
        }
    };

    {
        ip -> $ip:expr,
        port -> $port:expr,
        shared_struct -> $async:tt $build_fn:ident ()-> $shared_type:ty $bloc:block,
        $( $tokens:tt )*
    } => {
        water_http::start_server! {
            holder -> u8,
            name -> MainController,
            ip -> $ip,
            port -> $port,
            shared_struct -> $async $build_fn ()-> $shared_type $bloc,
            $( $tokens )*
        }
    };

    // =========================================================================
    // 1 PROPERTY + SHARED_STRUCT (This matches your code!)
    // =========================================================================
    {
        holder -> $t:ty,
        shared_struct -> $async:tt $build_fn:ident ()-> $shared_type:ty $bloc:block,
        $( $tokens:tt )*
    } => {
        water_http::start_server! {
            holder -> $t,
            name -> MainController,
            ip -> "0.0.0.0",
            port -> 8084,
            shared_struct -> $async $build_fn ()-> $shared_type $bloc,
            $( $tokens )*
        }
    };

    {
        name -> $name:ident,
        shared_struct -> $async:tt $build_fn:ident ()-> $shared_type:ty $bloc:block,
        $( $tokens:tt )*
    } => {
        water_http::start_server! {
            holder -> u8,
            name -> $name,
            ip -> "0.0.0.0",
            port -> 8084,
            shared_struct -> $async $build_fn ()-> $shared_type $bloc,
            $( $tokens )*
        }
    };

    {
        ip -> $ip:expr,
        shared_struct -> $async:tt $build_fn:ident ()-> $shared_type:ty $bloc:block,
        $( $tokens:tt )*
    } => {
        water_http::start_server! {
            holder -> u8,
            name -> MainController,
            ip -> $ip,
            port -> 8084,
            shared_struct -> $async $build_fn ()-> $shared_type $bloc,
            $( $tokens )*
        }
    };

    {
        port -> $port:expr,
        shared_struct -> $async:tt $build_fn:ident ()-> $shared_type:ty $bloc:block,
        $( $tokens:tt )*
    } => {
        water_http::start_server! {
            holder -> u8,
            name -> MainController,
            ip -> "0.0.0.0",
            port -> $port,
            shared_struct -> $async $build_fn ()-> $shared_type $bloc,
            $( $tokens )*
        }
    };

    // =========================================================================
    // NO PROPERTIES AT ALL (JUST THE SHARED_STRUCT) OR FALLBACK
    // =========================================================================
    {
        shared_struct -> $async:tt $build_fn:ident ()-> $shared_type:ty $bloc:block,
        $( $tokens:tt )*
    } => {
        water_http::start_server! {
            holder -> u8,
            name -> MainController,
            ip -> "0.0.0.0",
            port -> 8084,
            shared_struct -> $async $build_fn ()-> $shared_type $bloc,
            $( $tokens )*
        }
    };

    {
        $( $tokens:tt )*
    } => {
        water_http::start_server! {
            holder -> u8,
            name -> MainController,
            ip -> "0.0.0.0",
            port -> 8084,
            $( $tokens )*
        }
    };
}


#[macro_export]
macro_rules! extract_request_body {
    {
        $context:ident [
            $($body:ident -> $val:expr),*
      ] else $block:block
    } => {
        use water_http::http::request::DynamicBodyMapTrait;
        let body = match $context.get_body_map().await {
            Ok(body) => body,
            _ => {
                $block
                return
            }
        };
        $(
         let $body = body.get($val);
        )*

    };
    {
        $context:ident [
            $($body:ident ),*
      ] else $block:block
    } => {
        use water_http::http::request::DynamicBodyMapTrait;
        let body = match $context.get_body_map().await {
            Ok(body) => body,
            _ => {
                $block
                return
            }
        };
        $(
         let $body = body.get(stringify!($body));
        )*
    };

}