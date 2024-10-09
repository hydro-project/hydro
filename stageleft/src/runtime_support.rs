use std::borrow::BorrowMut;
use std::marker::PhantomData;
use std::mem::MaybeUninit;

use proc_macro2::{Span, TokenStream};
use quote::quote;

use crate::Quoted;

pub fn get_final_crate_name(crate_name: &str) -> TokenStream {
    let final_crate = proc_macro_crate::crate_name(crate_name)
        .unwrap_or_else(|_| panic!("{crate_name} should be present in `Cargo.toml`"));

    match final_crate {
        proc_macro_crate::FoundCrate::Itself => {
            if std::env::var("CARGO_BIN_NAME").is_ok() {
                let underscored = crate_name.replace('-', "_");
                let underscored_ident = syn::Ident::new(&underscored, Span::call_site());
                quote! { #underscored_ident }
            } else {
                quote! { crate }
            }
        }
        proc_macro_crate::FoundCrate::Name(name) => {
            let ident = syn::Ident::new(&name, Span::call_site());
            quote! { #ident }
        }
    }
}

thread_local! {
    pub(crate) static MACRO_TO_CRATE: std::cell::RefCell<Option<(String, String)>> = const { std::cell::RefCell::new(None) };
}

pub fn set_macro_to_crate(macro_name: &str, crate_name: &str) {
    MACRO_TO_CRATE.with_borrow_mut(|cell| {
        *cell.borrow_mut() = Some((macro_name.to_string(), crate_name.to_string()));
    });
}

pub trait ParseFromLiteral {
    fn parse_from_literal(literal: &syn::Expr) -> Self;
}

macro_rules! impl_parse_from_literal_numeric {
    ($($ty:ty),*) => {
        $(
            impl ParseFromLiteral for $ty {
                fn parse_from_literal(literal: &syn::Expr) -> Self {
                    match literal {
                        syn::Expr::Lit(syn::ExprLit {
                            lit: syn::Lit::Int(lit_int),
                            ..
                        }) => lit_int.base10_parse().unwrap(),
                        _ => panic!("Expected literal"),
                    }
                }
            }
        )*
    };
}

impl ParseFromLiteral for bool {
    fn parse_from_literal(literal: &syn::Expr) -> Self {
        match literal {
            syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Bool(lit_bool),
                ..
            }) => lit_bool.value(),
            _ => panic!("Expected literal"),
        }
    }
}

impl_parse_from_literal_numeric!(i8, i16, i32, i64, i128, isize);
impl_parse_from_literal_numeric!(u8, u16, u32, u64, u128, usize);

pub trait FreeVariable<O> {
    fn to_tokens(self) -> (Option<TokenStream>, Option<TokenStream>)
    where
        Self: Sized;

    fn uninitialized(&self) -> O {
        #[expect(clippy::uninit_assumed_init, reason = "this code should never run")]
        unsafe {
            MaybeUninit::uninit().assume_init()
        }
    }
}

macro_rules! impl_free_variable_from_literal_numeric {
    ($($ty:ty),*) => {
        $(
            impl FreeVariable<$ty> for $ty {
                fn to_tokens(self) -> (Option<TokenStream>, Option<TokenStream>) {
                    (None, Some(quote!(#self)))
                }
            }

            impl<'a> Quoted<'a, $ty> for $ty {}
        )*
    };
}

impl_free_variable_from_literal_numeric!(i8, i16, i32, i64, i128, isize);
impl_free_variable_from_literal_numeric!(u8, u16, u32, u64, u128, usize);

impl FreeVariable<&str> for &str {
    fn to_tokens(self) -> (Option<TokenStream>, Option<TokenStream>) {
        (None, Some(quote!(#self)))
    }
}

pub struct Import<T> {
    module_path: &'static str,
    crate_name: &'static str,
    path: &'static str,
    as_name: &'static str,
    _phantom: PhantomData<T>,
}

impl<T> Copy for Import<T> {}
impl<T> Clone for Import<T> {
    fn clone(&self) -> Self {
        *self
    }
}

pub fn create_import<T>(
    module_path: &'static str,
    crate_name: &'static str,
    path: &'static str,
    as_name: &'static str,
    _unused_type_check: T,
) -> Import<T> {
    Import {
        module_path,
        crate_name,
        path,
        as_name,
        _phantom: PhantomData,
    }
}

impl<T> FreeVariable<T> for Import<T> {
    fn to_tokens(self) -> (Option<TokenStream>, Option<TokenStream>) {
        let final_crate_root = get_final_crate_name(self.crate_name);

        let module_path = syn::parse_str::<syn::Path>(self.module_path).unwrap();
        let parsed = syn::parse_str::<syn::Path>(self.path).unwrap();
        let as_ident = syn::Ident::new(self.as_name, Span::call_site());
        (
            Some(quote!(use #final_crate_root::#module_path::#parsed as #as_ident;)),
            None,
        )
    }
}

pub fn type_hint<O>(v: O) -> O {
    v
}

pub fn fn0_type_hint<'a, O>(f: impl Fn() -> O + 'a) -> impl Fn() -> O + 'a {
    f
}

pub fn fn1_type_hint<'a, I, O>(f: impl Fn(I) -> O + 'a) -> impl Fn(I) -> O + 'a {
    f
}

pub fn fn1_borrow_type_hint<'a, I, O>(f: impl Fn(&I) -> O + 'a) -> impl Fn(&I) -> O + 'a {
    f
}

pub fn fn2_borrow_mut_type_hint<'a, I1, I2, O>(
    f: impl Fn(&mut I1, I2) -> O + 'a,
) -> impl Fn(&mut I1, I2) -> O + 'a {
    f
}
