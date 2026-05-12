//! Singleton reference handle for capturing singletons in `q!()` closures.

use std::cell::RefCell;
use std::marker::PhantomData;

use proc_macro2::Span;
use quote::quote;
use stageleft::runtime_support::{FreeVariableWithContextWithProps, QuoteTokens};

use crate::compile::ir::SharedNode;
use crate::location::Location;

/// A lightweight handle to a singleton that can be captured inside `q!()` closures.
///
/// Created via [`Singleton::by_ref()`] or [`Optional::by_ref()`]. When used inside a `q!()`
/// closure, resolves to a `&T` reference to the singleton's value at runtime.
///
/// This type is `Clone` so it can be captured in multiple closures.
pub struct SingletonRef<'a, T, L> {
    pub(crate) node: SharedNode,
    pub(crate) _phantom: PhantomData<(&'a T, L)>,
}

impl<T, L> Clone for SingletonRef<'_, T, L> {
    fn clone(&self) -> Self {
        Self {
            node: SharedNode(self.node.0.clone()),
            _phantom: PhantomData,
        }
    }
}

// Thread-local storage for singleton references captured during `q!()` expansion.
// Maps local ident name -> SharedNode for each singleton captured in the current closure.
thread_local! {
    static SINGLETON_REFS: RefCell<Option<Vec<(syn::Ident, SharedNode)>>> = const { RefCell::new(None) };
}

/// Activate the singleton reference capture context. Must be called before `q!()` expansion
/// that may capture singletons. Returns the captured references when the scope ends.
pub fn with_singleton_capture<R>(f: impl FnOnce() -> R) -> (R, Vec<(syn::Ident, SharedNode)>) {
    SINGLETON_REFS.with(|cell| {
        let prev = cell.borrow_mut().replace(Vec::new());
        assert!(prev.is_none(), "nested singleton capture scopes are not supported");
    });
    let result = f();
    let captured = SINGLETON_REFS.with(|cell| {
        cell.borrow_mut().take().unwrap()
    });
    (result, captured)
}

/// Register a singleton reference capture. Called by `SingletonRef::to_tokens`.
fn register_singleton_ref(ident: syn::Ident, node: SharedNode) {
    SINGLETON_REFS.with(|cell| {
        let mut guard = cell.borrow_mut();
        let refs = guard.as_mut().expect(
            "SingletonRef used inside q!() but no singleton capture scope is active. \
             This is a bug — singleton capture should be set up by the operator that uses q!()."
        );
        refs.push((ident, node));
    });
}

static SINGLETON_REF_COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

impl<'a, T, L> FreeVariableWithContextWithProps<L, ()> for SingletonRef<'a, T, L>
where
    L: Location<'a>,
{
    type O = &'a T;

    fn to_tokens(self, _ctx: &L) -> (QuoteTokens, ()) {
        let id = SINGLETON_REF_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let ident = syn::Ident::new(
            &format!("__hydro_singleton_ref_{}", id),
            Span::call_site(),
        );

        register_singleton_ref(ident.clone(), self.node);

        (
            QuoteTokens {
                prelude: None,
                expr: Some(quote!(#ident)),
            },
            (),
        )
    }
}
