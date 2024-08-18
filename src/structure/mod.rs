pub mod context_route_function_finder;
use std::future::Future;
use std::pin::Pin;
use crate::framework_http::HttpContext;

pub type HttpContextRCFunction<T> =for <'a> fn(&'a mut HttpContext<T>) -> Pin<Box<dyn Future<Output = ()> + Send + 'a >>;
pub type HttpMiddleWare<T> =  for <'a> fn(&'a mut HttpContext<T>)->Pin<Box<dyn Future <Output = MiddlewareResult> + Send
    +'a
>>;

#[derive(Debug)]
pub struct HttpContextRController<T:Send + 'static>{
    pub father_controller:Option<*const HttpContextRController<T>>,
    pub prefix:Option<String>,
    pub middleware:Option<HttpMiddleWare<T>>,
    pub functions:Vec<(String,String,HttpContextRCFunction<T>)>,
    pub children:Vec<HttpContextRController<T>>,
    pub apply_parents_middlewares:bool
}

unsafe impl<T:Send + 'static> Sync for HttpContextRController<T> {

}

impl <T:Send> HttpContextRController<T> {
    #[async_recursion::async_recursion]
    async fn try_passing_all_middlewares(&self,context:&mut HttpContext<T>)->MiddlewareResult{
        if self.apply_parents_middlewares {
           unsafe  {
               if let Some(father_controller) = self.father_controller.as_ref() {
                   let father_controller  = father_controller.as_ref();

                   let father_middlewares_results =
                       father_controller.expect("encounter non expected error while using father controller pointer").try_passing_all_middlewares(context).await;
                   if let MiddlewareResult::Stop = father_middlewares_results {
                       return father_middlewares_results;
                   }
               }
           }
        }
        if let Some(middleware) = self.middleware.as_ref() {
            return middleware(context).await;
        }
        MiddlewareResult::Pass
    }

    pub fn new()->Self{
        HttpContextRController{
            father_controller:None,
            prefix:None,
            middleware:None,
            functions:vec![],
            children:vec![],
            apply_parents_middlewares:true
        }
    }


    pub fn ____insure_binding(&'static mut self){
        let self_pointer :*const Self = self;
        for child in &mut self.children {
            child.father_controller = Some(self_pointer);
            child.____insure_binding();
        }
    }
}
pub enum MiddlewareResult {
    Pass,
    Stop,
}


pub enum RequestMethods {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
    HEAD,
    OPTIONS,
    CONNECT,
    TRACE,
}
