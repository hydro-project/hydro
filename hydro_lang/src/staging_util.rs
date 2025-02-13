use std::marker::PhantomData;

use proc_macro2::{Span, TokenStream};
use quote::quote;

pub type Invariant<'a, D = ()> = PhantomData<(fn(&'a ()) -> &'a (), D)>;

pub fn get_this_crate() -> TokenStream {
    let hydro_lang_crate = proc_macro_crate::crate_name("hydro_lang")
        .expect("hydro_lang should be present in `Cargo.toml`");
    match hydro_lang_crate {
        proc_macro_crate::FoundCrate::Itself => quote! { hydro_lang },
        proc_macro_crate::FoundCrate::Name(name) => {
            let ident = syn::Ident::new(&name, Span::call_site());
            quote! { #ident }
        }
    }
}
