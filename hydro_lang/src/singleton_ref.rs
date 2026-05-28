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
pub struct SingletonRef<'a, 'slf, T, L, const IS_MUT: bool = false> {
    /// Will be updated to `HydroNode::Singleton` when used, if not already.
    pub(crate) ir_node: &'slf RefCell<HydroNode>,
    _phantom: PhantomData<(&'a T, L)>,
}
/// Alias for [`SingletonRef`] with `IS_MUT = true`.
pub type SingletonMut<'a, 'slf, T, L> = SingletonRef<'a, 'slf, T, L, true>;

impl<'slf, T, L, const IS_MUT: bool> SingletonRef<'_, 'slf, T, L, IS_MUT> {
    /// Creates a `SingletonRef` from a shared node.
    pub(crate) fn new(ir_node: &'slf RefCell<HydroNode>) -> Self {
        Self {
            ir_node,
            _phantom: PhantomData,
        }
    }

    /// Converts this singleton into a shared (non-`mut`) `SingletonRef`.
    pub fn as_ref(&self) -> SingletonRef<'_, 'slf, T, L, false> {
        SingletonRef {
            ir_node: self.ir_node,
            _phantom: PhantomData,
        }
    }

    /// Converts this singleton into a exclusive (`mut`) `SingletonRef`.
    pub fn as_mut(&self) -> SingletonRef<'_, 'slf, T, L, true> {
        SingletonRef {
            ir_node: self.ir_node,
            _phantom: PhantomData,
        }
    }
}

impl<T, L, const IS_MUT: bool> Copy for SingletonRef<'_, '_, T, L, IS_MUT> {}
impl<T, L, const IS_MUT: bool> Clone for SingletonRef<'_, '_, T, L, IS_MUT> {
    fn clone(&self) -> Self {
        *self
    }
}

// Thread-local storage for singleton references captured during `q!()` expansion.
// Maps local ident name -> (SharedNode, is_mut) for each singleton captured in the current closure.
thread_local! {
    static SINGLETON_REFS: RefCell<Option<Vec<(syn::Ident, HydroNode, bool)>>> = const { RefCell::new(None) };
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
    let expr = (f)();
    let singleton_refs = SINGLETON_REFS.with(|cell| cell.borrow_mut().take().unwrap());
    crate::compile::ir::ClosureExpr::new(expr, singleton_refs)
}

static SINGLETON_REF_COUNTER: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

impl<'a, 'slf, T: 'a, L, const IS_MUT: bool> SingletonRef<'a, 'slf, T, L, IS_MUT>
where
    L: Location<'a>,
{
    fn to_tokens_helper(self, _ctx: &L) -> (QuoteTokens, ()) {
        let id = SINGLETON_REF_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let ident = syn::Ident::new(&format!("__hydro_singleton_ref_{}", id), Span::call_site());

        SINGLETON_REFS.with(|cell| {
            let mut guard = cell.borrow_mut();
            let refs = guard.as_mut().expect(
                "SingletonRef used inside q!() but no singleton capture scope is active. \
                 This is a bug — singleton capture should be set up by the operator that uses q!().",
            );

            let metadata = self.ir_node.borrow().metadata().clone();

            // Wrap in HydroNode::Singleton for materialization + identity tracking. If already a Singleton node,
            // reuse it.
            if !matches!(&*self.ir_node.borrow(), HydroNode::Singleton { .. }) {
                let orig = self.ir_node.replace(HydroNode::Placeholder);
                *self.ir_node.borrow_mut() = HydroNode::Singleton {
                    inner: SharedNode(Rc::new(RefCell::new(orig))),
                    metadata: metadata.clone(),
                };
            }

            let borrow: std::cell::Ref<'_, HydroNode> = self.ir_node.borrow();
            let HydroNode::Singleton { inner, .. } = &*borrow else {
                unreachable!()
            };

            refs.push((
                ident.clone(),
                HydroNode::Singleton {
                    inner: SharedNode(Rc::clone(&inner.0)),
                    metadata,
                },
                IS_MUT,
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

impl<'a, 'slf, T: 'a, L> FreeVariableWithContextWithProps<L, ()> for SingletonRef<'a, 'slf, T, L>
where
    L: Location<'a>,
{
    type O = &'a T;

    fn to_tokens(self, ctx: &L) -> (QuoteTokens, ()) {
        self.to_tokens_helper(ctx)
    }
}

impl<'a, 'slf, T: 'a, L> FreeVariableWithContextWithProps<L, ()> for SingletonMut<'a, 'slf, T, L>
where
    L: Location<'a>,
{
    type O = &'a mut T;

    fn to_tokens(self, ctx: &L) -> (QuoteTokens, ()) {
        self.to_tokens_helper(ctx)
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

    /// Compile-only: singleton ref inside partition with downstream operators on both branches.
    ///
    /// This exercises the ident_stack pop logic in the "already built" path of Partition
    /// code generation. When the second branch is processed, singleton ref idents pushed by
    /// transform_children must be popped to keep the stack consistent for downstream ops.
    #[test]
    fn singleton_by_ref_partition_with_downstream_ops() {
        let mut flow = FlowBuilder::new();
        let node = flow.process::<P1>();

        let threshold = node
            .source_iter(q!(0..5i32))
            .fold(q!(|| 0i32), q!(|acc: &mut i32, x| *acc += x));
        let threshold_ref = threshold.by_ref();

        let (above, below) = node
            .source_iter(q!(1..=10i32))
            .partition(q!(|x| *x > *threshold_ref));

        // Downstream operators on both branches — if the pop is missing, these will fail
        above.map(q!(|x| x * 2)).for_each(q!(|_| {}));
        below.map(q!(|x| x + 100)).for_each(q!(|_| {}));
        threshold.into_stream().for_each(q!(|_| {}));
        let _built = flow.finalize();
    }

    #[test]
    fn singleton_by_mut_compiles() {
        let mut flow = FlowBuilder::new();
        let node = flow.process::<P1>();

        let my_count = node
            .source_iter(q!(0..5i32))
            .fold(q!(|| 0i32), q!(|acc: &mut i32, x| *acc += x));
        let count_mut = my_count.by_mut();

        node.source_iter(q!(1..=3i32))
            .map(q!(|x| x + *count_mut))
            .for_each(q!(|_| {}));

        my_count.into_stream().for_each(q!(|_| {}));
        let _built = flow.finalize();
    }
}
