//! Reference handles for capturing singletons, optionals, and streams in `q!()` closures.
//!
//! Each handle type wraps a `&RefCell<HydroNode>` and, when captured inside a `q!()` closure,
//! registers itself with the current capture scope. At codegen time, the IR node is lowered
//! to the corresponding DFIR pseudo-operator (`singleton()`, `optional()`, or `handoff()`),
//! and the reference resolves to the appropriate borrow type.

use std::cell::RefCell;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::rc::Rc;

use proc_macro2::Span;
use quote::quote;
use stageleft::runtime_support::{FreeVariableWithContextWithProps, QuoteTokens};

use crate::compile::ir::{HydroNode, SharedNode};
use crate::location::Location;

/// Determines which DFIR pseudo-operator a reference node lowers to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum HandoffRefKind {
    /// `-> singleton()` — exactly one item, `#var` gives `&T`.
    Singleton,
    /// `-> optional()` — zero or one item, `#var` gives `&Option<T>`.
    Optional,
    /// `-> handoff()` — zero or more items, `#var` gives `&Vec<T>`.
    Vec,
}

// Thread-local storage for handoff references captured during `q!()` expansion.
// Stores the HydroNode `(node, is_mut, access_group)` for each reference captured in the current closure,
// along with the FlowState for computing access groups.
// The index determines the ident name via `handoff_ref_ident`.
thread_local! {
    static CAPTURED_REFS: RefCell<Option<Vec<(HydroNode, bool, u32)>>> = const { RefCell::new(None) };
}

/// Returns the canonical ident for a captured ref at the given index within a closure.
pub(crate) fn handoff_ref_ident(index: usize) -> syn::Ident {
    syn::Ident::new(
        &format!("__hydro_singleton_ref_{}", index),
        Span::call_site(),
    )
}

/// Activate the reference capture context. Must be called before `q!()` expansion
/// that may capture handoff references. Returns a `ClosureExpr` bundling the expression with any
/// captured references.
pub fn with_ref_capture(
    f: impl FnOnce() -> crate::compile::ir::DebugExpr,
) -> crate::compile::ir::ClosureExpr {
    CAPTURED_REFS.with(|cell| {
        let prev = cell.borrow_mut().replace(Vec::new());
        assert!(
            prev.is_none(),
            "nested handoff reference capture scopes are not supported"
        );
    });
    let expr = (f)();
    let captured_refs = CAPTURED_REFS.with(|cell| cell.borrow_mut().take().unwrap());
    crate::compile::ir::ClosureExpr::new(expr, captured_refs)
}

/// Shared registration logic: wraps the IR node in `HydroNode::Reference` if needed,
/// pushes it to the capture list, and returns the ident to use in the closure body.
fn register_handoff_ref(
    ir_node: &RefCell<HydroNode>,
    is_mut: bool,
    kind: HandoffRefKind,
    singleton_access_counters: &mut HashMap<*const RefCell<HydroNode>, u32>,
) -> syn::Ident {
    CAPTURED_REFS.with(|cell| {
        let mut guard = cell.borrow_mut();
        let refs = guard.as_mut().expect(
            "HandoffRef used inside q!() but no reference capture scope is active. \
             This is a bug — reference capture should be set up by the operator that uses q!().",
        );

        let index = refs.len();
        let ident = handoff_ref_ident(index);

        let metadata = ir_node.borrow().metadata().clone();

        // Wrap in HydroNode::Reference for materialization + identity tracking.
        // If already a Reference node, reuse it.
        if !matches!(&*ir_node.borrow(), HydroNode::Reference { .. }) {
            let orig = ir_node.replace(HydroNode::Placeholder);
            *ir_node.borrow_mut() = HydroNode::Reference {
                inner: SharedNode(Rc::new(RefCell::new(orig))),
                kind,
                metadata: metadata.clone(),
            };
        }

        let borrow: std::cell::Ref<'_, HydroNode> = ir_node.borrow();
        let HydroNode::Reference { inner, .. } = &*borrow else {
            unreachable!()
        };

        // Compute access group at staging time (code order).
        let ptr = ir_node as *const RefCell<HydroNode>;
        let group = {
            let counter = singleton_access_counters.entry(ptr).or_insert(0);
            if is_mut {
                *counter += 1;
                let g = *counter;
                *counter += 1;
                g
            } else {
                *counter
            }
        };

        refs.push((
            HydroNode::Reference {
                inner: SharedNode(Rc::clone(&inner.0)),
                kind,
                metadata,
            },
            is_mut,
            group,
        ));

        ident
    })
}

/// Macro to define a handoff reference struct with all necessary trait impls.
macro_rules! define_handoff_ref {
    (
        $(
            $(#[$meta:meta])*
            $name:ident, $is_mut:expr, $kind:expr, $output:ty
        )+
    ) => {
        $(
            $(#[$meta])*
            pub struct $name<'a, 'slf, T, L> {
                pub(crate) ir_node: &'slf RefCell<HydroNode>,
                _phantom: PhantomData<(&'a T, L)>,
            }

            impl<'slf, T, L> $name<'_, 'slf, T, L> {
                /// Creates a new reference handle from an IR node cell.
                pub(crate) fn new(ir_node: &'slf RefCell<HydroNode>) -> Self {
                    Self {
                        ir_node,
                        _phantom: PhantomData,
                    }
                }
            }

            impl<T, L> Copy for $name<'_, '_, T, L> {}
            impl<T, L> Clone for $name<'_, '_, T, L> {
                fn clone(&self) -> Self {
                    *self
                }
            }

            impl<'a, 'slf, T: 'a, L> FreeVariableWithContextWithProps<L, ()> for $name<'a, 'slf, T, L>
            where
                L: Location<'a>,
            {
                type O = $output;

                fn to_tokens(self, ctx: &L) -> (QuoteTokens, ()) {
                    let ident = register_handoff_ref(
                        self.ir_node,
                        $is_mut,
                        $kind,
                        &mut ctx.flow_state().borrow_mut().singleton_access_counters,
                    );
                    (
                        QuoteTokens {
                            prelude: None,
                            expr: Some(quote!(#ident)),
                        },
                        (),
                    )
                }
            }
        )+
    };
}

#[stageleft::export(
    SingletonRef,
    SingletonMut,
    OptionalRef,
    OptionalMut,
    StreamRef,
    StreamMut
)]
define_handoff_ref!(
    /// A shared reference handle to a singleton, resolves to `&T` at runtime.
    ///
    /// Created via [`Singleton::by_ref()`](crate::live_collections::Singleton::by_ref).
    SingletonRef, false, HandoffRefKind::Singleton, &'a T

    /// A mutable reference handle to a singleton, resolves to `&mut T` at runtime.
    ///
    /// Created via [`Singleton::by_mut()`](crate::live_collections::Singleton::by_mut).
    SingletonMut, true, HandoffRefKind::Singleton, &'a mut T

    /// A shared reference handle to an optional, resolves to `&Option<T>` at runtime.
    ///
    /// Created via [`Optional::by_ref()`](crate::live_collections::Optional::by_ref).
    OptionalRef, false, HandoffRefKind::Optional, &'a Option<T>

    /// A mutable reference handle to an optional, resolves to `&mut Option<T>` at runtime.
    ///
    /// Created via [`Optional::by_mut()`](crate::live_collections::Optional::by_mut).
    OptionalMut, true, HandoffRefKind::Optional, &'a mut Option<T>

    /// A shared reference handle to a stream's handoff buffer, resolves to `&Vec<T>` at runtime.
    ///
    /// Created via [`Stream::by_ref()`](crate::live_collections::Stream::by_ref).
    StreamRef, false, HandoffRefKind::Vec, &'a Vec<T>

    /// A mutable reference handle to a stream's handoff buffer, resolves to `&mut Vec<T>` at runtime.
    ///
    /// Created via [`Stream::by_mut()`](crate::live_collections::Stream::by_mut).
    StreamMut, true, HandoffRefKind::Vec, &'a mut Vec<T>
);

#[cfg(test)]
#[cfg(feature = "build")]
mod tests {
    use stageleft::q;

    use crate::compile::builder::FlowBuilder;
    use crate::location::Location;

    struct P1 {}

    /// Compile-only test: verifies that `by_ref()` + `q!()` produces valid IR.
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

        my_count.into_stream().for_each(q!(|_| {}));
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

        above.map(q!(|x| x * 2)).for_each(q!(|_| {}));
        below.map(q!(|x| x + 100)).for_each(q!(|_| {}));
        threshold.into_stream().for_each(q!(|_| {}));
        let _built = flow.finalize();
    }

    /// Compile-only test: singleton by_mut.
    #[test]
    fn singleton_by_mut_compiles() {
        let mut flow = FlowBuilder::new();
        let node = flow.process::<P1>();

        let my_count = node
            .source_iter(q!(0..5i32))
            .fold(q!(|| 0i32), q!(|acc: &mut i32, x| *acc += x));
        let count_mut = my_count.by_mut();

        node.source_iter(q!(1..=3i32))
            .map(q!(|x| {
                *count_mut += x;
                x
            }))
            .for_each(q!(|_| {}));

        my_count.into_stream().for_each(q!(|_| {}));
        let _built = flow.finalize();
    }

    /// Compile-only test: optional by_ref.
    #[test]
    fn optional_by_ref_compiles() {
        let mut flow = FlowBuilder::new();
        let node = flow.process::<P1>();

        let my_opt = node.source_iter(q!(0..5i32)).reduce(q!(|a, b| *a += b));
        let opt_ref = my_opt.by_ref();

        node.source_iter(q!(1..=3i32))
            .map(q!(|x| x + opt_ref.unwrap_or(0)))
            .for_each(q!(|_| {}));

        my_opt.into_stream().for_each(q!(|_| {}));
        let _built = flow.finalize();
    }

    /// Compile-only test: stream by_ref.
    #[test]
    fn stream_by_ref_compiles() {
        let mut flow = FlowBuilder::new();
        let node = flow.process::<P1>();

        let my_stream = node.source_iter(q!(0..5i32));
        let stream_ref = my_stream.by_ref();

        node.source_iter(q!(1..=3i32))
            .map(q!(|x| x + stream_ref.len() as i32))
            .for_each(q!(|_| {}));

        my_stream.for_each(q!(|_| {}));
        let _built = flow.finalize();
    }

    /// Compile-only test: singleton by_mut in filter (TotalOrder).
    #[test]
    fn singleton_by_mut_filter() {
        let mut flow = FlowBuilder::new();
        let node = flow.process::<P1>();

        let my_count = node
            .source_iter(q!(0..5i32))
            .fold(q!(|| 0i32), q!(|acc: &mut i32, x| *acc += x));
        let count_mut = my_count.by_mut();

        node.source_iter(q!(1..=3i32))
            .filter(q!(|x| {
                *count_mut += *x;
                *count_mut > 0
            }))
            .for_each(q!(|_| {}));

        my_count.into_stream().for_each(q!(|_| {}));
        let _built = flow.finalize();
    }

    /// Compile-only test: singleton by_mut in flat_map_ordered (TotalOrder).
    #[test]
    fn singleton_by_mut_flat_map() {
        let mut flow = FlowBuilder::new();
        let node = flow.process::<P1>();

        let my_count = node
            .source_iter(q!(0..5i32))
            .fold(q!(|| 0i32), q!(|acc: &mut i32, x| *acc += x));
        let count_mut = my_count.by_mut();

        node.source_iter(q!(1..=3i32))
            .flat_map_ordered(q!(|x| {
                *count_mut += x;
                vec![*count_mut]
            }))
            .for_each(q!(|_| {}));

        my_count.into_stream().for_each(q!(|_| {}));
        let _built = flow.finalize();
    }

    /// Compile-only test: singleton by_mut in filter_map (TotalOrder).
    #[test]
    fn singleton_by_mut_filter_map() {
        let mut flow = FlowBuilder::new();
        let node = flow.process::<P1>();

        let my_count = node
            .source_iter(q!(0..5i32))
            .fold(q!(|| 0i32), q!(|acc: &mut i32, x| *acc += x));
        let count_mut = my_count.by_mut();

        node.source_iter(q!(1..=3i32))
            .filter_map(q!(|x| {
                *count_mut += x;
                Some(*count_mut)
            }))
            .for_each(q!(|_| {}));

        my_count.into_stream().for_each(q!(|_| {}));
        let _built = flow.finalize();
    }

    /// Compile-only test: singleton by_mut in inspect (TotalOrder).
    #[test]
    fn singleton_by_mut_inspect() {
        let mut flow = FlowBuilder::new();
        let node = flow.process::<P1>();

        let my_count = node
            .source_iter(q!(0..5i32))
            .fold(q!(|| 0i32), q!(|acc: &mut i32, x| *acc += x));
        let count_mut = my_count.by_mut();

        node.source_iter(q!(1..=3i32))
            .inspect(q!(|x| {
                *count_mut += *x;
            }))
            .for_each(q!(|_| {}));

        my_count.into_stream().for_each(q!(|_| {}));
        let _built = flow.finalize();
    }

    /// Compile-only test: singleton by_ref in for_each.
    #[test]
    fn singleton_by_ref_for_each() {
        let mut flow = FlowBuilder::new();
        let node = flow.process::<P1>();

        let my_count = node
            .source_iter(q!(0..5i32))
            .fold(q!(|| 0i32), q!(|acc: &mut i32, x| *acc += x));
        let count_ref = my_count.by_ref();

        node.source_iter(q!(1..=3i32))
            .for_each(q!(|x| println!("{}", x + *count_ref)));

        my_count.into_stream().for_each(q!(|_| {}));
        let _built = flow.finalize();
    }

    /// Compile-only test: singleton by_mut in for_each.
    #[test]
    fn singleton_by_mut_for_each() {
        let mut flow = FlowBuilder::new();
        let node = flow.process::<P1>();

        let my_count = node
            .source_iter(q!(0..5i32))
            .fold(q!(|| 0i32), q!(|acc: &mut i32, x| *acc += x));
        let count_mut = my_count.by_mut();

        node.source_iter(q!(1..=3i32)).for_each(q!(|x| {
            *count_mut += x;
        }));

        my_count.into_stream().for_each(q!(|_| {}));
        let _built = flow.finalize();
    }
}
