use std::collections::HashMap;
use crate::framework_http::HttpContext;
use crate::structure::{HttpContextRCFunction, WaterCapsuleController, MiddlewareResult};

pub (crate) async fn find_function_from_controllers_and_execute<T:Send>(context:&mut HttpContext<T>,
    controllers:&'static Vec<WaterCapsuleController<T>>
    )->Result<(),String>{
     let mut url_path = context.get_route_path().to_string();
     let  url_method_type = context.get_method();
     clean_path_content(&mut url_path);
     let mut url_parts = vec![];
    fill_vec_with_path_parts(&mut url_parts,&url_path);
     for controller in controllers{
         let result = find_target_controller(&url_parts,controller,url_method_type,None);
         if let Some(result) = result {
             return match result {
                 Ok((function,controller,params_map))=>{
                     let middleware_exam = controller.try_passing_all_middlewares(context).await;
                     if let MiddlewareResult::Pass = middleware_exam {
                         context.path_params_map = params_map;
                         let _res = function(context).await;
                     }
                     return Ok(());
                 }
                 Err(_e) => { Err(_e) }
             }
         }
     }
     Err("can not found a target".to_string())
}



fn clean_path_content(path:&mut String){
    if path.ends_with("/") {
        *path =( &path[..(path.len()-1)]).replace("//","/");
    }
}


fn fill_vec_with_path_parts<'a>(v:&mut Vec<&'a str>,s:&'a str){
    for item in s.split("/") {
        if item.is_empty() {
            continue;
        }
        v.push(item);
    }
}

 fn find_target_controller<T:Send>(url_parts:&[&str],
                                   controller:&'static WaterCapsuleController<T>,
                                   url_method_type:&str,
                                   father_path:Option<&str>,)
   ->Option<Result<(&'static HttpContextRCFunction<T>,&'static WaterCapsuleController<T>,HashMap<String,String>),String>>{
    let prefix_option = controller.prefix.as_ref();
    let mut prefix = String::new();
    if let Some(father_path) = father_path {
        prefix.extend(father_path.chars());
    }
    if let Some(prefix_option) = prefix_option {
        prefix.extend(prefix_option.chars());
    }
     clean_path_content(&mut prefix);
    let  mut current_url_parts = vec![];
    fill_vec_with_path_parts(&mut current_url_parts,&prefix);
    let url_parts_length = url_parts.len();
    let current_url_parts_length = current_url_parts.len();

    if current_url_parts_length <= url_parts_length {

        for (method,function_path,function) in &controller.functions {
            let  mut function_path_parts:Vec<&str> = vec![];
            fill_vec_with_path_parts(&mut function_path_parts,function_path);
            if (function_path_parts.len() + current_url_parts_length ) != url_parts_length {
                continue;
            }
            let mut  founded = true;
            let mut path_injected_parameters:HashMap<String,String> = HashMap::new();
            for (index,slice) in url_parts.iter().enumerate() {
                let function_slice = if index >= current_url_parts_length {
                    function_path_parts[index-(current_url_parts_length)]
                } else {
                    current_url_parts[index]
                };

                if  * slice == function_slice {
                    founded = true;
                    continue;
                }

                if function_slice.contains("{")
                    && function_slice.contains("}")  {
                    founded = true;
                    path_injected_parameters.insert(
                        function_slice[1..(function_slice.len()-1)].to_string()
                        ,(*slice).to_string()
                    );
                    continue;

                }

                founded = false;
            }
            if !founded || url_method_type != method{ continue ; }
            return Some(Ok((function,controller,path_injected_parameters)));
        }
    }

    for child_controller in &controller.children {
        let result = find_target_controller(url_parts,child_controller,
                                            url_method_type,
                                            Some(&prefix));
        if let Some(Err(_)) = result { continue; }
        return  result;
    }

      None
}