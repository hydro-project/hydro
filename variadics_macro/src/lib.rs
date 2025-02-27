#![doc = include_str!("../README.md")]
extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::{LitInt, parse_macro_input};

struct InputLen {
    input: Ident,
    len: LitInt,
}

impl Parse for InputLen {
    fn parse(ts: ParseStream) -> syn::Result<Self> {
        let input = ts.parse()?;
        ts.parse::<syn::Token![,]>()?;
        let len = ts.parse()?;
        Ok(InputLen { input, len })
    }
}

#[proc_macro]
pub fn tuple(ts: TokenStream) -> TokenStream {
    let InputLen { input, len } = parse_macro_input!(ts as InputLen);
    let len = len.base10_parse::<usize>().unwrap();
    let pattern = (0..len)
        .rev()
        .map(|i| format_ident!("x{}", i))
        .fold(quote! { () }, |rest, item| quote! { (#item, #rest) });
    let idents = (0..len).map(|i| format_ident!("x{}", i));
    let tuple = quote! {
        ( #( #idents, )* )
    };

    // Create the assignment statement
    let expanded = quote! {
        {
            let #pattern = #input;
            let retval = #tuple;
            retval
        }
    };

    expanded.into()
}
