extern crate proc_macro;

use syn::{parse_macro_input, AttributeArgs, ItemFn};

use proc_macro::TokenStream;

pub mod func;
pub mod loader_html;
pub mod parser_html;
pub mod parser_pysql;
pub mod string_util;
pub mod syntax_tree;

pub fn expr(args: TokenStream, func: TokenStream) -> TokenStream {
    //let args = parse_macro_input!(args as AttributeArgs);
    let target_fn: ItemFn = syn::parse(func).unwrap();
    let stream = func::impl_fn(
        "",
        &target_fn.sig.ident.to_string(),
        &args.to_string(),
        true,
        &[],
    )
    .into();
    stream
}

pub fn rb_html(args: TokenStream, func: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as AttributeArgs);
    let target_fn = syn::parse(func).unwrap();
    let stream = parser_html::impl_fn_html(&target_fn, &args);
    stream
}

/// support py_sql fn convert
pub fn rb_py(args: TokenStream, func: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as AttributeArgs);
    let target_fn = syn::parse(func).unwrap();
    let stream = parser_pysql::impl_fn_py(&target_fn, &args);
    stream
}
