//! Singleton reference handle for capturing singletons in `q!()` closures.

use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;

use proc_macro2::Span;
use quote::quote;
use stageleft::runtime_support::{FreeVariableWithContextWithProps, QuoteTokens};

use crate::compile::ir::{HydroNode, SharedNode};
use crate::location::Location;

/// A lightweight handle to a singleton that can be captured inside `q!()` closures.
///
/// Created via [`Singleton::by_ref()`]. When used inside a `q!()`
/// closure, resolves to the singleton's value at runtime.
///
/// This type is `Copy` (required by `q!()` macro internals).
/// Safety: The pointed-to `RefCell<HydroNode>` is kept alive by the `Tee` node
/// in the IR, which outlives all `SingletonRef` handles.
pub struct SingletonRef<'a, T, L> {
    pub(crate) node: *const RefCell<HydroNode>,
    pub(crate) _phantom: PhantomData<(&'a (), T, L)>,
}

impl<T, L> Copy for SingletonRef<'_, T, L> {}
impl<T, L> Clone for SingletonRef<'_, T, L> {
    fn clone(&self) -> Self {
        *self
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
fn register_singleton_ref(ident: syn::Ident, node_ptr: *const RefCell<HydroNode>) {
    SINGLETON_REFS.with(|cell| {
        let mut guard = cell.borrow_mut();
        let refs = guard.as_mut().expect(
            "SingletonRef used inside q!() but no singleton capture scope is active. \
             This is a bug — singleton capture should be set up by the operator that uses q!()."
        );
        // Reconstruct the Rc from the raw pointer (incrementing refcount).
        // Safety: The Rc is kept alive by the Tee node in the IR.
        let rc = unsafe { Rc::from_raw(node_ptr) };
        let cloned = rc.clone();
        std::mem::forget(rc); // Don't decrement the original refcount
        refs.push((ident, SharedNode(cloned)));
    });
}

static SINGLETON_REF_COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

impl<'a, T: 'a, L> FreeVariableWithContextWithProps<L, ()> for SingletonRef<'a, T, L>
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

#[cfg(test)]
#[cfg(feature = "build")]
mod tests {
    use stageleft::q;

    use crate::compile::builder::FlowBuilder;
    use crate::location::Location;

    struct P1 {}

    /// Compile-only test: verifies that `by_ref()` + `q!()` produces valid IR
    /// that can be finalized without panicking.
    #[test]
    fn singleton_by_ref_compiles() {
        let mut flow = FlowBuilder::new();
        let node = flow.process::<P1>();

        let my_count = node
            .source_iter(q!(0..5i32))
            .fold(q!(|| 0i32), q!(|acc: &mut i32, x| *acc += x));
        let count_ref = my_count.by_ref();

        node.source_iter(q!(1..=3i32))
            .map(q!(|x| x + *count_ref))
            .for_each(q!(|_| {}));

        // Also consume the singleton via pipe (tests Tee works correctly).
        my_count.into_stream().for_each(q!(|_| {}));

        // If this doesn't panic, the IR was built successfully with singleton refs.
        let _built = flow.finalize();
    }
}
