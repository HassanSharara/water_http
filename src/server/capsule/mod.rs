
/// defining all the used and imported macros for building your app structr
pub mod capsule_macros;

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use crate::server::{HttpContext, push_named_route};


pub (crate) type MiddlewareCallback<
    H ,
    const HEADER_SIZE:usize,
    const QUERY_SIZE:usize, >
  = for<'a,'context,>
     fn(&'a mut HttpContext<'context,H, HEADER_SIZE, QUERY_SIZE>)
      ->
      Pin<Box<dyn Future<Output=MiddlewareResult> + Send + 'a>> ;
type WaterSingleFunction<
    H,
    const HEADER_SIZE:usize,
    const QUERY_SIZE:usize, >
  = for<'a,'context,>
  fn (&'a mut HttpContext<'context,H, HEADER_SIZE, QUERY_SIZE>)
      ->
      Pin<Box<dyn Future<Output=()> + Send + 'a>> ;


unsafe impl<
    H:Send + 'static,
    const HEADER_SIZE:usize,
    const QUERY_SIZE:usize,
> Sync for CapsuleWaterController<H,HEADER_SIZE,QUERY_SIZE> {}


/// a struct for holding handlers and another controllers
#[derive(Debug)]
 pub struct CapsuleWaterController<
    H:Send + 'static,
    const HEADER_SIZE:usize,
    const QUERY_SIZE:usize,
>{

    /// father pointer it`s point to father controller if exist
    pub (crate) father:Option<*const CapsuleWaterController<H,HEADER_SIZE,QUERY_SIZE>>,
    /// for providing prefix for each route inside this controller
    pub prefix:Option<&'static str>,
    /// for building middleware to check if user has fixed requirements for this request
    pub middleware:Option<MiddlewareCallback<H,HEADER_SIZE,QUERY_SIZE>>,
    /// functions which holds all requests types and paths and handlers for these both
    functions:Vec<(String,String,WaterSingleFunction<H,HEADER_SIZE,QUERY_SIZE>)>,
    /// to determine if the current controller would affect by parents controllers or not
    pub apply_parents_middlewares:bool,
    /// if this controller has children so these children would be effected by father
    /// controller prefix and also by middleware if children middleware property (apply_parents_middlewares) was true
    children:Vec<CapsuleWaterController<H,HEADER_SIZE,QUERY_SIZE>>
 }

type FFinderResult<H,const HEADER_SIZE:usize,const QUERY_SIZE:usize> =
(&'static CapsuleWaterController<H,HEADER_SIZE,QUERY_SIZE>,&'static WaterSingleFunction<H,HEADER_SIZE,QUERY_SIZE>,
 Option<HashMap<String,String>>
);

 impl <
      H:Send + 'static,
      const HEADER_SIZE:usize,
      const QUERY_SIZE:usize,
      > CapsuleWaterController<H,HEADER_SIZE,QUERY_SIZE> {

     /// for creating new empty capsule controller
     pub fn new()-> CapsuleWaterController<H,HEADER_SIZE,QUERY_SIZE> {
         CapsuleWaterController{
             father:None,
             prefix:None,
             middleware:None,
             functions:vec![],
             apply_parents_middlewares:true,
             children:vec![]
         }
     }

     /// getting father controller if exist
     pub fn get_father_controller<'fat>(&self)->Option<&'fat CapsuleWaterController<H,HEADER_SIZE,QUERY_SIZE>>{
         if let Some(father) = self.father {
             let father = unsafe {father.as_ref()};
             match father {
                 Some(father) => { return Some(father)}
                 _=>{}
             }
         }
         None
     }
     pub (crate) fn push_all_ancestors_middlewares(&'static self,vec:& mut Vec<&'static MiddlewareCallback<H,HEADER_SIZE,QUERY_SIZE>>){
         let mut oc = Some(self);
         loop {
             match oc {
                 None => { break; }
                 Some(controller) => {
                     match controller.middleware.as_ref() {
                         None => {
                             if controller.apply_parents_middlewares {
                                 match controller.get_father_controller() {
                                     None => {break}
                                     Some(con) => {oc = Some(con);continue;}
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

     pub (crate) fn ____insure_binding(&'static mut self){
         let self_pointer :*const Self = self;
         for child in &mut self.children {
             child.father = Some(self_pointer);
             child.____insure_binding();
         }
     }
     pub (crate) fn set_up(&mut self,mut father_prefixes:String){
         if let Some(prefix) = &self.prefix {
             father_prefixes.push_str("/");
             father_prefixes.push_str(prefix);
             father_prefixes = father_prefixes.replace("//","/");
         }
         for (method,path,__) in &mut self.functions {
             if let Some(index) = method.find("_") {
                 let name = &method[index+1..];
                 if name.is_empty() {continue;}
                 push_named_route(name.to_string(),format!("{father_prefixes}/{path}").replace("//","/"));
                 *method = (&method[..index]).to_uppercase();
             }
         }


         for child in &mut self.children {
             child.set_up(father_prefixes.clone());
         }
     }


     /// pushing new function handler
     pub fn push_handler(&mut self,function:(String,String,WaterSingleFunction<H,HEADER_SIZE,QUERY_SIZE>)){
         self.functions.push(function);
     }
     /// pushing new child controller
     pub fn push_controller(&mut self,controller:CapsuleWaterController<H,HEADER_SIZE,QUERY_SIZE> ){
         self.children.push(controller);
     }



     // private functions

     pub (crate) fn get_prefix(&self)->Option<&str>{
         if let Some(prefix) = self.prefix {
                 return Some(Self::shave_path(prefix));
         }
         None
     }

     pub (crate) fn shave_path(mut input:&str)->&str {
          while input.starts_with("/") {
              input = &input[1..]
          }
          while input.ends_with("/") {
              let len = input.len();
              if len == 1  { return  "" }
              input = &input[..len-1]
          }
          input
     }



     pub (crate) const fn all_rest_path_braces()->&'static str{
         "{allRestPath}"
     }
     pub (crate) const fn all_rest_path()->&'static str{
         "allRestPath"
     }
     pub (crate) fn check_if_paths_are_equals(incoming_path:&str,cp:&str)->(bool,Option<HashMap<String,String>>){

         let _s_pattern = Self::all_rest_path_braces();
         if let Some(index) = cp.find(_s_pattern) {
             let first = Self::shave_path(&cp[..index]);
             if incoming_path.starts_with(first) {
                 let mut map:HashMap<String,String> = HashMap::new();
                 map.insert(Self::all_rest_path().to_string(),(
                     &incoming_path[first.len()..]
                     ).to_string());
                 return (true,Some(map))
             }
         }

         let inc_splitter:Vec<&str> = incoming_path.split("/").collect();
         let cp_splitter:Vec<&str> = cp.split("/").collect();
         const  ERR:(bool,Option<HashMap<String,String>>) = (false,None);
         if inc_splitter.len() != cp_splitter.len() { return  ERR }
         let mut map:Option<HashMap<String,String>> = None;
         for (index,part) in cp_splitter.iter().enumerate() {
             let ref inc_part = inc_splitter[index];
             let containing_arcs = part.contains("{") &&  part.contains("}");
             if part != inc_part && !containing_arcs {
                 return ERR
             }

             if containing_arcs  {
                 match map {
                     None => {
                         let mut n_map = HashMap::new();
                         n_map.insert((&part[1..part.len()-1]).to_string(),inc_part.to_string());
                         map = Some(n_map);
                     }
                     Some(ref mut map) => {
                         map.insert((&part[1..part.len()-1]).to_string(),inc_part.to_string());
                     }
                 }
             }
         }
         (true,map)
     }

     pub (crate) fn find_function(&'static self,original_path:&str,original_method:&str)
         ->Option<FFinderResult<H,HEADER_SIZE,QUERY_SIZE>>
     {
         let mut path = Self::shave_path(original_path);
         let prefix = self.get_prefix();
         if let Some(prefix) = prefix {
             if ! path.starts_with(prefix) {
                 return  None
             }
             let prefix_in_length = prefix.len() +1 ;
             if path.len()<= prefix_in_length { return  None}
             path = &path[prefix_in_length..];
         }
         for (method,cp,func) in &self.functions {
             if method != original_method  && method.to_uppercase() != original_method {
                 continue;
             }
             let (result,params) = Self::check_if_paths_are_equals(path,Self::shave_path(cp));
             if !result { continue }
             return Some((self,func,params))
         }
         for  child in &self.children {
             let check_child = child.find_function(path,original_method);
             if check_child.is_some() { return  check_child }
         }
         None
     }


 }



/// # checks if middleware has already responded to client
/// so that the server do not need to respond again
/// or pass the request to the next node
pub enum MiddlewareResult {
 Pass,
 Stop,
}






