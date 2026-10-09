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
//! # Hook scopes
//!
//! Every handle type carries a **scope** parameter naming the kind of root location the
//! hooked operator runs on, mirroring
//! [`Location::SimHookScope`](crate::location::Location::SimHookScope):
//!
//! - [`OnProcess<P>`] (the default): the operator has one instance, scripted directly.
//! - [`OnCluster<C>`]: every cluster member runs its own instance of the operator;
//!   select the one to script with [`.on(member_id)`](BatchHook::on), which yields an
//!   [`OnMember<C>`]-scoped handle.
//!
//! Operators name the scope in their `NonDet` payload type, so scope mismatches fail to
//! compile.
//!
//! This module contains only the handle types themselves (plain data), so components can
//! expose hookable signatures (e.g. `nondet_batch: NonDet<Option<BatchHook<u32>>>`, passed
//! directly to the `batch` operator it controls) without pulling
//! in any simulator machinery; binding a hook in a flow that is *deployed* rather than
//! simulated is harmless metadata that non-simulator backends ignore.

use std::hash::Hash;
use std::marker::PhantomData;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::live_collections::boundedness::{Boundedness, Unbounded};
use crate::live_collections::stream::{
    AtLeastOnce, ExactlyOnce, NoOrder, Ordering, Retries, TotalOrder,
};

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
///
/// Each handle's [`SimHook`] impl carries the trait bounds that the *scripted simulation
/// codegen* for its operator kind requires (serde round-tripping for decisions, equality
/// for value-naming decisions, `Hash + Eq + Clone` keys for keyed buffers). Since handles
/// can only be created through this trait, binding a hook to an operator over unsupported
/// types fails at the `flow.sim_hook()` call — an ordinary compile error in the test crate
/// — instead of surfacing as a rustc failure inside the generated simulation dylib.
pub trait SimHook {
    /// Creates every handle in this value, allocating fresh IDs via `next_id`.
    fn create(next_id: &mut dyn FnMut() -> usize) -> Self;
}

impl<H: SimHook> SimHook for Option<H> {
    fn create(next_id: &mut dyn FnMut() -> usize) -> Self {
        Some(H::create(next_id))
    }
}

/// Hook scope marker: the hooked operator runs on a
/// [`Process`](crate::location::Process) with tag `P` and has one instance, which the
/// handle scripts directly.
///
/// This is the default scope of every handle type. See [the module docs](self#hook-scopes).
pub struct OnProcess<P = ()> {
    _phantom: PhantomData<fn(P)>,
}

/// Hook scope marker: the hooked operator runs on a
/// [`Cluster`](crate::location::Cluster) with tag `C`, so every member runs its own
/// independent instance of the operator.
///
/// Select the instance to script with [`.on(member_id)`](BatchHook::on), which yields an
/// [`OnMember`]-scoped handle. Each member's instance makes its own decisions: a member
/// with buffered input needs its own decision or pause.
pub struct OnCluster<C = ()> {
    _phantom: PhantomData<fn(C)>,
}

/// Hook scope marker: one selected member's instance of an operator running on a
/// [`Cluster`](crate::location::Cluster) with tag `C`, produced by
/// [`.on(member_id)`](BatchHook::on).
///
/// Member-scoped handles script decisions like process-scoped ones, but cannot be
/// created or bound: operators are always bound through the unscoped [`OnCluster`]
/// handle.
pub struct OnMember<C = ()> {
    _phantom: PhantomData<fn(C)>,
}

/// Hook scopes a handle can be created and bound with: [`OnProcess`] and [`OnCluster`].
/// Operator signatures select the scope through
/// [`Location::SimHookScope`](crate::location::Location::SimHookScope).
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a scope that sim hook handles can be created with",
    note = "handles are created with the `OnProcess<P>` or `OnCluster<C>` scope of the operator they will be bound to; `OnMember` handles only arise from `.on(member_id)` at scripting time"
)]
#[sealed::sealed]
pub trait BindableHookScope {}
#[sealed::sealed]
impl<P> BindableHookScope for OnProcess<P> {}
#[sealed::sealed]
impl<C> BindableHookScope for OnCluster<C> {}

/// Hook scopes that name a single instance of the hooked operator and can therefore
/// script decisions and pauses: [`OnProcess`] and [`OnMember`]. An [`OnCluster`]-scoped
/// handle must first select a member with [`.on(member_id)`](BatchHook::on).
#[diagnostic::on_unimplemented(
    message = "a `{Self}`-scoped sim hook handle cannot script decisions",
    note = "a hook bound to an operator running on a cluster has one independent instance per member; select the member to script with `.on(member_id)`"
)]
#[sealed::sealed]
pub trait ScriptableHookScope {}
#[sealed::sealed]
impl<P> ScriptableHookScope for OnProcess<P> {}
#[sealed::sealed]
impl<C> ScriptableHookScope for OnMember<C> {}

/// Generates the `.on(member_id)` member-selection method on [`OnCluster`]-scoped
/// handles, shared by every handle type.
macro_rules! on_member_method {
    ($handle:ident < $($param:ident),* >) => {
        /// Selects one cluster member's instance of the hooked operator, returning an
        /// [`OnMember`]-scoped handle with the full scripting API.
        ///
        /// Each member's instance is scripted independently: `handle.on(0).release(2)`
        /// scripts member 0's next batch and says nothing about the other members, each
        /// of which needs its own decision (or pause) when it holds buffered input. A
        /// member ID outside the cluster's sizing is reported when the scripting call
        /// is awaited.
        pub fn on(&self, member_id: u32) -> $handle<$($param,)* OnMember<C>> {
            $handle {
                id: self.id,
                member: Some(member_id),
                _phantom: PhantomData,
            }
        }
    };
}

/// A hook handle controlling a `batch` operator over a stream of `T` elements with ordering
/// `O` and retry guarantee `R` (mirroring the type of the stream being batched). `S` is the
/// handle's [scope](self#hook-scopes).
///
/// A decision for a batch hook says which buffered elements form the next batch released
/// into the tick. See `hydro_lang::sim::hooks` for the decisions offered.
pub struct BatchHook<T, O: Ordering = TotalOrder, R: Retries = ExactlyOnce, S = OnProcess> {
    pub(crate) id: usize,
    /// The member selected by `.on(member_id)`; `None` for process-scoped handles.
    #[cfg_attr(
        not(feature = "sim"),
        expect(dead_code, reason = "only read by the `sim`-gated scripting API")
    )]
    pub(crate) member: Option<u32>,
    pub(crate) _phantom: PhantomData<fn(T, O, R, S)>,
}

impl<T, O: Ordering, R: Retries, C> BatchHook<T, O, R, OnCluster<C>> {
    on_member_method!(BatchHook<T, O, R>);
}

impl<T, O: Ordering, R: Retries, S> Clone for BatchHook<T, O, R, S> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T, O: Ordering, R: Retries, S> Copy for BatchHook<T, O, R, S> {}

impl<T, O: Ordering, R: Retries, S> std::fmt::Debug for BatchHook<T, O, R, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BatchHook").field("id", &self.id).finish()
    }
}

impl<T, O: Ordering, R: Retries, S: BindableHookScope> SimHook for BatchHook<T, O, R, S>
where
    T: Serialize + DeserializeOwned + PartialEq,
{
    fn create(next_id: &mut dyn FnMut() -> usize) -> Self {
        BatchHook {
            id: next_id(),
            member: None,
            _phantom: PhantomData,
        }
    }
}

/// A hook handle controlling a `snapshot` operator over a singleton of `T`. `S` is the
/// handle's [scope](self#hook-scopes).
///
/// A decision for a snapshot hook picks which buffered version of the state the next tick
/// execution observes. See `hydro_lang::sim::hooks` for the decisions offered.
pub struct SnapshotHook<T, S = OnProcess> {
    pub(crate) id: usize,
    /// The member selected by `.on(member_id)`; `None` for process-scoped handles.
    #[cfg_attr(
        not(feature = "sim"),
        expect(dead_code, reason = "only read by the `sim`-gated scripting API")
    )]
    pub(crate) member: Option<u32>,
    pub(crate) _phantom: PhantomData<fn(T, S)>,
}

impl<T, C> SnapshotHook<T, OnCluster<C>> {
    on_member_method!(SnapshotHook<T>);
}

impl<T, S> Clone for SnapshotHook<T, S> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T, S> Copy for SnapshotHook<T, S> {}

impl<T, S> std::fmt::Debug for SnapshotHook<T, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SnapshotHook")
            .field("id", &self.id)
            .finish()
    }
}

impl<T, S: BindableHookScope> SimHook for SnapshotHook<T, S>
where
    T: Clone + PartialEq + Serialize + DeserializeOwned,
{
    fn create(next_id: &mut dyn FnMut() -> usize) -> Self {
        SnapshotHook {
            id: next_id(),
            member: None,
            _phantom: PhantomData,
        }
    }
}

/// A hook handle controlling an `assume_ordering` operator over `T` elements, with retry
/// guarantee `R` (mirroring the type of the stream whose ordering is assumed). `S` is the
/// handle's [scope](self#hook-scopes).
///
/// For an `ExactlyOnce` stream, a top-level decision selects the next buffered element to
/// release, and an `assume_ordering` inside a tick instead takes one exhaustive ordering
/// of that tick's complete input. For an `AtLeastOnce` stream, ordering additionally
/// decides which *slots* each element's retries occupy, so top-level decisions split into
/// `emit` (release a slot, keep the element for re-emission) and `emit_final` (release
/// the element's last slot), and the in-tick ordering may emit each element into several
/// slots. See `hydro_lang::sim::hooks` for the decisions offered.
pub struct OrderingHook<T, B: Boundedness = Unbounded, R: Retries = ExactlyOnce, S = OnProcess> {
    pub(crate) id: usize,
    /// The member selected by `.on(member_id)`; `None` for process-scoped handles.
    #[cfg_attr(
        not(feature = "sim"),
        expect(dead_code, reason = "only read by the `sim`-gated scripting API")
    )]
    pub(crate) member: Option<u32>,
    pub(crate) _phantom: PhantomData<fn(T, B, R, S)>,
}

impl<T, B: Boundedness, R: Retries, C> OrderingHook<T, B, R, OnCluster<C>> {
    on_member_method!(OrderingHook<T, B, R>);
}

impl<T, B: Boundedness, R: Retries, S> Clone for OrderingHook<T, B, R, S> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T, B: Boundedness, R: Retries, S> Copy for OrderingHook<T, B, R, S> {}

impl<T, B: Boundedness, R: Retries, S> std::fmt::Debug for OrderingHook<T, B, R, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OrderingHook")
            .field("id", &self.id)
            .finish()
    }
}

impl<T, B: Boundedness, S: BindableHookScope> SimHook for OrderingHook<T, B, ExactlyOnce, S>
where
    T: Serialize + DeserializeOwned + PartialEq,
{
    fn create(next_id: &mut dyn FnMut() -> usize) -> Self {
        OrderingHook {
            id: next_id(),
            member: None,
            _phantom: PhantomData,
        }
    }
}

impl<T, B: Boundedness, S: BindableHookScope> SimHook for OrderingHook<T, B, AtLeastOnce, S>
where
    T: Clone + Serialize + DeserializeOwned + PartialEq,
{
    fn create(next_id: &mut dyn FnMut() -> usize) -> Self {
        OrderingHook {
            id: next_id(),
            member: None,
            _phantom: PhantomData,
        }
    }
}

/// A hook handle controlling an `assume_retries` operator over a stream of `T` elements
/// with ordering `O` (mirroring the type of the stream whose retries are assumed). `S`
/// is the handle's [scope](self#hook-scopes).
///
/// A decision for a retries hook says how many times a buffered element is released —
/// the point where the simulator injects the duplicates the `AtLeastOnce` type says
/// downstream must tolerate. Because every element admits arbitrarily many retries, the
/// decision space is infinite and this non-determinism can never be explored
/// autonomously: an `assume_retries` under simulation **must** be bound to a hook. See
/// `hydro_lang::sim::hooks` for the decisions offered.
pub struct RetriesHook<T, O: Ordering = NoOrder, B: Boundedness = Unbounded, S = OnProcess> {
    pub(crate) id: usize,
    /// The selected cluster member (set by `.on(member_id)` on an [`OnCluster`]-scoped
    /// handle); `None` for process-scoped handles. Only read by the simulator's
    /// scripting machinery (`hydro_lang::sim::hooks`), which is feature-gated.
    #[cfg_attr(
        not(feature = "sim"),
        expect(
            dead_code,
            reason = "read only by the `sim`-gated scripting machinery; handles are plain data usable without it"
        )
    )]
    pub(crate) member: Option<u32>,
    pub(crate) _phantom: PhantomData<fn(T, O, B, S)>,
}

impl<T, O: Ordering, B: Boundedness, C> RetriesHook<T, O, B, OnCluster<C>> {
    on_member_method!(RetriesHook<T, O, B>);
}

impl<T, O: Ordering, B: Boundedness, S> Clone for RetriesHook<T, O, B, S> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T, O: Ordering, B: Boundedness, S> Copy for RetriesHook<T, O, B, S> {}

impl<T, O: Ordering, B: Boundedness, S> std::fmt::Debug for RetriesHook<T, O, B, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RetriesHook").field("id", &self.id).finish()
    }
}

impl<T, O: Ordering, B: Boundedness, S: BindableHookScope> SimHook for RetriesHook<T, O, B, S>
where
    T: Clone + Serialize + DeserializeOwned + PartialEq,
{
    fn create(next_id: &mut dyn FnMut() -> usize) -> Self {
        RetriesHook {
            id: next_id(),
            member: None,
            _phantom: PhantomData,
        }
    }
}

/// A hook handle controlling a `batch` operator over a keyed stream with keys `K`, values
/// `V`, per-key value ordering `O`, and retry guarantee `R` (mirroring the type of the
/// keyed stream being batched). `S` is the handle's [scope](self#hook-scopes).
///
/// A decision for a keyed batch hook says which buffered `(key, value)` entries form the
/// next batch released into the tick. See `hydro_lang::sim::hooks` for the decisions
/// offered.
pub struct KeyedBatchHook<K, V, O: Ordering = TotalOrder, R: Retries = ExactlyOnce, S = OnProcess> {
    pub(crate) id: usize,
    /// The member selected by `.on(member_id)`; `None` for process-scoped handles.
    #[cfg_attr(
        not(feature = "sim"),
        expect(dead_code, reason = "only read by the `sim`-gated scripting API")
    )]
    pub(crate) member: Option<u32>,
    pub(crate) _phantom: PhantomData<fn(K, V, O, R, S)>,
}

impl<K, V, O: Ordering, R: Retries, C> KeyedBatchHook<K, V, O, R, OnCluster<C>> {
    on_member_method!(KeyedBatchHook<K, V, O, R>);
}

impl<K, V, O: Ordering, R: Retries, S> Clone for KeyedBatchHook<K, V, O, R, S> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<K, V, O: Ordering, R: Retries, S> Copy for KeyedBatchHook<K, V, O, R, S> {}

impl<K, V, O: Ordering, R: Retries, S> std::fmt::Debug for KeyedBatchHook<K, V, O, R, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KeyedBatchHook")
            .field("id", &self.id)
            .finish()
    }
}

impl<K, V, O: Ordering, R: Retries, S: BindableHookScope> SimHook for KeyedBatchHook<K, V, O, R, S>
where
    K: Hash + Eq + Clone + Serialize + DeserializeOwned,
    V: Serialize + DeserializeOwned + PartialEq,
{
    fn create(next_id: &mut dyn FnMut() -> usize) -> Self {
        KeyedBatchHook {
            id: next_id(),
            member: None,
            _phantom: PhantomData,
        }
    }
}

/// A hook handle controlling a `snapshot` (or `batch`) operator over a keyed singleton
/// with keys `K` and values `V`. `S` is the handle's [scope](self#hook-scopes).
///
/// A decision for a keyed snapshot hook picks which buffered version of each key's state
/// the next tick execution observes. See `hydro_lang::sim::hooks` for the decisions
/// offered.
pub struct KeyedSnapshotHook<K, V, S = OnProcess> {
    pub(crate) id: usize,
    /// The member selected by `.on(member_id)`; `None` for process-scoped handles.
    #[cfg_attr(
        not(feature = "sim"),
        expect(dead_code, reason = "only read by the `sim`-gated scripting API")
    )]
    pub(crate) member: Option<u32>,
    pub(crate) _phantom: PhantomData<fn(K, V, S)>,
}

impl<K, V, C> KeyedSnapshotHook<K, V, OnCluster<C>> {
    on_member_method!(KeyedSnapshotHook<K, V>);
}

impl<K, V, S> Clone for KeyedSnapshotHook<K, V, S> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<K, V, S> Copy for KeyedSnapshotHook<K, V, S> {}

impl<K, V, S> std::fmt::Debug for KeyedSnapshotHook<K, V, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KeyedSnapshotHook")
            .field("id", &self.id)
            .finish()
    }
}

impl<K, V, S: BindableHookScope> SimHook for KeyedSnapshotHook<K, V, S>
where
    K: Hash + Eq + Clone + Serialize + DeserializeOwned,
    V: Clone + PartialEq + Serialize + DeserializeOwned,
{
    fn create(next_id: &mut dyn FnMut() -> usize) -> Self {
        KeyedSnapshotHook {
            id: next_id(),
            member: None,
            _phantom: PhantomData,
        }
    }
}

/// A hook handle controlling an `assume_ordering` operator over a keyed stream with keys
/// `K` and values `V`. `S` is the handle's [scope](self#hook-scopes).
///
/// A top-level decision selects the next buffered `(key, value)` entry to release. An
/// `assume_ordering` inside a tick instead takes one exhaustive per-key ordering of that
/// tick's complete input. See `hydro_lang::sim::hooks` for the decisions offered.
pub struct KeyedOrderingHook<K, V, B: Boundedness = Unbounded, S = OnProcess> {
    pub(crate) id: usize,
    /// The member selected by `.on(member_id)`; `None` for process-scoped handles.
    #[cfg_attr(
        not(feature = "sim"),
        expect(dead_code, reason = "only read by the `sim`-gated scripting API")
    )]
    pub(crate) member: Option<u32>,
    pub(crate) _phantom: PhantomData<fn(K, V, B, S)>,
}

impl<K, V, B: Boundedness, C> KeyedOrderingHook<K, V, B, OnCluster<C>> {
    on_member_method!(KeyedOrderingHook<K, V, B>);
}

impl<K, V, B: Boundedness, S> Clone for KeyedOrderingHook<K, V, B, S> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<K, V, B: Boundedness, S> Copy for KeyedOrderingHook<K, V, B, S> {}

impl<K, V, B: Boundedness, S> std::fmt::Debug for KeyedOrderingHook<K, V, B, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KeyedOrderingHook")
            .field("id", &self.id)
            .finish()
    }
}

impl<K, V, B: Boundedness, S: BindableHookScope> SimHook for KeyedOrderingHook<K, V, B, S>
where
    K: Hash + Eq + Clone + Serialize + DeserializeOwned,
    V: Serialize + DeserializeOwned + PartialEq,
{
    fn create(next_id: &mut dyn FnMut() -> usize) -> Self {
        KeyedOrderingHook {
            id: next_id(),
            member: None,
            _phantom: PhantomData,
        }
    }
}

/// A hook handle controlling an `entries_partially_ordered` operator over a keyed stream
/// with keys `K` and values `V`. `S` is the handle's [scope](self#hook-scopes).
///
/// The operator preserves the order of values within each key while interleaving across
/// keys non-deterministically. A top-level decision releases the front entry of one key's
/// buffer; inside a tick, a single decision supplies the complete interleaving. See
/// `hydro_lang::sim::hooks` for the decisions offered.
pub struct PartialOrderingHook<K, V, B: Boundedness = Unbounded, S = OnProcess> {
    pub(crate) id: usize,
    /// The member selected by `.on(member_id)`; `None` for process-scoped handles.
    #[cfg_attr(
        not(feature = "sim"),
        expect(dead_code, reason = "only read by the `sim`-gated scripting API")
    )]
    pub(crate) member: Option<u32>,
    pub(crate) _phantom: PhantomData<fn(K, V, B, S)>,
}

impl<K, V, B: Boundedness, C> PartialOrderingHook<K, V, B, OnCluster<C>> {
    on_member_method!(PartialOrderingHook<K, V, B>);
}

impl<K, V, B: Boundedness, S> Clone for PartialOrderingHook<K, V, B, S> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<K, V, B: Boundedness, S> Copy for PartialOrderingHook<K, V, B, S> {}

impl<K, V, B: Boundedness, S> std::fmt::Debug for PartialOrderingHook<K, V, B, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PartialOrderingHook")
            .field("id", &self.id)
            .finish()
    }
}

impl<K, V, B: Boundedness, S: BindableHookScope> SimHook for PartialOrderingHook<K, V, B, S>
where
    K: Hash + Eq + Clone + Serialize + DeserializeOwned,
    V: Serialize + DeserializeOwned + PartialEq,
{
    fn create(next_id: &mut dyn FnMut() -> usize) -> Self {
        PartialOrderingHook {
            id: next_id(),
            member: None,
            _phantom: PhantomData,
        }
    }
}

/// A hook handle controlling a `merge_ordered` operator over streams of `T` elements. `S`
/// is the handle's [scope](self#hook-scopes).
///
/// The operator preserves the order of each input while interleaving the two inputs
/// non-deterministically. A top-level decision releases the front element of one input's
/// buffer; inside a tick, a single decision supplies the complete interleaving. See
/// `hydro_lang::sim::hooks` for the decisions offered.
pub struct MergeOrderedHook<T, B: Boundedness = Unbounded, S = OnProcess> {
    pub(crate) id: usize,
    /// The member selected by `.on(member_id)`; `None` for process-scoped handles.
    #[cfg_attr(
        not(feature = "sim"),
        expect(dead_code, reason = "only read by the `sim`-gated scripting API")
    )]
    pub(crate) member: Option<u32>,
    pub(crate) _phantom: PhantomData<fn(T, B, S)>,
}

impl<T, B: Boundedness, C> MergeOrderedHook<T, B, OnCluster<C>> {
    on_member_method!(MergeOrderedHook<T, B>);
}

impl<T, B: Boundedness, S> Clone for MergeOrderedHook<T, B, S> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T, B: Boundedness, S> Copy for MergeOrderedHook<T, B, S> {}

impl<T, B: Boundedness, S> std::fmt::Debug for MergeOrderedHook<T, B, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MergeOrderedHook")
            .field("id", &self.id)
            .finish()
    }
}

impl<T, B: Boundedness, S: BindableHookScope> SimHook for MergeOrderedHook<T, B, S>
where
    T: Serialize + DeserializeOwned + PartialEq,
{
    fn create(next_id: &mut dyn FnMut() -> usize) -> Self {
        MergeOrderedHook {
            id: next_id(),
            member: None,
            _phantom: PhantomData,
        }
    }
}

/// A hook handle controlling a `merge_ordered` operator over keyed streams with keys `K`
/// and values `V`. `S` is the handle's [scope](self#hook-scopes).
///
/// The operator preserves each input's order within every key while interleaving the two
/// inputs non-deterministically (cross-key order is unconstrained). A top-level decision
/// releases the front entry of one key's buffer in one input; inside a tick, a single
/// decision supplies the complete interleaving. See `hydro_lang::sim::hooks` for the
/// decisions offered.
pub struct KeyedMergeOrderedHook<K, V, B: Boundedness = Unbounded, S = OnProcess> {
    pub(crate) id: usize,
    /// The member selected by `.on(member_id)`; `None` for process-scoped handles.
    #[cfg_attr(
        not(feature = "sim"),
        expect(dead_code, reason = "only read by the `sim`-gated scripting API")
    )]
    pub(crate) member: Option<u32>,
    pub(crate) _phantom: PhantomData<fn(K, V, B, S)>,
}

impl<K, V, B: Boundedness, C> KeyedMergeOrderedHook<K, V, B, OnCluster<C>> {
    on_member_method!(KeyedMergeOrderedHook<K, V, B>);
}

impl<K, V, B: Boundedness, S> Clone for KeyedMergeOrderedHook<K, V, B, S> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<K, V, B: Boundedness, S> Copy for KeyedMergeOrderedHook<K, V, B, S> {}

impl<K, V, B: Boundedness, S> std::fmt::Debug for KeyedMergeOrderedHook<K, V, B, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KeyedMergeOrderedHook")
            .field("id", &self.id)
            .finish()
    }
}

impl<K, V, B: Boundedness, S: BindableHookScope> SimHook for KeyedMergeOrderedHook<K, V, B, S>
where
    K: Hash + Eq + Clone + Serialize + DeserializeOwned,
    V: Serialize + DeserializeOwned + PartialEq,
{
    fn create(next_id: &mut dyn FnMut() -> usize) -> Self {
        KeyedMergeOrderedHook {
            id: next_id(),
            member: None,
            _phantom: PhantomData,
        }
    }
}
