use core::panic;
use std::marker::PhantomData;

use internal::CaptureVec;
use proc_macro_crate::FoundCrate;
use proc_macro2::{Span, TokenStream};
use quote::quote;

pub mod internal {
    pub use proc_macro2::{Span, TokenStream};
    pub use quote::quote;
    pub use {proc_macro_crate, proc_macro2, syn};

    pub type CaptureVec = Vec<(String, (Option<TokenStream>, Option<TokenStream>))>;
}

pub use stageleft_macro::{entry, q, quse_fn, top_level_mod};

pub mod runtime_support;
use runtime_support::FreeVariableWithContext;

use crate::runtime_support::get_final_crate_name;

mod type_name;
pub use type_name::{add_private_reexport, quote_type};

#[cfg(windows)]
#[macro_export]
macro_rules! PATH_SEPARATOR {
    () => {
        r"\"
    };
}

#[cfg(not(windows))]
#[macro_export]
macro_rules! PATH_SEPARATOR {
    () => {
        r"/"
    };
}

#[macro_export]
macro_rules! stageleft_crate {
    ($macro_crate:ident) => {
        #[cfg(not(stageleft_macro))]
        #[doc(hidden)]
        pub use $macro_crate as __macro;

        #[cfg(stageleft_macro)]
        include!(concat!(
            env!("OUT_DIR"),
            $crate::PATH_SEPARATOR!(),
            "lib_macro.rs"
        ));

        #[cfg(not(stageleft_macro))]
        #[doc(hidden)]
        #[allow(
            unused,
            ambiguous_glob_reexports,
            unexpected_cfgs,
            clippy::suspicious_else_formatting,
            reason = "generated code"
        )]
        pub mod __staged {
            #[cfg(not(feature = "stageleft_devel"))]
            include!(concat!(
                env!("OUT_DIR"),
                $crate::PATH_SEPARATOR!(),
                "lib_pub.rs"
            ));
        }
    };
}

#[macro_export]
macro_rules! stageleft_no_entry_crate {
    () => {
        #[doc(hidden)]
        #[allow(
            unused,
            ambiguous_glob_reexports,
            unexpected_cfgs,
            clippy::suspicious_else_formatting,
            clippy::type_complexity,
            reason = "generated code"
        )]
        pub mod __staged {
            #[cfg(not(feature = "stageleft_devel"))]
            include!(concat!(
                env!("OUT_DIR"),
                $crate::PATH_SEPARATOR!(),
                "lib_pub.rs"
            ));
        }
    };
}

pub trait QuotedContext {
    fn create() -> Self;
}

pub struct BorrowBounds<'a> {
    _marker: PhantomData<fn(&'a ()) -> &'a ()>,
}

impl QuotedContext for BorrowBounds<'_> {
    fn create() -> Self {
        BorrowBounds {
            _marker: PhantomData,
        }
    }
}

pub trait QuotedWithContext<'a, T, Ctx>: FreeVariableWithContext<Ctx, O = T> {
    fn splice_untyped_ctx(self, ctx: &Ctx) -> syn::Expr
    where
        Self: Sized,
    {
        let (prelude, value) = self.to_tokens(ctx);
        if prelude.is_some() {
            panic!("Quoted value should not have prelude");
        }

        syn::parse2(value.unwrap()).unwrap()
    }

    fn splice_typed_ctx(self, ctx: &Ctx) -> syn::Expr
    where
        Self: Sized,
    {
        let inner_expr = self.splice_untyped_ctx(ctx);
        let stageleft_root = stageleft_root();

        let out_type = quote_type::<T>();

        syn::parse_quote! {
            #stageleft_root::runtime_support::type_hint::<#out_type>(#inner_expr)
        }
    }

    fn splice_fn0_ctx<O>(self, ctx: &Ctx) -> syn::Expr
    where
        Self: Sized,
        T: Fn() -> O,
    {
        let inner_expr = self.splice_untyped_ctx(ctx);
        let stageleft_root = stageleft_root();

        let out_type = quote_type::<O>();

        syn::parse_quote! {
            #stageleft_root::runtime_support::fn0_type_hint::<#out_type>(#inner_expr)
        }
    }

    fn splice_fn1_ctx<I, O>(self, ctx: &Ctx) -> syn::Expr
    where
        Self: Sized,
        T: Fn(I) -> O,
    {
        let inner_expr = self.splice_untyped_ctx(ctx);
        let stageleft_root = stageleft_root();

        let in_type = quote_type::<I>();
        let out_type = quote_type::<O>();

        syn::parse_quote! {
            #stageleft_root::runtime_support::fn1_type_hint::<#in_type, #out_type>(#inner_expr)
        }
    }

    fn splice_fn1_borrow_ctx<I, O>(self, ctx: &Ctx) -> syn::Expr
    where
        Self: Sized,
        T: Fn(&I) -> O,
    {
        let inner_expr = self.splice_untyped_ctx(ctx);
        let stageleft_root = stageleft_root();

        let in_type = quote_type::<I>();
        let out_type = quote_type::<O>();

        syn::parse_quote! {
            #stageleft_root::runtime_support::fn1_borrow_type_hint::<#in_type, #out_type>(#inner_expr)
        }
    }

    fn splice_fn2_borrow_mut_ctx<I1, I2, O>(self, ctx: &Ctx) -> syn::Expr
    where
        Self: Sized,
        T: Fn(&mut I1, I2) -> O,
    {
        let inner_expr = self.splice_untyped_ctx(ctx);
        let stageleft_root = stageleft_root();

        let in1_type = quote_type::<I1>();
        let in2_type = quote_type::<I2>();
        let out_type = quote_type::<O>();

        syn::parse_quote! {
            #stageleft_root::runtime_support::fn2_borrow_mut_type_hint::<#in1_type, #in2_type, #out_type>(#inner_expr)
        }
    }

    fn splice_untyped(self) -> syn::Expr
    where
        Self: Sized,
        Ctx: Default,
    {
        self.splice_untyped_ctx(&Default::default())
    }

    fn splice_typed(self) -> syn::Expr
    where
        Self: Sized,
        Ctx: Default,
    {
        let inner_expr = self.splice_untyped();
        let stageleft_root = stageleft_root();

        let out_type = quote_type::<T>();

        syn::parse_quote! {
            #stageleft_root::runtime_support::type_hint::<#out_type>(#inner_expr)
        }
    }

    fn splice_fn0<O>(self) -> syn::Expr
    where
        Self: Sized,
        Ctx: Default,
        T: Fn() -> O,
    {
        let inner_expr = self.splice_untyped();
        let stageleft_root = stageleft_root();

        let out_type = quote_type::<O>();

        syn::parse_quote! {
            #stageleft_root::runtime_support::fn0_type_hint::<#out_type>(#inner_expr)
        }
    }

    fn splice_fn1<I, O>(self) -> syn::Expr
    where
        Self: Sized,
        Ctx: Default,
        T: Fn(I) -> O,
    {
        let inner_expr = self.splice_untyped();
        let stageleft_root = stageleft_root();

        let in_type = quote_type::<I>();
        let out_type = quote_type::<O>();

        syn::parse_quote! {
            #stageleft_root::runtime_support::fn1_type_hint::<#in_type, #out_type>(#inner_expr)
        }
    }

    fn splice_fn1_borrow<I, O>(self) -> syn::Expr
    where
        Self: Sized,
        Ctx: Default,
        T: Fn(&I) -> O,
    {
        let inner_expr = self.splice_untyped();
        let stageleft_root = stageleft_root();

        let in_type = quote_type::<I>();
        let out_type = quote_type::<O>();

        syn::parse_quote! {
            #stageleft_root::runtime_support::fn1_borrow_type_hint::<#in_type, #out_type>(#inner_expr)
        }
    }

    fn splice_fn2_borrow_mut<I1, I2, O>(self) -> syn::Expr
    where
        Self: Sized,
        Ctx: Default,
        T: Fn(&mut I1, I2) -> O,
    {
        let inner_expr = self.splice_untyped();
        let stageleft_root = stageleft_root();

        let in1_type = quote_type::<I1>();
        let in2_type = quote_type::<I2>();
        let out_type = quote_type::<O>();

        syn::parse_quote! {
            #stageleft_root::runtime_support::fn2_borrow_mut_type_hint::<#in1_type, #in2_type, #out_type>(#inner_expr)
        }
    }
}

pub trait Quoted<'a, T>: QuotedWithContext<'a, T, ()> {}
impl<'a, T, F: QuotedWithContext<'a, T, ()>> Quoted<'a, T> for F {}

fn stageleft_root() -> syn::Ident {
    let stageleft_crate = proc_macro_crate::crate_name("stageleft")
        .unwrap_or_else(|_| panic!("stageleft should be present in `Cargo.toml`"));

    match stageleft_crate {
        FoundCrate::Name(name) => syn::Ident::new(&name, Span::call_site()),
        FoundCrate::Itself => syn::Ident::new("crate", Span::call_site()),
    }
}

pub trait IntoQuotedOnce<'a, T, Ctx>:
    for<'b> FnOnce(
        &'b Ctx,
        &mut String,
        &mut &'static str,
        &mut TokenStream,
        &mut CaptureVec,
        bool,
    ) -> T
    + 'a
    + QuotedWithContext<'a, T, Ctx>
{
    fn boxed(self) -> Box<dyn IntoQuotedOnce<'a, T, Ctx>>
    where
        Self: Sized,
    {
        Box::new(self)
    }
}

impl<
    'a,
    T,
    Ctx,
    F: for<'b> FnOnce(
            &'b Ctx,
            &mut String,
            &mut &'static str,
            &mut TokenStream,
            &mut CaptureVec,
            bool,
        ) -> T
        + 'a,
> QuotedWithContext<'a, T, Ctx> for F
{
}

impl<
    'a,
    T,
    Ctx,
    F: for<'b> FnOnce(
            &'b Ctx,
            &mut String,
            &mut &'static str,
            &mut TokenStream,
            &mut CaptureVec,
            bool,
        ) -> T
        + 'a,
> IntoQuotedOnce<'a, T, Ctx> for F
{
}

impl<
    T,
    Ctx,
    F: for<'b> FnOnce(
        &'b Ctx,
        &mut String,
        &mut &'static str,
        &mut TokenStream,
        &mut CaptureVec,
        bool,
    ) -> T,
> FreeVariableWithContext<Ctx> for F
{
    type O = T;

    fn to_tokens(self, ctx: &Ctx) -> (Option<TokenStream>, Option<TokenStream>) {
        let mut module_path = String::new();
        let mut crate_name = "";
        let mut expr_tokens = TokenStream::new();
        let mut free_variables = Vec::new();
        // this is an uninit value so we can't drop it
        std::mem::forget(self(
            ctx,
            &mut module_path,
            &mut crate_name,
            &mut expr_tokens,
            &mut free_variables,
            false,
        ));

        let instantiated_free_variables = free_variables.iter().flat_map(|(ident, value)| {
            let ident = syn::Ident::new(ident, Span::call_site());
            value.0.iter().map(|prelude| quote!(#prelude)).chain(
                value
                    .1
                    .iter()
                    .map(move |value| quote!(let #ident = #value;)),
            )
        });

        let final_crate_root = get_final_crate_name(crate_name);

        let module_path: syn::Path = syn::parse_str(&module_path).unwrap();
        let module_path_segments = module_path
            .segments
            .iter()
            .skip(1)
            .cloned()
            .collect::<Vec<_>>();
        let module_path = if module_path_segments.is_empty()
            || module_path_segments
                .iter()
                .any(|segment| segment.ident.to_string().starts_with("__doctest"))
        {
            None
        } else {
            Some(syn::Path {
                leading_colon: None,
                segments: syn::punctuated::Punctuated::from_iter(module_path_segments),
            })
        };

        let expr: syn::Expr = syn::parse2(expr_tokens).unwrap();
        let with_env = if let Some(module_path) = module_path {
            quote!({
                use #final_crate_root::__staged::#module_path::*;
                #(#instantiated_free_variables)*
                #expr
            })
        } else {
            quote!({
                use #final_crate_root::__staged::*;
                #(#instantiated_free_variables)*
                #expr
            })
        };

        (None, Some(with_env))
    }
}

pub trait IntoQuotedMut<'a, T, Ctx>:
    FnMut(&Ctx, &mut String, &mut &'static str, &mut TokenStream, &mut CaptureVec, bool) -> T + 'a
{
}

impl<
    'a,
    T,
    Ctx,
    F: FnMut(&Ctx, &mut String, &mut &'static str, &mut TokenStream, &mut CaptureVec, bool) -> T + 'a,
> IntoQuotedMut<'a, T, Ctx> for F
{
}

/// Represents a piece of data that will be passed into the generated code
pub struct RuntimeData<T> {
    ident: &'static str,
    _phantom: PhantomData<T>,
}

impl<'a, T: 'a, Ctx> QuotedWithContext<'a, T, Ctx> for RuntimeData<T> {}

impl<T: Copy> Copy for RuntimeData<T> {}

impl<T: Copy> Clone for RuntimeData<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> RuntimeData<T> {
    pub fn new(ident: &'static str) -> RuntimeData<T> {
        RuntimeData {
            ident,
            _phantom: PhantomData,
        }
    }
}

impl<T, Ctx> FreeVariableWithContext<Ctx> for RuntimeData<T> {
    type O = T;

    fn to_tokens(self, _ctx: &Ctx) -> (Option<TokenStream>, Option<TokenStream>) {
        let ident = syn::Ident::new(self.ident, Span::call_site());
        (None, Some(quote!(#ident)))
    }
}
