#![allow(static_mut_refs)]
use std::collections::HashMap;
use crate::server::capsule::WaterSingleFunction;
use crate::server::{CapsuleWaterController, MiddlewareCallback};
#[cfg(feature = "lazy_response")]
use crate::server::capsule::response_interceptors::InterceptorCallback;

#[cfg(not(feature = "thread_shared_struct"))]
pub struct CapsuleHolder<
    #[cfg(feature = "use_tokio_send")]
    H:'static + Send,
    #[cfg(not(feature = "use_tokio_send"))]
    H:'static,

    const HS:usize,const QS:usize> {
    pub (crate) method:String,
    pub (crate)  controller:&'static CapsuleWaterController<H,HS,QS>,
    pub (crate)  func:&'static WaterSingleFunction<H, HS,QS >,
    pub (crate) father_middlewares:Vec<&'static MiddlewareCallback<H,HS,QS>>,
    #[cfg(feature = "lazy_response")]
    pub (crate) father_interceptors: Vec<&'static InterceptorCallback<H, HS, QS>>,
}

#[cfg(feature = "thread_shared_struct")]
pub struct CapsuleHolder<
    #[cfg(feature = "use_tokio_send")]
    H:'static + Send,
    #[cfg(not(feature = "use_tokio_send"))]
    H:'static,
    #[cfg(feature = "use_tokio_send")]
    SHARED:Clone+'static + Send,
    #[cfg(not(feature = "use_tokio_send"))]
    SHARED:Clone+'static ,
    const HS:usize,
    const QS:usize> {

    pub (crate) method:String,
    pub (crate) controller:&'static CapsuleWaterController<H,SHARED,HS,QS>,
    pub (crate) func:&'static WaterSingleFunction<H,SHARED, HS,QS, >,
    pub (crate) father_middlewares:Vec<&'static MiddlewareCallback<H,SHARED,HS,QS>>,
    #[cfg(feature = "lazy_response")]
    pub (crate) father_interceptors: Vec<&'static InterceptorCallback<H,SHARED,HS, QS>>,
}

// -------------------- Types --------------------
#[cfg(not(feature = "thread_shared_struct"))]
pub type PathHolder<H,const QS:usize,const HS:usize> = CapsuleHolder<H,QS,HS>;

#[cfg(feature = "thread_shared_struct")]
pub type PathHolder<H,SHARED,const QS:usize,const HS:usize> = CapsuleHolder<H,SHARED,QS,HS>;

#[cfg(feature = "thread_shared_struct")]
pub  type DynamicPathVec<H,SHARED,const QS:usize,const HS:usize> = Vec<(String, PathHolder<H,SHARED,QS,HS>)>;
#[cfg(not(feature = "thread_shared_struct"))]
pub type DynamicPathVec<H,const QS:usize,const HS:usize> = Vec<(String, PathHolder<H,QS,HS>)>;



#[cfg(feature = "thread_shared_struct")]
pub struct MatcherInitializer<'a,
    #[cfg(feature = "use_tokio_send")]
    H:'static + Send,
    #[cfg(not(feature = "use_tokio_send"))]
    H:'static,
    #[cfg(feature = "use_tokio_send")]
    SHARED:Clone+'static + Send,
    #[cfg(not(feature = "use_tokio_send"))]
    SHARED:Clone+'static ,
    const QS:usize,const HS:usize> {
    static_paths:&'a mut HashMap<String,PathHolder<H,SHARED,QS,HS>>,
    dynamic_paths:&'a mut HashMap<usize,DynamicPathVec<H,SHARED,QS,HS>>,

}


#[cfg(not(feature = "thread_shared_struct"))]
pub struct MatcherInitializer<'a,
    #[cfg(feature = "use_tokio_send")]
    H:'static + Send,
    #[cfg(not(feature = "use_tokio_send"))]
    H:'static,
    const QS:usize,const HS:usize> {
    static_paths:&'a mut HashMap<String,PathHolder<H,QS,HS>>,
    dynamic_paths:&'a mut HashMap<usize,DynamicPathVec<H,QS,HS>>,
}


#[cfg(feature = "thread_shared_struct")]
#[derive(Clone)]
pub (crate) struct Matcher<
    #[cfg(feature = "use_tokio_send")]
    H:'static + Send,
    #[cfg(not(feature = "use_tokio_send"))]
    H:'static,
    #[cfg(feature = "use_tokio_send")]
    SHARED:Clone+'static + Send,
    #[cfg(not(feature = "use_tokio_send"))]
    SHARED:Clone+'static ,
    const QS:usize,const HS:usize> {
    pub(crate) static_paths:&'static  HashMap<String,PathHolder<H,SHARED,QS,HS>>,
    pub(crate) dynamic_paths:&'static HashMap<usize,DynamicPathVec<H,SHARED,QS,HS>>,

}


#[cfg(feature = "thread_shared_struct")]
impl < #[cfg(feature = "use_tokio_send")]
H:'static + Send,
    #[cfg(not(feature = "use_tokio_send"))]
    H:'static,
    #[cfg(feature = "use_tokio_send")]
    SHARED:Clone+'static + Send,
    #[cfg(not(feature = "use_tokio_send"))]
    SHARED:Clone+'static ,const HS:usize,const QS:usize>  Matcher<H,SHARED,QS,HS> {

    pub fn clone(&self)->Matcher<H,SHARED,QS,HS>{
        Matcher {
            static_paths:self.static_paths,
            dynamic_paths:self.dynamic_paths
        }
    }

    pub fn new(
        static_paths:&'static  HashMap<String,PathHolder<H,SHARED,QS,HS>>,
        dynamic_paths:&'static  HashMap<usize,DynamicPathVec<H,SHARED,QS,HS>>,
    )->Matcher<H,SHARED,QS,HS>{
        Matcher {
            static_paths,
            dynamic_paths
        }
    }

    #[inline(always)]
    pub fn match_path<'a>(
        &'a self,
        path: &'a str,
    ) -> Option<(&'a PathHolder<H,SHARED, QS, HS>, Option<HashMap<String,String>>)> {
        let path = path.trim_matches('/').split("?").next().unwrap();
        let segment_count = path.split('/').count();

        // 1️⃣ static
        if let Some(holder) = self.static_paths.get(path) {
            return Some((holder, None));
        }

        // 2️⃣ dynamic
        if let Some(dynamic_vec) = self.dynamic_paths.get(&segment_count) {
            'outer: for (original_path, holder) in dynamic_vec {
                let mut params = HashMap::new();
                let mut incoming_iter = path.split('/');
                let mut original_iter = original_path.split('/');

                while let (Some(opart), Some(ipart)) =
                    (original_iter.next(), incoming_iter.next())
                {
                    if opart.starts_with('{') && opart.ends_with('}') {
                        let name = &opart[1..opart.len() - 1];
                        params.insert(name.to_string(),ipart.to_string());
                    } else if opart != ipart {
                        continue 'outer;
                    }
                }

                if original_iter.next().is_none()
                    && incoming_iter.next().is_none()
                {
                    return Some((holder, Some(params)));
                }
            }
        }

        for check_count in (1..=segment_count).rev() {
            if let Some(dynamic_vec) = self.dynamic_paths.get(&check_count) {
                for (p, holder) in dynamic_vec {
                    if p.contains(">>") || p.contains("/*") || p.contains("<<") {
                        let base_prefix = p.split('{').next().unwrap_or("");
                        if path.starts_with(base_prefix) {
                            let mut params = HashMap::new();

                            if let Some(start_idx) = p.find('{') {
                                if let Some(end_idx) = p.find('}') {
                                    let param_key = &p[start_idx + 1..end_idx];

                                    let tail_value = &path[base_prefix.len()..];
                                    params.insert(param_key.to_string(), tail_value.to_string());
                                    return Some((holder, Some(params)));
                                }
                            }
                        }
                    }
                }
            }
        }
        None
    }
}



#[cfg(not(feature = "thread_shared_struct"))]
#[derive(Clone)]
pub struct Matcher<
    #[cfg(feature = "use_tokio_send")]
    H:'static + Send,
    #[cfg(not(feature = "use_tokio_send"))]
    H:'static,
    const QS:usize,const HS:usize> {
    pub(crate) static_paths:&'static  HashMap<String,PathHolder<H,QS,HS>>,
    pub(crate) dynamic_paths:&'static  HashMap<usize,DynamicPathVec<H,QS,HS>>,
}



#[cfg(not(feature = "thread_shared_struct"))]
impl <
    #[cfg(feature = "use_tokio_send")]
    H:'static + Send,
    #[cfg(not(feature = "use_tokio_send"))]
    H:'static,
    const QS:usize,
    const HS:usize
>
Matcher<H,QS,HS> {

    pub fn clone(&self)->Matcher<H,QS,HS>{
        Matcher {
            static_paths:self.static_paths,
            dynamic_paths:self.dynamic_paths
        }
    }
    pub fn new(
        static_paths:&'static  HashMap<String,PathHolder<H,QS,HS>>,
        dynamic_paths:&'static  HashMap<usize,DynamicPathVec<H,QS,HS>>,
    )->Matcher<H,QS,HS>{
        Matcher {
            static_paths,
            dynamic_paths
        }
    }
    pub fn match_path<'a>(
        &'a self,
        path: &'a str,
    ) -> Option<(&'a PathHolder<H, QS, HS>, Option<HashMap<String,String>>)> {
        let path = path.trim_matches('/').split("?").next().unwrap();

        // 1️⃣ static
        if let Some(holder) = self.static_paths.get(path) {
            return Some((holder, None));
        }
        let segment_count = path.split('/').count();

        // 2️⃣ dynamic
        if let Some(dynamic_vec) = self.dynamic_paths.get(&segment_count) {
            'outer: for (original_path, holder) in dynamic_vec {
                let mut params = HashMap::new();
                let mut incoming_iter = path.split('/');
                let mut original_iter = original_path.split('/');

                while let (Some(opart), Some(ipart)) =
                    (original_iter.next(), incoming_iter.next())
                {
                    if opart.starts_with('{') && opart.ends_with('}') {
                        let name = &opart[1..opart.len() - 1];
                        params.insert(name.to_string(),ipart.to_string());
                    } else if opart != ipart {
                        continue 'outer;
                    }
                }

                if original_iter.next().is_none()
                    && incoming_iter.next().is_none()
                {
                    return Some((holder, Some(params)));
                }
            }
        }

        for check_count in (1..=segment_count).rev() {
            if let Some(dynamic_vec) = self.dynamic_paths.get(&check_count) {
                for (p, holder) in dynamic_vec {
                    if p.contains(">>") || p.contains("/*") || p.contains("<<") {
                        let base_prefix = p.split('{').next().unwrap_or("");
                        if path.starts_with(base_prefix) {
                            let mut params = HashMap::new();

                            if let Some(start_idx) = p.find('{') {
                                if let Some(end_idx) = p.find('}') {
                                    let param_key = &p[start_idx + 1..end_idx];

                                    let tail_value = &path[base_prefix.len()..];
                                    params.insert(param_key.to_string(), tail_value.to_string());
                                    return Some((holder, Some(params)));
                                }
                            }
                        }
                    }
                }
            }
        }

        None
    }
}

#[cfg(not(feature = "thread_shared_struct"))]
impl <
    #[cfg(feature = "use_tokio_send")]
    H:'static + Send,
    #[cfg(not(feature = "use_tokio_send"))]
    H:'static,
    const HS:usize,const QS:usize>  MatcherInitializer<'_,H,HS,QS> {

    pub fn format(
        prefix_string:&mut String,
        static_paths:& mut HashMap<String,PathHolder<H,HS,QS>>,
        dynamic_paths:& mut HashMap<usize,DynamicPathVec<H,HS,QS>>,
        controller:&'static CapsuleWaterController<H,HS,QS>,
        mut father_controllers:Vec<&'static CapsuleWaterController<H,HS,QS>>
    ){

        if let Some(p) = controller.prefix.as_ref() {
            prefix_string.push_str("/");
            prefix_string.push_str(*p);
        }

        for (method,path,func) in &controller.functions {
            let mut controllers_vec:Vec<&'static CapsuleWaterController<H,HS,QS>> = vec![];
            for c in &father_controllers {
                controllers_vec.push(*c);
            }
            let path = format!("{prefix_string}/{path}").replace("//","/").trim_matches(|c| c=='/').to_string();
            if path.contains("{") || path.contains("}") {
                let s = path.split("/");
                let mut father_middlewares = vec![];
                #[cfg(feature = "lazy_response")]
                let mut father_interceptors = vec![];
                controller.push_all_ancestors_middlewares(&mut father_middlewares);
                #[cfg(feature = "lazy_response")]
                controller.push_all_ancestors_interceptors(&mut father_interceptors);
                #[cfg(feature = "lazy_response")]
                dynamic_paths.insert(
                    s.count()
                    ,
                    vec![
                        (path,CapsuleHolder{
                            method:method.split("_").next().unwrap().to_uppercase(),
                            controller,
                            func,

                            father_middlewares,
                            father_interceptors
                        })
                    ]
                );

                #[cfg(not(feature = "lazy_response"))]
                dynamic_paths.insert(
                    s.count()
                    ,
                    vec![
                        (path,CapsuleHolder{
                            method:method.split("_").next().unwrap().to_uppercase(),
                            controller,
                            func,

                            father_middlewares,
                        })
                    ]
                );
                continue
            }
            let mut father_middlewares = vec![];
            #[cfg(feature = "lazy_response")]
            let mut father_interceptors = vec![];

            controller.push_all_ancestors_middlewares(&mut father_middlewares);
            #[cfg(feature = "lazy_response")]
            controller.push_all_ancestors_interceptors(&mut father_interceptors);
            #[cfg(feature = "lazy_response")]
            static_paths.insert(path,CapsuleHolder{
                method:method.split("_").next().unwrap().to_uppercase(),
                controller,
                func,
                father_middlewares,
                father_interceptors
            });
            #[cfg(not(feature = "lazy_response"))]
            static_paths.insert(path,CapsuleHolder{
                method:method.split("_").next().unwrap().to_uppercase(),
                controller,
                func,
                father_middlewares,
            });
        }
        father_controllers.push(controller);

        for child in &controller.children {
            let mut controllers_vec:Vec<&'static CapsuleWaterController<H,HS,QS>> = vec![];
            for c in &father_controllers {
                controllers_vec.push(*c);
            }
            Self::format(
                prefix_string,
                static_paths,
                dynamic_paths,
                child,
                controllers_vec
            )
        }
    }


    pub (crate) fn serialize<'a>(
        static_paths:&'a mut HashMap<String,PathHolder<H,HS,QS>>,
        dynamic_paths:&'a mut HashMap<usize,DynamicPathVec<H,HS,QS>>,
        controller:&'static CapsuleWaterController<H,HS,QS>
    )->MatcherInitializer<'a,H,HS,QS>{
        let mut p = String::new();
        Self::format(

            &mut p,
            static_paths,
            dynamic_paths,
            controller,
            vec![]
        );

        let res = MatcherInitializer {static_paths,dynamic_paths};

        res
    }



    fn get_dynamic_vec(
        &mut self,
        len: usize,
    ) -> &mut DynamicPathVec<H, HS,QS, > {
        self.dynamic_paths.entry(len).or_insert_with(Vec::new)
    }

    pub fn save_path(
        &mut self,
        path: &str,
        path_holder: PathHolder<H,  HS,QS>,
    ) {
        let path = path.trim_matches('/');

        if path.contains('{') || path.contains("/:") {
            let count = path.split('/').count();
            let v = self.get_dynamic_vec(count);
            v.push((path.to_string(), path_holder));
            return;
        }

        self.static_paths.insert(path.to_string(), path_holder);
    }

}


#[cfg(feature = "thread_shared_struct")]
impl <
    #[cfg(feature = "use_tokio_send")]
    H:'static + Send,
    #[cfg(not(feature = "use_tokio_send"))]
    H:'static,
    #[cfg(feature = "use_tokio_send")]
    SHARED:Clone+'static + Send,
    #[cfg(not(feature = "use_tokio_send"))]
    SHARED:Clone+'static ,
    const QS:usize,const HS:usize>  MatcherInitializer<'_,H,SHARED,HS,QS> {

    pub fn format(
        prefix_string:&mut String,
        static_paths:& mut HashMap<String,PathHolder<H,SHARED,HS,QS>>,
        dynamic_paths:& mut HashMap<usize,DynamicPathVec<H,SHARED,HS,QS>>,
        controller:&'static CapsuleWaterController<H,SHARED,HS,QS>,
      ){

        if let Some(p) = controller.prefix.as_ref() {
            prefix_string.push_str("/");
            prefix_string.push_str(*p);}

        for (method,path,func) in &controller.functions {
            let path = format!("{prefix_string}/{path}").replace("//","/").trim_matches(|c| c=='/').to_string();
            if path.contains("{") || path.contains("}") {
                let s = path.split("/");
                let mut father_middlewares = vec![];
                #[cfg(feature = "lazy_response")]
                let mut father_interceptors = vec![];

                controller.push_all_ancestors_middlewares(&mut father_middlewares);
                #[cfg(feature = "lazy_response")]
                controller.push_all_ancestors_interceptors(&mut father_interceptors);
                #[cfg(feature = "lazy_response")]
                dynamic_paths.insert(
                    s.count()
                    ,
                    vec![
                        (path,CapsuleHolder{
                            method:method.split("_").next().unwrap().to_uppercase(),
                            controller,
                            func,
                            father_middlewares,
                            father_interceptors
                        })
                    ]
                );
                #[cfg(not(feature = "lazy_response"))]
                dynamic_paths.insert(
                    s.count()
                    ,
                    vec![
                        (path,CapsuleHolder{
                            method:method.split("_").next().unwrap().to_uppercase(),
                            controller,
                            func,
                            father_middlewares,
                        })
                    ]
                );
                continue
            }
            let mut father_middlewares = vec![];
            #[cfg(feature = "lazy_response")]
            let mut father_interceptors = vec![];

            controller.push_all_ancestors_middlewares(&mut father_middlewares);
            #[cfg(feature = "lazy_response")]
            controller.push_all_ancestors_interceptors(&mut father_interceptors);
            #[cfg(feature = "lazy_response")]

            static_paths.insert(path,CapsuleHolder{
                method:method.split("_").next().unwrap().to_uppercase(),
                controller,
                func,
                father_middlewares,
                father_interceptors
            });
            #[cfg(not(feature = "lazy_response"))]
            static_paths.insert(path,CapsuleHolder{
                method:method.split("_").next().unwrap().to_uppercase(),
                controller,
                func,
                father_middlewares,
            });
        }

        for child in &controller.children {

            Self::format(
                prefix_string,
                static_paths,
                dynamic_paths,
                child,
            )
        }
    }


    pub (crate) fn serialize<'a>(
        static_paths:&'a mut HashMap<String,PathHolder<H,SHARED,HS,QS>>,
        dynamic_paths:&'a mut HashMap<usize,DynamicPathVec<H,SHARED,HS,QS>>,
        controller:&'static CapsuleWaterController<H,SHARED,HS,QS>
    )->MatcherInitializer<'a,H,SHARED,HS,QS>{
        let mut p = String::new();
        Self::format(

            &mut p,
            static_paths,
            dynamic_paths,
            controller,
        );

        let res = MatcherInitializer {static_paths,dynamic_paths};

        res
    }



    fn get_dynamic_vec(
        &mut self,
        len: usize,
    ) -> &mut DynamicPathVec<H, SHARED,  HS,QS> {
        self.dynamic_paths.entry(len).or_insert_with(Vec::new)
    }

    pub fn save_path(
        &mut self,
        path: &str,
        path_holder: PathHolder<H, SHARED, HS,QS>,
    ) {
        let path = path.trim_matches('/');

        if path.contains('{') || path.contains("/:") {
            let count = path.split('/').count();
            let v = self.get_dynamic_vec(count);
            v.push((path.to_string(), path_holder));
            return;
        }

        self.static_paths.insert(path.to_string(), path_holder);
    }

    // pub fn match_path<'a>(
    //     &'a self,
    //     path: &'a str,
    // ) -> Option<(&'a PathHolder<H, SHARED, HS,QS>, Vec<(&'a str, &'a str)>)> {
    //     let path = path.trim_matches('/');
    //     let segment_count = path.split('/').count();
    //
    //     // 1️⃣ static
    //     if let Some(holder) = self.static_paths.get(path) {
    //         return Some((holder, Vec::new()));
    //     }
    //
    //     // 2️⃣ dynamic
    //     if let Some(dynamic_vec) = self.dynamic_paths.get(&segment_count) {
    //         'outer: for (original_path, holder) in dynamic_vec {
    //             let mut params = Vec::new();
    //             let mut incoming_iter = path.split('/');
    //             let mut original_iter = original_path.split('/');
    //
    //             while let (Some(opart), Some(ipart)) =
    //                 (original_iter.next(), incoming_iter.next())
    //             {
    //                 if opart.starts_with('{') && opart.ends_with('}') {
    //                     let name = &opart[1..opart.len() - 1];
    //                     params.push((name, ipart));
    //                 } else if opart != ipart {
    //                     continue 'outer;
    //                 }
    //             }
    //
    //             if original_iter.next().is_none()
    //                 && incoming_iter.next().is_none()
    //             {
    //                 return Some((holder, params));
    //             }
    //         }
    //     }
    //
    //
    //     for check_count in (1..=segment_count).rev() {
    //         if let Some(dynamic_vec) = self.dynamic_paths.get(&check_count) {
    //             for (p, holder) in dynamic_vec {
    //                 if p.contains(">>") || p.contains("/*") || p.contains("<<") {
    //                     let base_prefix = p.split('{').next().unwrap_or("");
    //                     if path.starts_with(base_prefix) {
    //                         let mut params = HashMap::new();
    //
    //                         if let Some(start_idx) = p.find('{') {
    //                             if let Some(end_idx) = p.find('}') {
    //                                 let param_key = &p[start_idx + 1..end_idx];
    //
    //                                 let tail_value = &path[base_prefix.len()..];
    //                                 params.insert(param_key.to_string(), tail_value.to_string());
    //                                 return Some((holder, Some(params)));
    //                             }
    //                         }
    //                     }
    //                 }
    //             }
    //         }
    //     }
    //     None
    // }

}


