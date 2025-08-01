use std::marker::PhantomData;

use quote::quote;
use stageleft::QuotedWithContext;
use stageleft::runtime_support::{FreeVariableWithContext, QuoteTokens};

/// Utility for wrapping a quoted expression with a custom transformation, such as explicitly
/// splicing it with a type hint.
pub(crate) struct ManualExpr<T, F> {
    fun: F,
    _phantom: PhantomData<T>,
}

impl<'a, T, F: FnOnce(&Ctx) -> syn::Expr, Ctx> QuotedWithContext<'a, T, Ctx> for ManualExpr<T, F> {}

impl<T, F: Copy> Copy for ManualExpr<T, F> {}

impl<T, F: Clone> Clone for ManualExpr<T, F> {
    fn clone(&self) -> Self {
        Self {
            fun: self.fun.clone(),
            _phantom: PhantomData,
        }
    }
}

impl<T, F> ManualExpr<T, F> {
    pub fn new(fun: F) -> ManualExpr<T, F> {
        ManualExpr {
            fun,
            _phantom: PhantomData,
        }
    }
}

impl<T, F: FnOnce(&Ctx) -> syn::Expr, Ctx> FreeVariableWithContext<Ctx> for ManualExpr<T, F> {
    type O = T;

    fn to_tokens(self, ctx: &Ctx) -> QuoteTokens {
        let expr = (self.fun)(ctx);
        QuoteTokens {
            prelude: None,
            expr: Some(quote!(#expr)),
        }
    }
}
