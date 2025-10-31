#![allow(static_mut_refs)]
use std::collections::HashMap;
use crate::server::capsule::WaterSingleFunction;
use crate::server::{CapsuleWaterController, MiddlewareCallback};

#[cfg(not(feature = "thread_shared_struct"))]
pub struct CapsuleHolder<H:'static,const HS:usize,const QS:usize> {
    pub (crate) method:String,
    pub (crate)  controller:&'static CapsuleWaterController<H,HS,QS>,
    pub (crate)  func:&'static WaterSingleFunction<H, HS,QS >,
    pub (crate)  father_controllers:Vec<&'static CapsuleWaterController<H,HS,QS>>,
    pub (crate) father_middlewares:Vec<&'static MiddlewareCallback<H,HS,QS>>
}

#[cfg(feature = "thread_shared_struct")]
pub struct CapsuleHolder<
    H:'static,
    SHARED:Clone+'static,
    const HS:usize,
    const QS:usize> {

    pub (crate) method:String,
    pub (crate) controller:&'static CapsuleWaterController<H,SHARED,HS,QS>,
    pub (crate) func:&'static WaterSingleFunction<H,SHARED, HS,QS, >,
    pub (crate) father_middlewares:Vec<&'static MiddlewareCallback<H,SHARED,HS,QS>>
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
pub struct MatcherInitializer<'a,H:'static,SHARED:Clone + 'static,const QS:usize,const HS:usize> {
    static_paths:&'a mut HashMap<String,PathHolder<H,SHARED,QS,HS>>,
    dynamic_paths:&'a mut HashMap<usize,DynamicPathVec<H,SHARED,QS,HS>>,

}


#[cfg(not(feature = "thread_shared_struct"))]
pub struct MatcherInitializer<'a,H:'static,const QS:usize,const HS:usize> {
    static_paths:&'a mut HashMap<String,PathHolder<H,QS,HS>>,
    dynamic_paths:&'a mut HashMap<usize,DynamicPathVec<H,QS,HS>>,
}


#[cfg(feature = "thread_shared_struct")]
#[derive(Clone)]
pub (crate) struct Matcher<H:'static,SHARED:Clone + 'static,const QS:usize,const HS:usize> {
    pub(crate) static_paths:&'static  HashMap<String,PathHolder<H,SHARED,QS,HS>>,
    pub(crate) dynamic_paths:&'static HashMap<usize,DynamicPathVec<H,SHARED,QS,HS>>,

}


#[cfg(feature = "thread_shared_struct")]
impl <H:'static,SHARED:Clone,const HS:usize,const QS:usize>  Matcher<H,SHARED,QS,HS> {

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
    pub fn match_path<'a>(
        &'a self,
        path: &'a str,
    ) -> Option<(&'a PathHolder<H,SHARED, QS, HS>, Option<Vec<(&'a str, &'a str)>>)> {
        let path = path.trim_matches('/').split("?").next().unwrap();
        let segment_count = path.split('/').count();

        // 1️⃣ static
        if let Some(holder) = self.static_paths.get(path) {
            return Some((holder, None));
        }

        // 2️⃣ dynamic
        if let Some(dynamic_vec) = self.dynamic_paths.get(&segment_count) {
            'outer: for (original_path, holder) in dynamic_vec {
                let mut params = Vec::new();
                let mut incoming_iter = path.split('/');
                let mut original_iter = original_path.split('/');

                while let (Some(opart), Some(ipart)) =
                    (original_iter.next(), incoming_iter.next())
                {
                    if opart.starts_with('{') && opart.ends_with('}') {
                        let name = &opart[1..opart.len() - 1];
                        params.push((name, ipart));
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

        None
    }
}



#[cfg(not(feature = "thread_shared_struct"))]
#[derive(Clone)]
pub struct Matcher<H:'static,const QS:usize,const HS:usize> {
    pub(crate) static_paths:&'static  HashMap<String,PathHolder<H,QS,HS>>,
    pub(crate) dynamic_paths:&'static  HashMap<usize,DynamicPathVec<H,QS,HS>>,
}



#[cfg(not(feature = "thread_shared_struct"))]
impl <H:'static,const QS:usize,const HS:usize>  Matcher<H,QS,HS> {

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
    ) -> Option<(&'a PathHolder<H, QS, HS>, Vec<(&'a str, &'a str)>)> {
        let path = path.trim_matches('/').split("?").next().unwrap();
        let segment_count = path.split('/').count();

        // 1️⃣ static
        if let Some(holder) = self.static_paths.get(path) {
            return Some((holder, Vec::new()));
        }

        // 2️⃣ dynamic
        if let Some(dynamic_vec) = self.dynamic_paths.get(&segment_count) {
            'outer: for (original_path, holder) in dynamic_vec {
                let mut params = Vec::new();
                let mut incoming_iter = path.split('/');
                let mut original_iter = original_path.split('/');

                while let (Some(opart), Some(ipart)) =
                    (original_iter.next(), incoming_iter.next())
                {
                    if opart.starts_with('{') && opart.ends_with('}') {
                        let name = &opart[1..opart.len() - 1];
                        params.push((name, ipart));
                    } else if opart != ipart {
                        continue 'outer;
                    }
                }

                if original_iter.next().is_none()
                    && incoming_iter.next().is_none()
                {
                    return Some((holder, params));
                }
            }
        }

        None
    }
}

#[cfg(not(feature = "thread_shared_struct"))]
impl <H:'static,const HS:usize,const QS:usize>  MatcherInitializer<'_,H,HS,QS> {

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

                controller.push_all_ancestors_middlewares(&mut father_middlewares);
                dynamic_paths.insert(
                    s.count()
                    ,
                    vec![
                        (path,CapsuleHolder{
                            method:method.to_string(),
                            controller,
                            func,
                            father_controllers:controllers_vec,
                            father_middlewares
                        })
                    ]
                );
                continue
            }
            let mut father_middlewares = vec![];

            controller.push_all_ancestors_middlewares(&mut father_middlewares);
            static_paths.insert(path,CapsuleHolder{
                method:method.to_string(),
                controller,
                func,
                father_controllers:controllers_vec,
                father_middlewares
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



    // pub fn match_path(
    //     &mut self,
    //     path: &str,
    // ) -> Option<(PathHolder<H,SHARED,HS,QS>, Vec<(&str, &str)>)> {
    //     let path = path.trim_matches('/');
    //     let segment_count = path.split('/').count();
    //
    //     unsafe {
    //         // 1️⃣ Check static paths
    //         if let Some(holder) = self.static_paths.get(path) {
    //             return Some((*holder, Vec::new()));
    //         }
    //
    //         // 2️⃣ Check dynamic paths
    //         if let Some(dynamic_vec) = self.dynamic_paths.get(&segment_count) {
    //             'outer: for (original_path, holder) in dynamic_vec {
    //                 let mut params = Vec::new();
    //                 let mut incoming_iter = path.split('/');
    //                 let mut original_iter = original_path.split('/');
    //
    //                 while let (Some(opart), Some(ipart)) = (original_iter.next(), incoming_iter.next())
    //                 {
    //                     if opart.starts_with('{') && opart.ends_with('}') {
    //                         // extract param name without {}
    //                         let name = &opart[1..opart.len() - 1];
    //                         params.push((name, ipart));
    //                     } else if opart != ipart {
    //                         continue 'outer; // mismatch
    //                     }
    //                 }
    //
    //                 // Ensure both iterators are fully consumed
    //                 if original_iter.next().is_none() && incoming_iter.next().is_none() {
    //                     return Some((*holder, params));
    //                 }
    //             }
    //         }
    //     }
    //
    //     None
    // }

}


#[cfg(feature = "thread_shared_struct")]
impl <H:'static,SHARED:Clone+'static,const QS:usize,const HS:usize>  MatcherInitializer<'_,H,SHARED,HS,QS> {

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
            let mut controllers_vec:Vec<&'static CapsuleWaterController<H,SHARED,HS,QS>> = vec![];

            let path = format!("{prefix_string}/{path}").replace("//","/").trim_matches(|c| c=='/').to_string();
            if path.contains("{") || path.contains("}") {
                let s = path.split("/");
                let mut father_middlewares = vec![];

                controller.push_all_ancestors_middlewares(&mut father_middlewares);

                dynamic_paths.insert(
                    s.count()
                    ,
                    vec![
                        (path,CapsuleHolder{
                            method:method.to_string(),
                            controller,
                            func,
                            father_middlewares
                        })
                    ]
                );
                continue
            }
            let mut father_middlewares = vec![];

            controller.push_all_ancestors_middlewares(&mut father_middlewares);
            static_paths.insert(path,CapsuleHolder{
                method:method.to_string(),
                controller,
                func,
                father_middlewares
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

    pub fn match_path<'a>(
        &'a self,
        path: &'a str,
    ) -> Option<(&'a PathHolder<H, SHARED, HS,QS>, Vec<(&'a str, &'a str)>)> {
        let path = path.trim_matches('/');
        let segment_count = path.split('/').count();

        // 1️⃣ static
        if let Some(holder) = self.static_paths.get(path) {
            return Some((holder, Vec::new()));
        }

        // 2️⃣ dynamic
        if let Some(dynamic_vec) = self.dynamic_paths.get(&segment_count) {
            'outer: for (original_path, holder) in dynamic_vec {
                let mut params = Vec::new();
                let mut incoming_iter = path.split('/');
                let mut original_iter = original_path.split('/');

                while let (Some(opart), Some(ipart)) =
                    (original_iter.next(), incoming_iter.next())
                {
                    if opart.starts_with('{') && opart.ends_with('}') {
                        let name = &opart[1..opart.len() - 1];
                        params.push((name, ipart));
                    } else if opart != ipart {
                        continue 'outer;
                    }
                }

                if original_iter.next().is_none()
                    && incoming_iter.next().is_none()
                {
                    return Some((holder, params));
                }
            }
        }

        None
    }

    // pub fn match_path(
    //     &mut self,
    //     path: &str,
    // ) -> Option<(PathHolder<H,SHARED,HS,QS>, Vec<(&str, &str)>)> {
    //     let path = path.trim_matches('/');
    //     let segment_count = path.split('/').count();
    //
    //     unsafe {
    //         // 1️⃣ Check static paths
    //         if let Some(holder) = self.static_paths.get(path) {
    //             return Some((*holder, Vec::new()));
    //         }
    //
    //         // 2️⃣ Check dynamic paths
    //         if let Some(dynamic_vec) = self.dynamic_paths.get(&segment_count) {
    //             'outer: for (original_path, holder) in dynamic_vec {
    //                 let mut params = Vec::new();
    //                 let mut incoming_iter = path.split('/');
    //                 let mut original_iter = original_path.split('/');
    //
    //                 while let (Some(opart), Some(ipart)) = (original_iter.next(), incoming_iter.next())
    //                 {
    //                     if opart.starts_with('{') && opart.ends_with('}') {
    //                         // extract param name without {}
    //                         let name = &opart[1..opart.len() - 1];
    //                         params.push((name, ipart));
    //                     } else if opart != ipart {
    //                         continue 'outer; // mismatch
    //                     }
    //                 }
    //
    //                 // Ensure both iterators are fully consumed
    //                 if original_iter.next().is_none() && incoming_iter.next().is_none() {
    //                     return Some((*holder, params));
    //                 }
    //             }
    //         }
    //     }
    //
    //     None
    // }

}

// -------------------- Helper for dynamic vec --------------------





// -------------------- Save a path --------------------


// -------------------- Match a path and extract params --------------------

//
// // -------------------- Tests --------------------
// #[cfg(test)]
// mod tests {
//     use super::*;
//
//
//
//     #[test]
//     fn test_dynamic_path() {
//         init_paths();
//
//         save_path("/about", 1);
//         assert_eq!(match_path("/about").map(|(h, _)| h), Some(1));
//         assert_eq!(match_path("/about/").map(|(h, _)| h), Some(1));
//
//
//         save_path("/users/{id}", 2);
//         save_path("/users/{id}/posts/{post_id}", 3);
//
//         let res1 = match_path("/users/42").unwrap();
//         assert_eq!(res1.0, 2);
//         assert_eq!(res1.1, vec![("id", "42")]);
//
//         let res2 = match_path("/users/42/posts/99").unwrap();
//         assert_eq!(res2.0, 3);
//         assert_eq!(res2.1, vec![("id", "42"), ("post_id", "99")]);
//
//         assert_eq!(match_path("/users/42/posts"), None);
//     }
//
//     #[test]
//     fn test_no_match() {
//         init_paths();
//         save_path("/about", 1);
//         assert_eq!(match_path("/contact"), None);
//     }
// }
