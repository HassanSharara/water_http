use crate::http::HttpSender;
#[cfg(feature = "enable_dynamic_body_cache")]
use crate::http::request::DynamicBodyMap;
use crate::http::status_code::HttpStatusCode;

pub enum HeaderInterceptorApplyFor {
    All,
    Specific(HttpStatusCode),
}

impl HeaderInterceptorApplyFor {

    pub fn can_apply(&self,status:&HttpStatusCode)->bool{
        match self
        {
            HeaderInterceptorApplyFor::All => { true}
            HeaderInterceptorApplyFor::Specific(s) => {
                s.status == status.status
            }
        }
    }
}


pub(crate) type HeaderInterceptorFunction<const HEADER_SIZE:usize, const QUERY_SIZE: usize> =
    for<'a,'context> fn (&mut HttpSender<'a,'context,HEADER_SIZE,QUERY_SIZE>);
pub struct ContextAccessories<const HEADER_SIZE:usize, const QUERY_SIZE: usize> {
    pub (crate) initial_interceptor:Option<(HeaderInterceptorApplyFor,HeaderInterceptorFunction<HEADER_SIZE,QUERY_SIZE>)>,
    #[cfg(feature = "enable_dynamic_body_cache")]
    pub cached_body:Option<DynamicBodyMap>
}
impl <const HEADER_SIZE:usize, const QUERY_SIZE: usize>
ContextAccessories<HEADER_SIZE,QUERY_SIZE> {

    pub fn default() -> Self {
        #[cfg(not(feature = "enable_dynamic_body_cache"))]
        {
            return  ContextAccessories {
                initial_interceptor:None
            }
        }
        #[cfg(feature = "enable_dynamic_body_cache")]
        ContextAccessories {
            initial_interceptor:None,
            cached_body:None
        }
    }


}

