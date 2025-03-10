//! See [`include_mdtests!`] macro documentation.
use proc_macro2::Span;
use quote::quote;
use syn::{Ident, LitStr, parse_macro_input};

#[doc = include_str!("../README.md")]
#[proc_macro]
pub fn include_mdtests(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let current_dir = std::env::current_dir().unwrap();
    let input_glob = parse_macro_input!(input as LitStr);

    let doc_mods = glob::glob(input_glob.value().as_str())
        .expect("Failed to read glob pattern")
        .map(|entry| entry.expect("Failed to read glob entry"))
        .map(|path| {
            let path_lit_str = {
                let path_abs = current_dir.join(path.clone());
                let path_abs_str = path_abs.to_str().expect("Failed to convert path to string");
                LitStr::new(path_abs_str, Span::call_site())
            };

            let mod_ident = {
                let mut ident_string = path
                    .to_string_lossy()
                    .chars()
                    .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
                    .collect::<String>();
                if ident_string
                    .chars()
                    .next()
                    .is_none_or(|c| c.is_ascii_digit())
                {
                    // Identifiers cannot start with a digit, prepend an underscore.
                    ident_string.insert(0, '_');
                }
                Ident::new(&ident_string, Span::call_site())
            };

            quote! {
                #[doc = include_str!(#path_lit_str)]
                mod #mod_ident {}
            }
        });

    let out = quote! {
        #( #doc_mods )*
    };
    out.into()
}
