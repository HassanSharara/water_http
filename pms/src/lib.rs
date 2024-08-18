#![allow(unused,unused_assignments)]
mod functions_builder;
mod macro_r_builder;

use proc_macro::{Delimiter, Span, TokenStream, TokenTree};
use functions_builder::*;
use quote::quote;


#[proc_macro_attribute]
pub fn main(input:TokenStream,stream:TokenStream)->TokenStream{
    if !input.to_string().is_empty() {
        panic!("there is no parameters need to parse please delete this : {}",input.to_string());
    }
    let mut result = Vec::<TokenTree>::new();
    let q:TokenStream = quote! {
        #[tokio::main]
    }.into();
    result.extend(q.into_iter());
    result.extend(stream.into_iter());
    vec_to_token(result)
}
#[proc_macro_attribute]
pub
fn route(input:TokenStream,parameters:TokenStream)->TokenStream{

    return parameters;
    // let mut parsed_context_name = String::new();
    // let parameters_tree:Vec<TokenTree> = parameters.into_iter().collect();
    // let mut result  = Vec::<TokenTree>::new();
    // let mut is_async_fn = false;
    // let mut first_parenthesis_founded = false;
    // let input_tokens = input.clone().into_iter().collect::<Vec<TokenTree>>();
    // let mut url = String::new();
    // if  input_tokens.len() == 3 {
    //     let mut sep_found = false;
    //     for input_token in input_tokens {
    //         if sep_found  {
    //             let input_token = input_token.to_string();
    //             url = format!("{url}{input_token}");
    //             continue;
    //         }
    //         if input_token.to_string() == "," {
    //             sep_found = true;
    //             continue;}
    //
    //     }
    // }else{
    //     url = input.to_string();
    // }
    // /// loop over given token tree
    // for token_tree in &parameters_tree {
    //     if token_tree.to_string() == "async"{
    //         is_async_fn = true;
    //     }
    //     if token_tree.to_string() == "fn" && ! is_async_fn {
    //         let async_token_tree:TokenStream = quote! {
    //             async fn
    //         }.into();
    //         result.extend(async_token_tree.into_iter().collect::<Vec<TokenTree>>());
    //         continue;
    //     }
    //     if let TokenTree::Group(g) = token_tree {
    //
    //         if g.delimiter() == Delimiter::Parenthesis {
    //             if !first_parenthesis_founded {
    //                 first_parenthesis_founded = true;
    //
    //                 let mut  context_content_founded = false;
    //                 let v = g.stream().into_iter().collect::<Vec<TokenTree>>();
    //                 let first_parenthesis_index_option = &v
    //                     .windows(3).position(
    //                     |w| {
    //                         vec_to_token(w.to_vec()).to_string().trim() == ": & mut"
    //                     }
    //                 );
    //                 if let Some(_p_index) = first_parenthesis_index_option {
    //
    //                     for i in &v[_p_index.to_owned()..] {
    //                         let tree_to_string = i.to_string();
    //                         if tree_to_string.to_uppercase().contains("CONTEXT")
    //                             {
    //                             context_content_founded = true;
    //                             break;
    //                         }
    //                         if i.to_string().trim() == "," {
    //                             break;
    //                         }
    //                     }
    //                     if context_content_founded {
    //                         if _p_index < &1 {
    //                             panic!("encounter error with your function syntax");
    //                         }
    //                         parsed_context_name = (&v[(_p_index.to_owned())-1].to_string()).to_string();
    //                     }
    //                     else {
    //                         panic!("context:&mut HttpContext should be used as parameter in this function \
    //                       \n\
    //                       error : {}",vec_to_token(parameters_tree).to_string());
    //                     }
    //                 }
    //
    //             }
    //         }
    //         else if g.delimiter() == Delimiter::Brace {
    //             let mut function_body_of_braces = Vec::<TokenTree>::new();
    //             if !parsed_context_name.is_empty() {
    //                 /// extracting objects from url
    //                 let mut injected_strings_by_url  = Vec::<TokenTree>::new();
    //                 let mut params_founded = false;
    //                 let  context_name  = proc_macro2::Ident::new(&parsed_context_name,
    //                                                              proc_macro2::Span::call_site());
    //                 for (_index,value) in url.split("/").into_iter().enumerate(){
    //                     let start = value.find("{");
    //                     if let Some(start) = start {
    //                         let end = value.find("}");
    //                         if let Some(end) = end {
    //
    //                             if !params_founded {
    //                                 params_founded = true;
    //                                 let q:TokenStream = quote! {
    //                                      let current_url_path = #context_name.get_route_path();
    //                                     let current_url_path_vec = current_url_path.split("/")
    //                                     .collect::<Vec<&str>>();
    //                                 }.into();
    //                                 function_body_of_braces.extend(q.into_iter());
    //                             }
    //                             let  name  = proc_macro2::Ident::new(&value[start+1..end],
    //                                                                  proc_macro2::Span::call_site());
    //                             let q:TokenStream = quote! {
    //                                 // let #name = #_index;
    //                             }.into();
    //                             function_body_of_braces.extend(q.into_iter());
    //                         }
    //                     }
    //                 }
    //                 function_body_of_braces.extend(g.stream().into_iter().collect::<Vec<TokenTree>>());
    //                 let brace : TokenStream = TokenTree::Group(
    //                     proc_macro::Group::new(
    //                         Delimiter::Brace,
    //                         vec_to_token(function_body_of_braces)
    //                     )
    //                 ).into();
    //                 result.extend(
    //                     brace.into_iter().collect::<Vec<TokenTree>>()
    //                 );
    //                 continue;
    //             }
    //         }
    //
    //     }
    //
    //     result.push(token_tree.to_owned());
    // }
    // vec_to_token(result)
}




