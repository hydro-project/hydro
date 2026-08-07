//! Definitions for live collections, which offer the core APIs for writing distributed applications.
//!
//! Traditional programs (like those in Rust) typically manipulate **collections** of data elements,
//! such as those stored in a `Vec` or `HashMap`. These collections are **fixed** in the sense that
//! any transformations applied to them such as `map` are immediately executed on a snapshot of the
//! collection. This means that the output will not be updated when the input collection is modified.
//!
//! In Hydro, programs instead work with **live collections** which are expected to dynamically
//! change over time as new elements are added or removed (in response to API requests, streaming
//! ingestion, etc). Applying a transformation like `map` to a live collection results in another live
//! collection that will dynamically change over time. All network inputs and outputs in Hydro are
//! handled via live collections, so the majority of application logic written with Hydro will involve
//! manipulating live collections.
//!
//! See the [Hydro docs](https://hydro.run/docs/hydro/reference/introduction/live-collections) for more.

pub mod boundedness;

/// The context type for quoted closures (`q!(...)`) passed to operators on live collections.
///
/// This bundles the [`crate::location::Location`] where the collection is materialized
/// with a marker for the [`boundedness::Boundedness`] of the collection the closure operates on.
/// Free variables captured inside such closures can constrain both components. For example,
/// reference handles created via `by_ref()` / `by_mut()` (see [`crate::handoff_ref`]) require
/// that both the location *and* the boundedness of the referenced collection match those of the
/// collection whose operator captures the reference. This prevents, e.g., a reference to a
/// [`boundedness::Bounded`] singleton from being accessed inside a `map` over an
/// [`boundedness::Unbounded`] stream: the singleton is only materialized on the first tick,
/// while the closure keeps running on later ticks, where accessing the reference would crash.
pub struct OperatorContext<L, B>(L, std::marker::PhantomData<B>);

impl<L: Clone, B> OperatorContext<L, B> {
    /// Constructs an [`OperatorContext`] value for splicing quoted closures for an operator on
    /// a collection materialized at `location` with boundedness `B`.
    pub(crate) fn new(location: &L) -> Self {
        OperatorContext(location.clone(), std::marker::PhantomData)
    }
}

/// A context type for quoted snippets (`q!(...)`) from which a [`crate::location::Location`]
/// can be extracted.
///
/// This is implemented both for bare locations (used when splicing quoted *values*, such as
/// the argument to [`crate::location::Location::source_iter`]) and for [`OperatorContext`]
/// (used when splicing quoted *closures* passed to operators on live collections). Free
/// variables that only depend on the location of the splice site (such as
/// [`crate::location::cluster::CLUSTER_SELF_ID`]) are generic over this trait so that they can
/// be captured in both kinds of quoted snippets.
pub trait ContextWithLocation<'a> {
    /// The location type of this context.
    type Location: crate::location::Location<'a>;

    /// Extracts the location of the splice site from this context.
    fn context_location(&self) -> &Self::Location;
}

impl<'a, L: crate::location::Location<'a>> ContextWithLocation<'a> for L {
    type Location = L;

    fn context_location(&self) -> &L {
        self
    }
}

impl<'a, L: crate::location::Location<'a>, B> ContextWithLocation<'a> for OperatorContext<L, B> {
    type Location = L;

    fn context_location(&self) -> &L {
        &self.0
    }
}

pub mod keyed_singleton;
#[doc(inline)]
pub use keyed_singleton::KeyedSingleton;

pub mod keyed_stream;
#[doc(inline)]
pub use keyed_stream::KeyedStream;

pub mod optional;
#[doc(inline)]
pub use optional::Optional;

pub mod singleton;
#[doc(inline)]
pub use singleton::Singleton;

pub mod stream;
#[doc(inline)]
pub use stream::Stream;

pub mod sliced;

#[doc(hidden)]
pub mod batch_atomic;

/// Wraps a freshly-created live collection IR node in an `Rc<RefCell<...>>` and registers it
/// with the flow state, so that the [`crate::compile::builder::FlowBuilder`] can yank the IR
/// of collections that are still alive when the flow is finalized (see hydro-project/hydro#3051).
pub(crate) fn tracked_ir_node(
    flow_state: &crate::compile::builder::FlowState,
    ir_node: crate::compile::ir::HydroNode,
) -> std::rc::Rc<std::cell::RefCell<crate::compile::ir::HydroNode>> {
    let cell = std::rc::Rc::new(std::cell::RefCell::new(ir_node));
    flow_state
        .borrow_mut()
        .live_collection_nodes
        .push(std::rc::Rc::downgrade(&cell));
    cell
}
