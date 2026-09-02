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
use crate::live_collections::stream::{ExactlyOnce, NoOrder, Ordering, Retries, TotalOrder};

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

/// A hook handle controlling an `assume_ordering` operator over `T` elements, with retry
/// guarantee `R` (mirroring the type of the stream whose ordering is assumed).
///
/// For an `ExactlyOnce` stream, a top-level decision selects the next buffered element to
/// release, and an `assume_ordering` inside a tick instead takes one exhaustive ordering
/// of that tick's complete input. For an `AtLeastOnce` stream, ordering additionally
/// decides which *slots* each element's retries occupy, so top-level decisions split into
/// `emit` (release a slot, keep the element for re-emission) and `emit_final` (release
/// the element's last slot), and the in-tick ordering may emit each element into several
/// slots. See `hydro_lang::sim::hooks` for the decisions offered.
pub struct OrderingHook<T, B: Boundedness = Unbounded, R: Retries = ExactlyOnce> {
    pub(crate) id: usize,
    pub(crate) _phantom: PhantomData<fn(T, B, R)>,
}

impl<T, B: Boundedness, R: Retries> Clone for OrderingHook<T, B, R> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T, B: Boundedness, R: Retries> Copy for OrderingHook<T, B, R> {}

impl<T, B: Boundedness, R: Retries> std::fmt::Debug for OrderingHook<T, B, R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OrderingHook")
            .field("id", &self.id)
            .finish()
    }
}

impl<T, B: Boundedness, R: Retries> SimHook for OrderingHook<T, B, R> {
    fn create(next_id: &mut dyn FnMut() -> usize) -> Self {
        OrderingHook {
            id: next_id(),
            _phantom: PhantomData,
        }
    }
}

/// A hook handle controlling an `assume_retries` operator over a stream of `T` elements
/// with ordering `O` (mirroring the type of the stream whose retries are assumed).
///
/// A decision for a retries hook says how many times a buffered element is released —
/// the point where the simulator injects the duplicates the `AtLeastOnce` type says
/// downstream must tolerate. Because every element admits arbitrarily many retries, the
/// decision space is infinite and this non-determinism can never be explored
/// autonomously: an `assume_retries` under simulation **must** be bound to a hook. See
/// `hydro_lang::sim::hooks` for the decisions offered.
pub struct RetriesHook<T, O: Ordering = NoOrder, B: Boundedness = Unbounded> {
    pub(crate) id: usize,
    pub(crate) _phantom: PhantomData<fn(T, O, B)>,
}

impl<T, O: Ordering, B: Boundedness> Clone for RetriesHook<T, O, B> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T, O: Ordering, B: Boundedness> Copy for RetriesHook<T, O, B> {}

impl<T, O: Ordering, B: Boundedness> std::fmt::Debug for RetriesHook<T, O, B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RetriesHook").field("id", &self.id).finish()
    }
}

impl<T, O: Ordering, B: Boundedness> SimHook for RetriesHook<T, O, B> {
    fn create(next_id: &mut dyn FnMut() -> usize) -> Self {
        RetriesHook {
            id: next_id(),
            _phantom: PhantomData,
        }
    }
}

/// A hook handle controlling a `batch` operator over a keyed stream with keys `K`, values
/// `V`, per-key value ordering `O`, and retry guarantee `R` (mirroring the type of the
/// keyed stream being batched).
///
/// A decision for a keyed batch hook says which buffered `(key, value)` entries form the
/// next batch released into the tick. See `hydro_lang::sim::hooks` for the decisions
/// offered.
pub struct KeyedBatchHook<K, V, O: Ordering = TotalOrder, R: Retries = ExactlyOnce> {
    pub(crate) id: usize,
    pub(crate) _phantom: PhantomData<fn(K, V, O, R)>,
}

impl<K, V, O: Ordering, R: Retries> Clone for KeyedBatchHook<K, V, O, R> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<K, V, O: Ordering, R: Retries> Copy for KeyedBatchHook<K, V, O, R> {}

impl<K, V, O: Ordering, R: Retries> std::fmt::Debug for KeyedBatchHook<K, V, O, R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KeyedBatchHook")
            .field("id", &self.id)
            .finish()
    }
}

impl<K, V, O: Ordering, R: Retries> SimHook for KeyedBatchHook<K, V, O, R> {
    fn create(next_id: &mut dyn FnMut() -> usize) -> Self {
        KeyedBatchHook {
            id: next_id(),
            _phantom: PhantomData,
        }
    }
}

/// A hook handle controlling a `snapshot` (or `batch`) operator over a keyed singleton
/// with keys `K` and values `V`.
///
/// A decision for a keyed snapshot hook picks which buffered version of each key's state
/// the next tick execution observes. See `hydro_lang::sim::hooks` for the decisions
/// offered.
pub struct KeyedSnapshotHook<K, V> {
    pub(crate) id: usize,
    pub(crate) _phantom: PhantomData<fn(K, V)>,
}

impl<K, V> Clone for KeyedSnapshotHook<K, V> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<K, V> Copy for KeyedSnapshotHook<K, V> {}

impl<K, V> std::fmt::Debug for KeyedSnapshotHook<K, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KeyedSnapshotHook")
            .field("id", &self.id)
            .finish()
    }
}

impl<K, V> SimHook for KeyedSnapshotHook<K, V> {
    fn create(next_id: &mut dyn FnMut() -> usize) -> Self {
        KeyedSnapshotHook {
            id: next_id(),
            _phantom: PhantomData,
        }
    }
}

/// A hook handle controlling an `assume_ordering` operator over a keyed stream with keys
/// `K` and values `V`.
///
/// A top-level decision selects the next buffered `(key, value)` entry to release. An
/// `assume_ordering` inside a tick instead takes one exhaustive per-key ordering of that
/// tick's complete input. See `hydro_lang::sim::hooks` for the decisions offered.
pub struct KeyedOrderingHook<K, V, B: Boundedness = Unbounded> {
    pub(crate) id: usize,
    pub(crate) _phantom: PhantomData<fn(K, V, B)>,
}

impl<K, V, B: Boundedness> Clone for KeyedOrderingHook<K, V, B> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<K, V, B: Boundedness> Copy for KeyedOrderingHook<K, V, B> {}

impl<K, V, B: Boundedness> std::fmt::Debug for KeyedOrderingHook<K, V, B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KeyedOrderingHook")
            .field("id", &self.id)
            .finish()
    }
}

impl<K, V, B: Boundedness> SimHook for KeyedOrderingHook<K, V, B> {
    fn create(next_id: &mut dyn FnMut() -> usize) -> Self {
        KeyedOrderingHook {
            id: next_id(),
            _phantom: PhantomData,
        }
    }
}

/// A hook handle controlling an `entries_partially_ordered` operator over a keyed stream
/// with keys `K` and values `V`.
///
/// The operator preserves the order of values within each key while interleaving across
/// keys non-deterministically. A top-level decision releases the front entry of one key's
/// buffer; inside a tick, a single decision supplies the complete interleaving. See
/// `hydro_lang::sim::hooks` for the decisions offered.
pub struct PartialOrderingHook<K, V, B: Boundedness = Unbounded> {
    pub(crate) id: usize,
    pub(crate) _phantom: PhantomData<fn(K, V, B)>,
}

impl<K, V, B: Boundedness> Clone for PartialOrderingHook<K, V, B> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<K, V, B: Boundedness> Copy for PartialOrderingHook<K, V, B> {}

impl<K, V, B: Boundedness> std::fmt::Debug for PartialOrderingHook<K, V, B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PartialOrderingHook")
            .field("id", &self.id)
            .finish()
    }
}

impl<K, V, B: Boundedness> SimHook for PartialOrderingHook<K, V, B> {
    fn create(next_id: &mut dyn FnMut() -> usize) -> Self {
        PartialOrderingHook {
            id: next_id(),
            _phantom: PhantomData,
        }
    }
}

/// A hook handle controlling a `merge_ordered` operator over streams of `T` elements.
///
/// The operator preserves the order of each input while interleaving the two inputs
/// non-deterministically. A top-level decision releases the front element of one input's
/// buffer; inside a tick, a single decision supplies the complete interleaving. See
/// `hydro_lang::sim::hooks` for the decisions offered.
pub struct MergeOrderedHook<T, B: Boundedness = Unbounded> {
    pub(crate) id: usize,
    pub(crate) _phantom: PhantomData<fn(T, B)>,
}

impl<T, B: Boundedness> Clone for MergeOrderedHook<T, B> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T, B: Boundedness> Copy for MergeOrderedHook<T, B> {}

impl<T, B: Boundedness> std::fmt::Debug for MergeOrderedHook<T, B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MergeOrderedHook")
            .field("id", &self.id)
            .finish()
    }
}

impl<T, B: Boundedness> SimHook for MergeOrderedHook<T, B> {
    fn create(next_id: &mut dyn FnMut() -> usize) -> Self {
        MergeOrderedHook {
            id: next_id(),
            _phantom: PhantomData,
        }
    }
}

/// A hook handle controlling a `merge_ordered` operator over keyed streams with keys `K`
/// and values `V`.
///
/// The operator preserves each input's order within every key while interleaving the two
/// inputs non-deterministically (cross-key order is unconstrained). A top-level decision
/// releases the front entry of one key's buffer in one input; inside a tick, a single
/// decision supplies the complete interleaving. See `hydro_lang::sim::hooks` for the
/// decisions offered.
pub struct KeyedMergeOrderedHook<K, V, B: Boundedness = Unbounded> {
    pub(crate) id: usize,
    pub(crate) _phantom: PhantomData<fn(K, V, B)>,
}

impl<K, V, B: Boundedness> Clone for KeyedMergeOrderedHook<K, V, B> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<K, V, B: Boundedness> Copy for KeyedMergeOrderedHook<K, V, B> {}

impl<K, V, B: Boundedness> std::fmt::Debug for KeyedMergeOrderedHook<K, V, B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KeyedMergeOrderedHook")
            .field("id", &self.id)
            .finish()
    }
}

impl<K, V, B: Boundedness> SimHook for KeyedMergeOrderedHook<K, V, B> {
    fn create(next_id: &mut dyn FnMut() -> usize) -> Self {
        KeyedMergeOrderedHook {
            id: next_id(),
            _phantom: PhantomData,
        }
    }
}
