/// defining all the used and imported macros for building your app struct
pub mod capsule_macros;

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use crate::server::{HttpContext, push_named_route};

/// ----------------------------
/// Future aliases (Send vs non-Send)
/// ----------------------------
#[cfg(feature = "use_tokio_send")]
type BoxFutureSend<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

#[cfg(not(feature = "use_tokio_send"))]
type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

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
    H,

    #[cfg(not(feature = "use_tokio_send"))]
    SHARED:Clone,

    #[cfg(feature = "use_tokio_send")]
    SHARED:Clone + Send + 'static,
    const HEADER_SIZE: usize,
    const QUERY_SIZE: usize,
> {
    pub(crate) father: Option<*const CapsuleWaterController<H, SHARED, HEADER_SIZE, QUERY_SIZE>>,
    pub prefix: Option<&'static str>,
    pub middleware: Option<MiddlewareCallback<H, SHARED, HEADER_SIZE, QUERY_SIZE>>,
    functions: Vec<(String, String, WaterSingleFunction<H, SHARED, HEADER_SIZE, QUERY_SIZE>)>,
    pub apply_parents_middlewares: bool,
    children: Vec<CapsuleWaterController<H, SHARED, HEADER_SIZE, QUERY_SIZE>>,
}

#[cfg(not(feature = "thread_shared_struct"))]
#[derive(Debug)]
pub struct CapsuleWaterController<
    #[cfg(feature = "use_tokio_send")]
    H: Send + 'static,
    #[cfg(not(feature = "use_tokio_send"))]
    H,
    const HEADER_SIZE: usize,
    const QUERY_SIZE: usize,
> {
    pub(crate) father: Option<*const CapsuleWaterController<H, HEADER_SIZE, QUERY_SIZE>>,
    pub prefix: Option<&'static str>,
    pub middleware: Option<MiddlewareCallback<H, HEADER_SIZE, QUERY_SIZE>>,
    functions: Vec<(String, String, WaterSingleFunction<H, HEADER_SIZE, QUERY_SIZE>)>,
    pub apply_parents_middlewares: bool,
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
            Self {
                father: None,
                prefix: None,
                middleware: None,
                functions: vec![],
                apply_parents_middlewares: true,
                children: vec![],
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


        pub(crate) fn ____insure_binding(&'static mut self) {
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

        pub(crate) fn get_prefix(&self) -> Option<&str> {
            self.prefix.map(Self::shave_path)
        }

        pub(crate) fn shave_path(mut input: &str) -> &str {
            while input.starts_with('/') {
                input = &input[1..];
            }
            while input.ends_with('/') {
                let len = input.len();
                if len == 1 { return ""; }
                input = &input[..len-1];
            }
            input
        }

        pub(crate) const fn all_rest_path_braces() -> &'static str {
            "{allRestPath}"
        }
        pub(crate) const fn all_rest_path() -> &'static str {
            "allRestPath"
        }

        pub(crate) fn check_if_paths_are_equals(incoming_path: &str, cp: &str) -> (bool, Option<HashMap<String, String>>) {
            let _s_pattern = Self::all_rest_path_braces();
            if let Some(index) = cp.find(_s_pattern) {
                let first = Self::shave_path(&cp[..index]);
                if incoming_path.starts_with(first) {
                    let mut map: HashMap<String, String> = HashMap::new();
                    map.insert(Self::all_rest_path().to_string(), (&incoming_path[first.len()..]).to_string());
                    return (true, Some(map));
                }
            }

            let inc_splitter: Vec<&str> = incoming_path.split('/').collect();
            let cp_splitter: Vec<&str> = cp.split('/').collect();
            const ERR: (bool, Option<HashMap<String, String>>) = (false, None);
            if inc_splitter.len() != cp_splitter.len() { return ERR; }
            let mut map: Option<HashMap<String, String>> = None;
            for (index, part) in cp_splitter.iter().enumerate() {
                let inc_part = inc_splitter[index];
                 if index + 1 == cp_splitter.len() {
                    if let Some(qi) = inc_part.find("?"){
                        if &inc_part[..qi] == *part { return (true,None)}
                    }
                }
                let containing_arcs = part.contains('{') && part.contains('}');
                if part != &inc_part && !containing_arcs {
                    return ERR;
                }

                if containing_arcs {
                    match &mut map {
                        None => {
                            let mut n_map = HashMap::new();
                            n_map.insert((&part[1..part.len()-1]).to_string(), inc_part.to_string());
                            map = Some(n_map);
                        }
                        Some(ref mut m) => {
                            m.insert((&part[1..part.len()-1]).to_string(), inc_part.to_string());
                        }
                    }
                }
            }
            (true, map)
        }

        #[cfg(feature = "thread_shared_struct")]
         pub(crate) fn find_function(
            &'static self,
            original_path: &str,
            original_method: &str
        ) -> Option<(
            &'static Self,
            &'static WaterSingleFunction<H,SHARED, HEADER_SIZE, QUERY_SIZE>,
            Option<HashMap<String, String>>
        )> {
            let mut path = Self::shave_path(original_path);
            if let Some(prefix) = self.get_prefix() {
                if !path.starts_with(prefix) {
                    return None;
                }
                let prefix_in_length = prefix.len() + 1;
                if path.len() <= prefix_in_length { return None; }
                path = &path[prefix_in_length..];
            }

            for (method, cp, func) in &self.functions {
                if method != original_method && method.to_uppercase() != original_method {
                    continue;
                }
                let (result, params) = Self::check_if_paths_are_equals(path, Self::shave_path(cp));
                println!("path is {path} while cp is {cp}");
                if !result { continue; }
                return Some((self, func, params));
            }

            for child in &self.children {
                if let Some(found) = child.find_function(path, original_method) {
                    return Some(found);
                }
            }
            None
        }
        #[cfg(not(feature = "thread_shared_struct"))]
        pub(crate) fn find_function(
            &'static self,
            original_path: &str,
            original_method: &str
        ) -> Option<(
            &'static Self,
            &'static WaterSingleFunction<H, HEADER_SIZE, QUERY_SIZE>,
            Option<HashMap<String, String>>
        )> {
            let mut path = Self::shave_path(original_path);
            if let Some(prefix) = self.get_prefix() {
                if !path.starts_with(prefix) {
                    return None;
                }
                let prefix_in_length = prefix.len() + 1;
                if path.len() <= prefix_in_length { return None; }
                path = &path[prefix_in_length..];
            }

            for (method, cp, func) in &self.functions {
                if method != original_method && method.to_uppercase() != original_method {
                    continue;
                }
                let (result, params) = Self::check_if_paths_are_equals(path, Self::shave_path(cp));
                if !result { continue; }
                return Some((self, func, params));
            }

            for child in &self.children {
                if let Some(found) = child.find_function(path, original_method) {
                    return Some(found);
                }
            }
            None
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
