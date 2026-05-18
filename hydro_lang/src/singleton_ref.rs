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
/// Created via [`Singleton::by_ref()`](crate::live_collections::Singleton::by_ref). When used
/// inside a `q!()` closure, resolves to a reference to the singleton's value (`&T`) at runtime.
///
/// This type is `Copy` (required by `q!()` macro internals).
/// TODO(mingwei): <https://github.com/hydro-project/stageleft/issues/73>
pub struct SingletonRef<'a, T, L> {
    pub(crate) node: *const RefCell<HydroNode>,
    _phantom: PhantomData<(&'a (), T, L)>,
}
impl<T, L> SingletonRef<'_, T, L> {
    /// Creates a `SingletonRef` from a shared node.
    ///
    /// Note that this will permanently keep the `Rc` alive, intentionally creating a memory leak
    /// (like [`Box::leak`]).
    pub(crate) fn new(rc_ptr: Rc<RefCell<HydroNode>>) -> Self {
        // SAFETY: `rc_ptr` will now never be dropped, and therefore the count cannot reach zero.
        let node = Rc::into_raw(rc_ptr);
        Self {
            node,
            _phantom: PhantomData,
        }
    }
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
    static SINGLETON_REFS: RefCell<Option<Vec<(syn::Ident, HydroNode)>>> = const { RefCell::new(None) };
}

/// Activate the singleton reference capture context. Must be called before `q!()` expansion
/// that may capture singletons. Returns a `ClosureExpr` bundling the expression with any
/// captured singleton references.
pub fn with_singleton_capture(
    f: impl FnOnce() -> crate::compile::ir::DebugExpr,
) -> crate::compile::ir::ClosureExpr {
    SINGLETON_REFS.with(|cell| {
        let prev = cell.borrow_mut().replace(Vec::new());
        assert!(
            prev.is_none(),
            "nested singleton capture scopes are not supported"
        );
    });
    let expr = f();
    let singleton_refs = SINGLETON_REFS.with(|cell| cell.borrow_mut().take().unwrap());
    crate::compile::ir::ClosureExpr::new(expr, singleton_refs)
}

static SINGLETON_REF_COUNTER: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

impl<'a, T: 'a, L> FreeVariableWithContextWithProps<L, ()> for SingletonRef<'a, T, L>
where
    L: Location<'a>,
{
    type O = &'a T;

    fn to_tokens(self, _ctx: &L) -> (QuoteTokens, ()) {
        let id = SINGLETON_REF_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let ident = syn::Ident::new(&format!("__hydro_singleton_ref_{}", id), Span::call_site());

        SINGLETON_REFS.with(|cell| {
            let mut guard = cell.borrow_mut();
            let refs = guard.as_mut().expect(
                "SingletonRef used inside q!() but no singleton capture scope is active. \
                 This is a bug — singleton capture should be set up by the operator that uses q!().",
            );
            // Reconstruct the Rc from the raw pointer.
            // SAFETY: The `Rc` is leaked by `Rc::into_raw` in `Self::new` and is forever valid.
            // The created `Rc`s `Drop` must not run, that would remove the original refcount.
            let rc = unsafe { Rc::from_raw(self.node) };
            let cloned = rc.clone();
            std::mem::forget(rc); // Don't decrement the original refcount

            let metadata = cloned.borrow().metadata().clone(); // TODO(mingwei): wrong metadata!
            refs.push((
                ident.clone(),
                HydroNode::Singleton {
                    inner: SharedNode(cloned),
                    metadata,
                },
            ));
        });

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

    /// Test with a non-Copy type (Vec) to ensure we're borrowing, not copying.
    #[test]
    fn singleton_by_ref_non_copy() {
        let mut flow = FlowBuilder::new();
        let node = flow.process::<P1>();

        let my_vec = node.source_iter(q!(0..5i32)).fold(
            q!(|| Vec::<i32>::new()),
            q!(|acc: &mut Vec<i32>, x| acc.push(x)),
        );
        let vec_ref = my_vec.by_ref();

        node.source_iter(q!(1..=3i32))
            .map(q!(|x| x + vec_ref.len() as i32))
            .for_each(q!(|_| {}));

        // Also consume the singleton via pipe.
        my_vec.into_stream().for_each(q!(|_| {}));

        let _built = flow.finalize();
    }

    /// Compile-only: singleton ref inside filter closure.
    #[test]
    fn singleton_by_ref_filter() {
        let mut flow = FlowBuilder::new();
        let node = flow.process::<P1>();

        let threshold = node
            .source_iter(q!(0..5i32))
            .fold(q!(|| 0i32), q!(|acc: &mut i32, x| *acc += x));
        let threshold_ref = threshold.by_ref();

        node.source_iter(q!(1..=10i32))
            .filter(q!(|x| *x > *threshold_ref))
            .for_each(q!(|_| {}));

        threshold.into_stream().for_each(q!(|_| {}));
        let _built = flow.finalize();
    }

    /// Compile-only: singleton ref inside flat_map closure.
    #[test]
    fn singleton_by_ref_flat_map() {
        let mut flow = FlowBuilder::new();
        let node = flow.process::<P1>();

        let count = node
            .source_iter(q!(0..3i32))
            .fold(q!(|| 0i32), q!(|acc: &mut i32, _| *acc += 1));
        let count_ref = count.by_ref();

        node.source_iter(q!(1..=2i32))
            .flat_map_ordered(q!(|x| (0..*count_ref).map(move |i| x + i)))
            .for_each(q!(|_| {}));

        count.into_stream().for_each(q!(|_| {}));
        let _built = flow.finalize();
    }

    /// Compile-only: singleton ref inside inspect closure.
    #[test]
    fn singleton_by_ref_inspect() {
        let mut flow = FlowBuilder::new();
        let node = flow.process::<P1>();

        let count = node
            .source_iter(q!(0..5i32))
            .fold(q!(|| 0i32), q!(|acc: &mut i32, _| *acc += 1));
        let count_ref = count.by_ref();

        node.source_iter(q!(1..=3i32))
            .inspect(q!(|x| println!("count={}, x={}", *count_ref, x)))
            .for_each(q!(|_| {}));

        count.into_stream().for_each(q!(|_| {}));
        let _built = flow.finalize();
    }

    /// Compile-only: singleton ref inside partition predicate.
    #[test]
    fn singleton_by_ref_partition() {
        let mut flow = FlowBuilder::new();
        let node = flow.process::<P1>();

        let threshold = node
            .source_iter(q!(0..5i32))
            .fold(q!(|| 0i32), q!(|acc: &mut i32, x| *acc += x));
        let threshold_ref = threshold.by_ref();

        let (above, below) = node
            .source_iter(q!(1..=10i32))
            .partition(q!(|x| *x > *threshold_ref));

        above.for_each(q!(|_| {}));
        below.for_each(q!(|_| {}));
        threshold.into_stream().for_each(q!(|_| {}));
        let _built = flow.finalize();
    }
}
