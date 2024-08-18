#![allow(unused)]
use proc_macro::{TokenStream, TokenTree};


pub fn vec_to_token(data:Vec<TokenTree>)->TokenStream{
    data.into_iter().collect()
}
fn http_methods_builder(input:TokenStream,parameters:TokenStream)->TokenStream{

    input
}
