//! See [`include_mdtests`] macro documentation.
use std::path::{MAIN_SEPARATOR, Path};

use proc_macro2::Span;
use quote::quote;
use syn::{Ident, LitStr, parse_macro_input};

#[doc = include_str!("../README.md")]
#[proc_macro]
pub fn include_mdtests(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input_string = parse_macro_input!(input as LitStr).value();
    // let input_string = input_string.replace("/", &MAIN_SEPARATOR.to_string());

    let base_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let base_dir = base_dir.replace(MAIN_SEPARATOR, "/");
    // let base_dir = Path::new(&base_dir);

    let pattern = format!(
        "{}/{}",
        base_dir,
        input_string,
    );
    println!("{}", pattern);
    let globbed_files = glob::glob(&pattern)
        .expect("Failed to read glob pattern")
        .map(|entry| entry.expect("Failed to read glob entry"))
        .map(|path| {
            // let path_abs = base_dir.join(path.clone());
            // let path_abs_str = path_abs.to_str().expect("Failed to convert path to string");
            let path_abs_str = format!("{}/{}", base_dir, path.to_string_lossy());
            let file_name_without_extension = path.to_str().expect("Failed to get file stem");
            let lit = LitStr::new(&path_abs_str, Span::call_site());
            let mut ident_string = file_name_without_extension
                .chars()
                .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
                .collect::<String>();
            if ident_string.chars().next().unwrap().is_ascii_digit() {
                // Identifiers cannot start with a digit, prepend an underscore.
                ident_string.insert(0, '_');
            }
            let file_name_ident = Ident::new(&ident_string, Span::call_site());
            quote! {
                #[doc = include_str!(#lit)]
                mod #file_name_ident {}
            }
        });
    let out = quote! {
        #( #globbed_files )*
    };
    out.into()
}
