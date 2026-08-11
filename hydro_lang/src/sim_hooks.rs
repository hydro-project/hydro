//! Handle types for **simulator hooks**: scripting the decisions of unsafe operators.
//!
//! Every unsafe operator (like [`Stream::batch`](crate::live_collections::Stream::batch) or
//! [`Singleton::snapshot`](crate::live_collections::Singleton::snapshot)) takes a
//! [`NonDet`](crate::nondet::NonDet) guard. A guard can optionally carry a **hook handle**,
//! which lets a simulation test take manual control of the non-deterministic decision made
//! by that operator (which elements form the next batch, which version of a piece of state
//! a snapshot reveals, ...). See `hydro_lang::sim::hooks` for the test-side scripting API.
//!
//! Handles are created from the [`FlowBuilder`](crate::compile::builder::FlowBuilder) via
//! [`FlowBuilder::sim_hook`](crate::compile::builder::FlowBuilder::sim_hook) *before* the
//! program under test is constructed, and attached to the operator they control with the
//! `nondet!(... hook = handle)` syntax. Handles are small and `Copy`: the same value is
//! passed into the program during construction and used later inside the test body to
//! script decisions.
//!
//! This module contains only the handle types themselves (plain data), so components can
//! expose hookable signatures (e.g. `nondet_batch: NonDet<Option<BatchHook<u32>>>`, passed
//! directly to the `batch` operator it controls) without pulling
//! in any simulator machinery; binding a hook in a flow that is *deployed* rather than
//! simulated is harmless metadata that non-simulator backends ignore.

use std::marker::PhantomData;

use crate::live_collections::boundedness::{Boundedness, Unbounded};
use crate::live_collections::stream::{ExactlyOnce, Ordering, Retries, TotalOrder};

/// A simulator hook handle (or a set of them) that can be created in one call to
/// [`FlowBuilder::sim_hook`](crate::compile::builder::FlowBuilder::sim_hook).
///
/// Individual handle types implement this trait, and a struct of handles (a component's
/// "testing interface") can implement it by creating every field. Fields are typed
/// `Option<...>` so the struct doubles as a composite hook payload: its [`Default`]
/// ("no hooks") is what a plain `nondet!(...)` guard carries, while `flow.sim_hook()`
/// fills in every handle:
///
/// ```rust,ignore
/// #[derive(Clone, Copy, Default)]
/// pub struct CounterHooks {
///     pub batch: Option<BatchHook<u32>>,
///     pub snapshot: Option<SnapshotHook<u64>>,
/// }
///
/// impl SimHook for CounterHooks {
///     fn create(next_id: &mut dyn FnMut() -> usize) -> Self {
///         CounterHooks {
///             batch: SimHook::create(next_id),
///             snapshot: SimHook::create(next_id),
///         }
///     }
/// }
/// ```
///
/// Such structs nest (a field can itself be a struct of handles), and since handles are
/// `Copy` a test can pass the struct around or destructure it freely.
pub trait SimHook {
    /// Creates every handle in this value, allocating fresh IDs via `next_id`.
    fn create(next_id: &mut dyn FnMut() -> usize) -> Self;
}

impl<H: SimHook> SimHook for Option<H> {
    fn create(next_id: &mut dyn FnMut() -> usize) -> Self {
        Some(H::create(next_id))
    }
}

/// A hook handle controlling a `batch` operator over a stream of `T` elements with ordering
/// `O` and retry guarantee `R` (mirroring the type of the stream being batched).
///
/// A decision for a batch hook says which buffered elements form the next batch released
/// into the tick. See `hydro_lang::sim::hooks` for the decisions offered.
pub struct BatchHook<T, O: Ordering = TotalOrder, R: Retries = ExactlyOnce> {
    pub(crate) id: usize,
    pub(crate) _phantom: PhantomData<fn(T, O, R)>,
}

impl<T, O: Ordering, R: Retries> Clone for BatchHook<T, O, R> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T, O: Ordering, R: Retries> Copy for BatchHook<T, O, R> {}

impl<T, O: Ordering, R: Retries> std::fmt::Debug for BatchHook<T, O, R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BatchHook").field("id", &self.id).finish()
    }
}

impl<T, O: Ordering, R: Retries> SimHook for BatchHook<T, O, R> {
    fn create(next_id: &mut dyn FnMut() -> usize) -> Self {
        BatchHook {
            id: next_id(),
            _phantom: PhantomData,
        }
    }
}

/// A hook handle controlling a `snapshot` operator over a singleton of `T`.
///
/// A decision for a snapshot hook picks which buffered version of the state the next tick
/// execution observes. See `hydro_lang::sim::hooks` for the decisions offered.
pub struct SnapshotHook<T> {
    pub(crate) id: usize,
    pub(crate) _phantom: PhantomData<fn(T)>,
}

impl<T> Clone for SnapshotHook<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for SnapshotHook<T> {}

impl<T> std::fmt::Debug for SnapshotHook<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SnapshotHook")
            .field("id", &self.id)
            .finish()
    }
}

impl<T> SimHook for SnapshotHook<T> {
    fn create(next_id: &mut dyn FnMut() -> usize) -> Self {
        SnapshotHook {
            id: next_id(),
            _phantom: PhantomData,
        }
    }
}

/// A hook handle controlling an `assume_ordering` operator over `T` elements.
///
/// A top-level decision selects the next buffered element to release. An `assume_ordering`
/// inside a tick instead takes one exhaustive ordering of that tick's complete input. See
/// `hydro_lang::sim::hooks` for the decisions offered.
pub struct OrderingHook<T, B: Boundedness = Unbounded> {
    pub(crate) id: usize,
    pub(crate) _phantom: PhantomData<fn(T, B)>,
}

impl<T, B: Boundedness> Clone for OrderingHook<T, B> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T, B: Boundedness> Copy for OrderingHook<T, B> {}

impl<T, B: Boundedness> std::fmt::Debug for OrderingHook<T, B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OrderingHook")
            .field("id", &self.id)
            .finish()
    }
}

impl<T, B: Boundedness> SimHook for OrderingHook<T, B> {
    fn create(next_id: &mut dyn FnMut() -> usize) -> Self {
        OrderingHook {
            id: next_id(),
            _phantom: PhantomData,
        }
    }
}
