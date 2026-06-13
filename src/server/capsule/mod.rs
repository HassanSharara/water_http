/// defining all the used and imported macros for building your app struct
pub mod capsule_macros;
/// providing matching tech
pub mod matcher;
mod response_interceptors;

use std::future::Future;
use std::pin::Pin;
use crate::server::{HttpContext, push_named_route};
use smallbox::SmallBox;
use smallbox::space::S32 as SmallBoxSize;
#[cfg(feature = "lazy_response")]
use crate::server::capsule::response_interceptors::InterceptorCallback;

/// ----------------------------
/// SmallBox Future aliases (Zero-allocation for small handlers)
/// ----------------------------
#[cfg(feature = "use_tokio_send")]
pub(crate) type BoxFutureSend<'a, T> = Pin<SmallBox<dyn Future<Output = T> + Send + 'a, SmallBoxSize>>;

#[cfg(not(feature = "use_tokio_send"))]
pub(crate) type BoxFuture<'a, T> = Pin<SmallBox<dyn Future<Output = T> + 'a, SmallBoxSize>>;

// /// ----------------------------
// /// Future aliases (Send vs non-Send)
// /// ----------------------------
// #[cfg(feature = "use_tokio_send")]
// type BoxFutureSend<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
//
// #[cfg(not(feature = "use_tokio_send"))]
// type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

/// ----------------------------
/// Middleware and handler type aliases
/// (two variants depending on thread_shared_struct)
/// ----------------------------

#[cfg(all(feature = "thread_shared_struct", feature = "use_tokio_send"))]
pub(crate) type MiddlewareCallback<
    H,
    SHARED,
    const HEADER_SIZE: usize,
    const QUERY_SIZE: usize,
> = for<'a, 'context> fn(
    &'a mut HttpContext<'context, H, SHARED, HEADER_SIZE, QUERY_SIZE>,
) -> BoxFutureSend<'a, MiddlewareResult>;

#[cfg(all(feature = "thread_shared_struct", not(feature = "use_tokio_send")))]
pub(crate) type MiddlewareCallback<
    H,
    SHARED,
    const HEADER_SIZE: usize,
    const QUERY_SIZE: usize,
> = for<'a, 'context> fn(
    &'a mut HttpContext<'context, H, SHARED, HEADER_SIZE, QUERY_SIZE>,
) -> BoxFuture<'a, MiddlewareResult>;

#[cfg(all(not(feature = "thread_shared_struct"), feature = "use_tokio_send"))]
pub(crate) type MiddlewareCallback<
    H,
    const HEADER_SIZE: usize,
    const QUERY_SIZE: usize,
> = for<'a, 'context> fn(
    &'a mut HttpContext<'context, H, HEADER_SIZE, QUERY_SIZE>,
) -> BoxFutureSend<'a, MiddlewareResult>;

#[cfg(all(not(feature = "thread_shared_struct"), not(feature = "use_tokio_send")))]
pub(crate) type MiddlewareCallback<
    H,
    const HEADER_SIZE: usize,
    const QUERY_SIZE: usize,
> = for<'a, 'context> fn(
    &'a mut HttpContext<'context, H, HEADER_SIZE, QUERY_SIZE>,
) -> BoxFuture<'a, MiddlewareResult>;

/// WaterSingleFunction: handler that returns ()
#[cfg(all(feature = "thread_shared_struct", feature = "use_tokio_send"))]
type WaterSingleFunction<
    H,
    SHARED,
    const HEADER_SIZE: usize,
    const QUERY_SIZE: usize,
> = for<'a, 'context> fn(
    &'a mut HttpContext<'context, H, SHARED, HEADER_SIZE, QUERY_SIZE>,
) -> BoxFutureSend<'a, ()>;

#[cfg(all(feature = "thread_shared_struct", not(feature = "use_tokio_send")))]
type WaterSingleFunction<
    H,
    SHARED,
    const HEADER_SIZE: usize,
    const QUERY_SIZE: usize,
> = for<'a, 'context> fn(
    &'a mut HttpContext<'context, H, SHARED, HEADER_SIZE, QUERY_SIZE>,
) -> BoxFuture<'a, ()>;

#[cfg(all(not(feature = "thread_shared_struct"), feature = "use_tokio_send"))]
type WaterSingleFunction<
    H,
    const HEADER_SIZE: usize,
    const QUERY_SIZE: usize,
> = for<'a, 'context> fn(
    &'a mut HttpContext<'context, H, HEADER_SIZE, QUERY_SIZE>,
) -> BoxFutureSend<'a, ()>;

#[cfg(all(not(feature = "thread_shared_struct"), not(feature = "use_tokio_send")))]
type WaterSingleFunction<
    H,
    const HEADER_SIZE: usize,
    const QUERY_SIZE: usize,
> = for<'a, 'context> fn(
    &'a mut HttpContext<'context, H, HEADER_SIZE, QUERY_SIZE>,
) -> BoxFuture<'a, ()>;

/// ----------------------------
/// CapsuleWaterController struct variants
/// ----------------------------

#[cfg(feature = "thread_shared_struct")]
#[derive(Debug)]
pub struct CapsuleWaterController<
    #[cfg(feature = "use_tokio_send")]
    H: Send + 'static,
    #[cfg(not(feature = "use_tokio_send"))]
    H:'static,

    #[cfg(not(feature = "use_tokio_send"))]
    SHARED:Clone + 'static,

    #[cfg(feature = "use_tokio_send")]
    SHARED:Clone + Send + 'static,
    const HEADER_SIZE: usize,
    const QUERY_SIZE: usize,
> {
    pub(crate) father: Option<*const CapsuleWaterController<H, SHARED, HEADER_SIZE, QUERY_SIZE>>,
    pub prefix: Option<&'static str>,
    pub middleware: Option<MiddlewareCallback<H, SHARED, HEADER_SIZE, QUERY_SIZE>>,
    #[cfg(feature = "lazy_response")]
    pub interceptor: Option<InterceptorCallback<H, SHARED, HEADER_SIZE, QUERY_SIZE>>,
    functions: Vec<(String, String, WaterSingleFunction<H, SHARED, HEADER_SIZE, QUERY_SIZE>)>,
    pub apply_parents_middlewares: bool,
    #[cfg(feature = "lazy_response")]
    pub apply_parents_interceptors: bool,
    children: Vec<CapsuleWaterController<H, SHARED, HEADER_SIZE, QUERY_SIZE>>,
}

#[cfg(not(feature = "thread_shared_struct"))]
#[derive(Debug)]
pub struct CapsuleWaterController<
    #[cfg(feature = "use_tokio_send")]
    H: Send + 'static,
    #[cfg(not(feature = "use_tokio_send"))]
    H:'static,
    const HEADER_SIZE: usize,
    const QUERY_SIZE: usize,
> {
    pub(crate) father: Option<*const CapsuleWaterController<H, HEADER_SIZE, QUERY_SIZE>>,
    pub prefix: Option<&'static str>,
    pub middleware: Option<MiddlewareCallback<H, HEADER_SIZE, QUERY_SIZE>>,
    #[cfg(feature = "lazy_response")]
    pub interceptor: Option<InterceptorCallback<H, HEADER_SIZE, QUERY_SIZE>>,
    functions: Vec<(String, String, WaterSingleFunction<H, HEADER_SIZE, QUERY_SIZE>)>,
    pub apply_parents_middlewares: bool,
    #[cfg(feature = "lazy_response")]
    pub apply_parents_interceptors: bool,
    children: Vec<CapsuleWaterController<H, HEADER_SIZE, QUERY_SIZE>>,
}

/// Finder result alias
// #[cfg(feature = "thread_shared_struct")]
// type FFinderResult<H, SHARED, const HEADER_SIZE: usize, const QUERY_SIZE: usize> = (
//     &'static CapsuleWaterController<H, SHARED, HEADER_SIZE, QUERY_SIZE>,
//     &'static WaterSingleFunction<H, SHARED, HEADER_SIZE, QUERY_SIZE>,
//     Option<HashMap<String, String>>,
// );
//
// #[cfg(not(feature = "thread_shared_struct"))]
// type FFinderResult<H, const HEADER_SIZE: usize, const QUERY_SIZE: usize> = (
//     &'static CapsuleWaterController<H, HEADER_SIZE, QUERY_SIZE>,
//     &'static WaterSingleFunction<H, HEADER_SIZE, QUERY_SIZE>,
//     Option<HashMap<String, String>>,
// );

/// ----------------------------
/// unsafe impl Sync / Send as in your original code
/// ----------------------------

#[cfg(feature = "thread_shared_struct")]
unsafe impl<
    #[cfg(feature = "use_tokio_send")]
    H: Send + 'static,
    #[cfg(not(feature = "use_tokio_send"))]
    H,
    #[cfg(all(feature = "thread_shared_struct",not(feature = "use_tokio_send")))]
    SHARED:Clone,


    #[cfg(all(feature = "thread_shared_struct",feature = "use_tokio_send"))]
    SHARED:Clone + Send + 'static,
    const HEADER_SIZE: usize,
    const QUERY_SIZE: usize,
> Sync for CapsuleWaterController<H,SHARED, HEADER_SIZE, QUERY_SIZE> {}
#[cfg(not(feature = "thread_shared_struct"))]
unsafe impl<
    #[cfg(feature = "use_tokio_send")]
    H: Send + 'static,
    #[cfg(not(feature = "use_tokio_send"))]
    H,
    const HEADER_SIZE: usize,
    const QUERY_SIZE: usize,
> Sync for CapsuleWaterController<H, HEADER_SIZE, QUERY_SIZE> {}

#[cfg(all(feature = "use_tokio_send", not(feature = "thread_shared_struct")))]
unsafe impl<
    H: Send + 'static,
    const HEADER_SIZE: usize,
    const QUERY_SIZE: usize,
> Send for CapsuleWaterController<H, HEADER_SIZE, QUERY_SIZE> {}

#[cfg(all(feature = "use_tokio_send", feature = "thread_shared_struct"))]
unsafe impl<
    H: Send + 'static,
    SHARED: Clone + Send + 'static,
    const HEADER_SIZE: usize,
    const QUERY_SIZE: usize,
> Send for CapsuleWaterController<H, SHARED, HEADER_SIZE, QUERY_SIZE> {}

/// ----------------------------
/// Common impl body macro to avoid repetition
/// (the macro expands inside the appropriate `impl` block)
/// ----------------------------
macro_rules! controller_impl {
    () => {
        /// create new controller
        pub fn new() -> Self {
            #[cfg(feature = "lazy_response")]
             {
                 return Self {
                father: None,
                prefix: None,
                middleware: None,
                functions: vec![],
                apply_parents_middlewares: true,
                apply_parents_interceptors:true,
                                interceptor:None,
                children: vec![],
            }
             }
            #[cfg(not(feature = "lazy_response"))]
            {
                return   Self {
                father: None,
                prefix: None,
                middleware: None,
                functions: vec![],
                apply_parents_middlewares: true,
                children: vec![],
            }
            }
        }

        /// get father if exists
        pub fn get_father_controller<'fat>(&self) -> Option<&'fat Self> {
            if let Some(ptr) = self.father {
                // safety: we stored pointer to self during setup; user must call set_up or ____insure_binding
                unsafe { ptr.as_ref() }
            } else {
                None
            }
        }


        #[cfg(feature = "thread_shared_struct")]

        #[inline(always)]
        pub(crate) fn push_all_ancestors_middlewares(&'static self, vec: &mut Vec<&'static MiddlewareCallback<H,SHARED, HEADER_SIZE, QUERY_SIZE>>) {
            let mut oc = Some(self);
            loop {
                match oc {
                    None => break,
                    Some(controller) => {
                        match controller.middleware.as_ref() {
                            None => {
                                if controller.apply_parents_middlewares {
                                    match controller.get_father_controller() {
                                        None => break,
                                        Some(con) => {
                                            oc = Some(con);
                                            continue;
                                        }
                                    }
                                }
                                break
                            }
                            Some(middleware) => {
                                vec.push(middleware);
                                if !controller.apply_parents_middlewares {
                                    break;
                                }
                                oc = controller.get_father_controller();
                                continue;
                            }
                        }
                    }
                }
            }
            vec.reverse();
        }

        #[cfg(not(feature = "thread_shared_struct"))]
        #[inline(always)]
        pub(crate) fn push_all_ancestors_middlewares(&'static self, vec: &mut Vec<&'static MiddlewareCallback<H, HEADER_SIZE, QUERY_SIZE>>) {
            let mut oc = Some(self);
            loop {
                match oc {
                    None => break,
                    Some(controller) => {
                        match controller.middleware.as_ref() {
                            None => {
                                if controller.apply_parents_middlewares {
                                    match controller.get_father_controller() {
                                        None => break,
                                        Some(con) => {
                                            oc = Some(con);
                                            continue;
                                        }
                                    }
                                }
                                break
                            }
                            Some(middleware) => {
                                vec.push(middleware);
                                if !controller.apply_parents_middlewares {
                                    break;
                                }
                                oc = controller.get_father_controller();
                                continue;
                            }
                        }
                    }
                }
            }
            vec.reverse();
        }

        #[cfg(feature = "lazy_response")]
        #[cfg(feature = "thread_shared_struct")]
        #[inline(always)]
        pub(crate) fn push_all_ancestors_interceptors(&'static self, vec: &mut Vec<&'static InterceptorCallback<H, SHARED, HEADER_SIZE, QUERY_SIZE>>) {
            let mut oc = Some(self);
            loop {
                match oc {
                    None => break,
                    Some(controller) => {
                        match controller.interceptor.as_ref() {
                            None => {
                                if controller.apply_parents_interceptors {
                                    match controller.get_father_controller() {
                                        None => break,
                                        Some(con) => {
                                            oc = Some(con);
                                            continue;
                                        }
                                    }
                                }
                                break
                            }
                            Some(interceptor) => {
                                vec.push(interceptor);
                                if !controller.apply_parents_interceptors {
                                    break;
                                }
                                oc = controller.get_father_controller();
                                continue;
                            }
                        }
                    }
                }
            }
            vec.reverse()
        }


        #[cfg(feature = "lazy_response")]
        #[cfg(not(feature = "thread_shared_struct"))]
        #[inline(always)]
        pub(crate) fn push_all_ancestors_interceptors(&'static self, vec: &mut Vec<&'static InterceptorCallback<H, HEADER_SIZE, QUERY_SIZE>>) {
            let mut oc = Some(self);
            loop {
                match oc {
                    None => break,
                    Some(controller) => {
                        match controller.interceptor.as_ref() {
                            None => {
                                if controller.apply_parents_interceptors {
                                    match controller.get_father_controller() {
                                        None => break,
                                        Some(con) => {
                                            oc = Some(con);
                                            continue;
                                        }
                                    }
                                }
                                break
                            }
                            Some(interceptor) => {
                                vec.push(interceptor);
                                if !controller.apply_parents_interceptors {
                                    break;
                                }
                                oc = controller.get_father_controller();
                                continue;
                            }
                        }
                    }
                }
            }
            vec.reverse();
        }


        pub(crate) fn ____insure_binding(& mut self) {
            let self_pointer: *const Self = self;
            for child in &mut self.children {
                child.father = Some(self_pointer);
                child.____insure_binding();
            }
        }

        pub(crate) fn set_up(&mut self, mut father_prefixes: String) {
            if let Some(prefix) = &self.prefix {
                father_prefixes.push('/');
                father_prefixes.push_str(prefix);
                father_prefixes = father_prefixes.replace("//", "/");
            }

            for (method, path, _) in &mut self.functions {
                if let Some(index) = method.find('_') {
                    let name = &method[index + 1..];
                    if name.is_empty() { continue; }
                    push_named_route(name.to_string(), format!("{father_prefixes}/{path}").replace("//", "/"));
                    *method = (&method[..index]).to_uppercase();
                }
            }

            for child in &mut self.children {
                child.set_up(father_prefixes.clone());
            }
        }

        #[cfg(feature = "thread_shared_struct")]
        pub fn push_handler(&mut self, function: (String, String, WaterSingleFunction<H,SHARED, HEADER_SIZE, QUERY_SIZE>)) {
            self.functions.push(function);
        }

        #[cfg(not(feature = "thread_shared_struct"))]
        pub fn push_handler(&mut self, function: (String, String, WaterSingleFunction<H, HEADER_SIZE, QUERY_SIZE>)) {
            self.functions.push(function);
        }
        pub fn push_controller(&mut self, controller: Self) {
            self.children.push(controller);
        }

    };
}

/// ----------------------------
/// impl blocks expanding common macro
/// ----------------------------

#[cfg(feature = "thread_shared_struct")]
impl<
    #[cfg(feature = "use_tokio_send")]
    H: Send + 'static,
    #[cfg(not(feature = "use_tokio_send"))]
    H,

    #[cfg(all(not(feature = "use_tokio_send")))]
    SHARED:Clone,

    #[cfg(all(feature = "use_tokio_send"))]
    SHARED:Clone + Send + 'static,
    const HEADER_SIZE: usize,
    const QUERY_SIZE: usize,
> CapsuleWaterController<H, SHARED, HEADER_SIZE, QUERY_SIZE> {
    controller_impl!();
}

#[cfg(not(feature = "thread_shared_struct"))]
impl<
    #[cfg(feature = "use_tokio_send")]
    H: Send + 'static,
    #[cfg(not(feature = "use_tokio_send"))]
    H,
    const HEADER_SIZE: usize,
    const QUERY_SIZE: usize,
> CapsuleWaterController<H, HEADER_SIZE, QUERY_SIZE> {
    controller_impl!();
}

/// ----------------------------
/// middleware result enum
/// ----------------------------
pub enum MiddlewareResult {
    Pass,
    Stop,
}
