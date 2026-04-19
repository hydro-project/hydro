//! Consistency markers and the `Consistent` wrapper for typed consistency tracking.
//!
//! The `Consistent<C, S>` wrapper pairs a consistency level `C` with a live
//! collection `S` (typically a `Stream`). The consistency level is a phantom
//! type that tracks the coordination guarantee at compile time:
//!
//! - `Seq`: sequentially consistent — all replicas agree on a total order
//! - `Conv`: convergent — all replicas converge via lattice join
//! - `SelfCon`: self-consistent — future-monotone per-replica
//! - `Incon`: inconsistent — no guarantee
//!
//! Operators on `Consistent` propagate the consistency level through the
//! pipeline. The IDE shows the level on hover without any user annotation.

use std::marker::PhantomData;

// ---------------------------------------------------------------------------
// Consistency markers
// ---------------------------------------------------------------------------

/// Trait for consistency level markers.
pub trait Consistency {}

/// Sequentially consistent: all replicas produce prefixes of the same
/// deterministic sequence, respecting each client's program order.
pub enum Seq {}
impl Consistency for Seq {}

/// Convergent (strong eventual consistency): all replicas converge to the
/// same value via a commutative, idempotent merge (lattice join).
pub enum Conv {}
impl Consistency for Conv {}

/// Self-consistent: future-monotone per-replica, but different replicas
/// or runs may produce different results.
pub enum SelfCon {}
impl Consistency for SelfCon {}

/// Inconsistent: output may contradict earlier observations.
pub enum Incon {}
impl Consistency for Incon {}

// ---------------------------------------------------------------------------
// MinConsistency — compute the weaker of two levels
// ---------------------------------------------------------------------------

/// Computes the weaker of two consistency levels.
/// Strength: Seq > Conv > SelfCon > Incon.
pub trait MinConsistency<Other> {
    type Min: Consistency;
}

// Seq (strongest)
impl MinConsistency<Seq> for Seq { type Min = Seq; }
impl MinConsistency<Conv> for Seq { type Min = Conv; }
impl MinConsistency<SelfCon> for Seq { type Min = SelfCon; }
impl MinConsistency<Incon> for Seq { type Min = Incon; }

// Conv
impl MinConsistency<Seq> for Conv { type Min = Conv; }
impl MinConsistency<Conv> for Conv { type Min = Conv; }
impl MinConsistency<SelfCon> for Conv { type Min = SelfCon; }
impl MinConsistency<Incon> for Conv { type Min = Incon; }

// SelfCon
impl MinConsistency<Seq> for SelfCon { type Min = SelfCon; }
impl MinConsistency<Conv> for SelfCon { type Min = SelfCon; }
impl MinConsistency<SelfCon> for SelfCon { type Min = SelfCon; }
impl MinConsistency<Incon> for SelfCon { type Min = Incon; }

// Incon (weakest)
impl MinConsistency<Seq> for Incon { type Min = Incon; }
impl MinConsistency<Conv> for Incon { type Min = Incon; }
impl MinConsistency<SelfCon> for Incon { type Min = Incon; }
impl MinConsistency<Incon> for Incon { type Min = Incon; }

// ---------------------------------------------------------------------------
// Consistent wrapper
// ---------------------------------------------------------------------------

/// A live collection with a known consistency guarantee.
///
/// `C` is the consistency level (e.g., `Seq`, `Conv`).
/// `S` is the underlying collection (e.g., `Stream<T, L, B, O, R>`).
///
/// The wrapper is zero-cost — `C` is phantom. Access the inner collection
/// via `.inner` or `Deref`.
pub struct Consistent<C: Consistency, S> {
    pub inner: S,
    _phantom: PhantomData<C>,
}

impl<C: Consistency, S> Consistent<C, S> {
    /// Wrap a collection with a consistency label.
    pub fn new(inner: S) -> Self {
        Self { inner, _phantom: PhantomData }
    }

    /// Unwrap, discarding the consistency label.
    pub fn into_inner(self) -> S {
        self.inner
    }

    /// Change the consistency label (unsafe in the logical sense — the caller
    /// asserts the new label is correct).
    pub fn relabel<C2: Consistency>(self) -> Consistent<C2, S> {
        Consistent::new(self.inner)
    }
}

/// Bounded → Unbounded conversion preserves consistency.
impl<'a, C: Consistency, T, L: Location<'a>, O: Ordering, R: Retries>
    From<Consistent<C, Stream<T, L, Bounded, O, R>>>
    for Consistent<C, Stream<T, L, Unbounded, O, R>>
{
    fn from(c: Consistent<C, Stream<T, L, Bounded, O, R>>) -> Self {
        Consistent::new(c.inner.into())
    }
}

// ---------------------------------------------------------------------------
// Stream operator forwarding
// ---------------------------------------------------------------------------

use crate::live_collections::boundedness::{Bounded, Boundedness, IsBounded, Unbounded};
use crate::live_collections::optional::Optional;
use crate::live_collections::singleton::{Singleton, SingletonBound};
use crate::live_collections::stream::{
    AtLeastOnce, ExactlyOnce, IsExactlyOnce, IsOrdered, MinOrder, MinRetries, NoOrder, Ordering,
    Retries, Stream, TotalOrder, WeakerOrderingThan, WeakerRetryThan,
};
use crate::location::tick::Tick;
use crate::location::{Location, NoTick};
use crate::nondet::NonDet;
use crate::properties::{
    AggFuncAlgebra, ApplyMonotoneStream, IsProved, ValidCommutativityFor, ValidIdempotenceFor,
};
use stageleft::{IntoQuotedMut, QuotedWithContext};

// ---- Preserving operators: pass C through unchanged ----

impl<'a, Con: Consistency, T, L: Location<'a>, B: Boundedness, O: Ordering, R: Retries>
    Consistent<Con, Stream<T, L, B, O, R>>
{
    pub fn map<U, F>(self, f: impl IntoQuotedMut<'a, F, L>) -> Consistent<Con, Stream<U, L, B, O, R>>
    where F: Fn(T) -> U + 'a {
        Consistent::new(self.inner.map(f))
    }

    pub fn filter<F>(self, f: impl IntoQuotedMut<'a, F, L>) -> Consistent<Con, Stream<T, L, B, O, R>>
    where F: Fn(&T) -> bool + 'a {
        Consistent::new(self.inner.filter(f))
    }

    pub fn filter_map<U, F>(self, f: impl IntoQuotedMut<'a, F, L>) -> Consistent<Con, Stream<U, L, B, O, R>>
    where F: Fn(T) -> Option<U> + 'a {
        Consistent::new(self.inner.filter_map(f))
    }

    pub fn inspect<F>(self, f: impl IntoQuotedMut<'a, F, L>) -> Consistent<Con, Stream<T, L, B, O, R>>
    where F: Fn(&T) + 'a {
        Consistent::new(self.inner.inspect(f))
    }

    pub fn flat_map_ordered<U, I, F>(self, f: impl IntoQuotedMut<'a, F, L>) -> Consistent<Con, Stream<U, L, B, O, R>>
    where F: Fn(T) -> I + 'a, I: IntoIterator<Item = U> {
        Consistent::new(self.inner.flat_map_ordered(f))
    }

    pub fn flatten_ordered<U>(self) -> Consistent<Con, Stream<U, L, B, O, R>>
    where T: IntoIterator<Item = U> {
        Consistent::new(self.inner.flatten_ordered())
    }

    pub fn enumerate(self) -> Consistent<Con, Stream<(usize, T), L, B, O, R>>
    where O: IsOrdered, R: IsExactlyOnce {
        Consistent::new(self.inner.enumerate())
    }

    pub fn unique(self) -> Consistent<Con, Stream<T, L, B, O, ExactlyOnce>>
    where T: Eq + std::hash::Hash {
        Consistent::new(self.inner.unique())
    }

    pub fn filter_if(self, signal: Singleton<bool, L, Bounded>) -> Consistent<Con, Stream<T, L, B, O, R>> {
        Consistent::new(self.inner.filter_if(signal))
    }

    pub fn filter_if_some<U>(self, signal: Optional<U, L, Bounded>) -> Consistent<Con, Stream<T, L, B, O, R>> {
        Consistent::new(self.inner.filter_if_some(signal))
    }

    pub fn filter_if_none<U>(self, other: Optional<U, L, Bounded>) -> Consistent<Con, Stream<T, L, B, O, R>> {
        Consistent::new(self.inner.filter_if_none(other))
    }

    pub fn filter_not_in<O2: Ordering, B2>(self, other: Stream<T, L, B2, O2, R>) -> Consistent<Con, Stream<T, L, B, O, R>>
    where T: Eq + std::hash::Hash, B2: Boundedness + IsBounded {
        Consistent::new(self.inner.filter_not_in(other))
    }

    pub fn ir_node_named(self, name: &str) -> Consistent<Con, Stream<T, L, B, O, R>> {
        Consistent::new(self.inner.ir_node_named(name))
    }

    pub fn make_bounded(self) -> Consistent<Con, Stream<T, L, Bounded, O, R>>
    where B: IsBounded {
        Consistent::new(self.inner.make_bounded())
    }

    pub fn weaken_ordering<O2: WeakerOrderingThan<O>>(self) -> Consistent<Con, Stream<T, L, B, O2, R>> {
        Consistent::new(self.inner.weaken_ordering())
    }

    pub fn weakest_ordering(self) -> Consistent<Con, Stream<T, L, B, NoOrder, R>> {
        Consistent::new(self.inner.weakest_ordering())
    }

    pub fn weaken_retries<R2: WeakerRetryThan<R>>(self) -> Consistent<Con, Stream<T, L, B, O, R2>> {
        Consistent::new(self.inner.weaken_retries())
    }

    pub fn weakest_retries(self) -> Consistent<Con, Stream<T, L, B, O, AtLeastOnce>> {
        Consistent::new(self.inner.weakest_retries())
    }

    // ---- Scan: preserves C (deterministic stateful transform) ----

    pub fn scan<A, U, I, F>(
        self,
        init: impl IntoQuotedMut<'a, I, L>,
        f: impl IntoQuotedMut<'a, F, L>,
    ) -> Consistent<Con, Stream<U, L, B, TotalOrder, ExactlyOnce>>
    where
        L: NoTick,
        O: IsOrdered,
        R: IsExactlyOnce,
        I: Fn() -> A + 'a,
        F: Fn(&mut A, T) -> Option<U> + 'a,
    {
        Consistent::new(self.inner.scan(init, f))
    }

    // ---- Sort: preserves C on bounded input ----

    pub fn sort(self) -> Consistent<Con, Stream<T, L, Bounded, TotalOrder, R>>
    where T: Ord, B: IsBounded {
        Consistent::new(self.inner.sort())
    }

    // ---- Terminals: C is visible in the type before consumption ----

    pub fn for_each<F: Fn(T) + 'a>(self, f: impl IntoQuotedMut<'a, F, L>)
    where L: NoTick, O: IsOrdered, R: IsExactlyOnce {
        self.inner.for_each(f)
    }

    pub fn dest_sink<S>(self, sink: impl QuotedWithContext<'a, S, L>)
    where S: 'a + futures::Sink<T> + Unpin, L: NoTick, O: IsOrdered, R: IsExactlyOnce {
        self.inner.dest_sink(sink)
    }

    // ---- Aggregations that discharge to Conv (commutative+idempotent) ----

    pub fn fold<A, I, F, Comm, Idemp, M, B2: SingletonBound>(
        self,
        init: impl IntoQuotedMut<'a, I, L>,
        comb: impl IntoQuotedMut<'a, F, L, AggFuncAlgebra<Comm, Idemp, M>>,
    ) -> Consistent<Conv, Singleton<A, L, B2>>
    where
        I: Fn() -> A + 'a,
        F: Fn(&mut A, T),
        Comm: ValidCommutativityFor<O> + IsProved,
        Idemp: ValidIdempotenceFor<R> + IsProved,
        B: ApplyMonotoneStream<M, B2>,
    {
        Consistent::new(self.inner.fold(init, comb))
    }

    pub fn reduce<F, Comm, Idemp>(
        self,
        comb: impl IntoQuotedMut<'a, F, L, AggFuncAlgebra<Comm, Idemp>>,
    ) -> Consistent<Conv, Optional<T, L, B>>
    where
        F: Fn(&mut T, T) + 'a,
        Comm: ValidCommutativityFor<O> + IsProved,
        Idemp: ValidIdempotenceFor<R> + IsProved,
    {
        Consistent::new(self.inner.reduce(comb))
    }

    // ---- Aggregations without proof: erase consistency ----

    pub fn max(self) -> Optional<T, L, B>
    where T: Ord, B: IsBounded {
        self.inner.max()
    }

    pub fn min(self) -> Optional<T, L, B>
    where T: Ord, B: IsBounded {
        self.inner.min()
    }

    pub fn first(self) -> Optional<T, L, B>
    where O: IsOrdered, R: IsExactlyOnce {
        self.inner.first()
    }

    pub fn last(self) -> Optional<T, L, B>
    where O: IsOrdered, R: IsExactlyOnce, B: IsBounded {
        self.inner.last()
    }

    pub fn collect_vec(self) -> Singleton<Vec<T>, L, B>
    where O: IsOrdered, R: IsExactlyOnce {
        self.inner.collect_vec()
    }

    pub fn is_empty(self) -> Singleton<bool, L, Bounded>
    where B: IsBounded {
        self.inner.is_empty()
    }

    pub fn partition<F>(
        self,
        f: impl IntoQuotedMut<'a, F, L>,
    ) -> (Consistent<Con, Stream<T, L, B, O, R>>, Consistent<Con, Stream<T, L, B, O, R>>)
    where F: Fn(&T) -> bool + 'a {
        let (a, b) = self.inner.partition(f);
        (Consistent::new(a), Consistent::new(b))
    }
}

// ---- Downgrading operators: compute MinConsistency ----

impl<'a, C1: Consistency, T, L: Location<'a>, B: Boundedness, O: Ordering, R: Retries>
    Consistent<C1, Stream<T, L, B, O, R>>
{
    pub fn chain<C2: Consistency, O2: Ordering, R2: Retries, B2: Boundedness>(
        self,
        other: Consistent<C2, Stream<T, L, B2, O2, R2>>,
    ) -> Consistent<<C1 as MinConsistency<C2>>::Min, Stream<T, L, B2, <O as MinOrder<O2>>::Min, <R as MinRetries<R2>>::Min>>
    where
        B: IsBounded,
        O: MinOrder<O2>,
        R: MinRetries<R2>,
        C1: MinConsistency<C2>,
    {
        Consistent::new(self.inner.chain(other.inner))
    }

    pub fn cross_product<C2: Consistency, T2, O2: Ordering>(
        self,
        other: Consistent<C2, Stream<T2, L, B, O2, R>>,
    ) -> Consistent<<C1 as MinConsistency<C2>>::Min, Stream<(T, T2), L, B, NoOrder, R>>
    where
        T: Clone,
        T2: Clone,
        C1: MinConsistency<C2>,
    {
        Consistent::new(self.inner.cross_product(other.inner))
    }
}

impl<'a, C1: Consistency, T, L: Location<'a> + NoTick, O: Ordering, R: Retries>
    Consistent<C1, Stream<T, L, Unbounded, O, R>>
{
    pub fn merge_unordered<C2: Consistency, O2: Ordering, R2: Retries>(
        self,
        other: Consistent<C2, Stream<T, L, Unbounded, O2, R2>>,
    ) -> Consistent<<C1 as MinConsistency<C2>>::Min, Stream<T, L, Unbounded, NoOrder, <R as MinRetries<R2>>::Min>>
    where
        R: MinRetries<R2>,
        C1: MinConsistency<C2>,
    {
        Consistent::new(self.inner.merge_unordered(other.inner))
    }
}

// ---- Join: computes MinConsistency on keyed streams ----

impl<'a, C1: Consistency, K, V1, L: Location<'a>, B: Boundedness, O: Ordering, R: Retries>
    Consistent<C1, Stream<(K, V1), L, B, O, R>>
{
    pub fn join<C2: Consistency, V2, O2: Ordering, R2: Retries>(
        self,
        n: Consistent<C2, Stream<(K, V2), L, B, O2, R2>>,
    ) -> Consistent<<C1 as MinConsistency<C2>>::Min, Stream<(K, (V1, V2)), L, B, NoOrder, <R as MinRetries<R2>>::Min>>
    where
        K: Eq + std::hash::Hash,
        R: MinRetries<R2>,
        C1: MinConsistency<C2>,
    {
        Consistent::new(self.inner.join(n.inner))
    }
}

// ---- Additional preserving operators ----

impl<'a, Con: Consistency, T, L: Location<'a>, B: Boundedness, O: Ordering, R: Retries>
    Consistent<Con, Stream<T, L, B, O, R>>
{
    pub fn flat_map_unordered<U, I, F>(self, f: impl IntoQuotedMut<'a, F, L>) -> Consistent<Con, Stream<U, L, B, NoOrder, R>>
    where I: IntoIterator<Item = U>, F: Fn(T) -> I + 'a {
        Consistent::new(self.inner.flat_map_unordered(f))
    }

    pub fn flatten_unordered<U>(self) -> Consistent<Con, Stream<U, L, B, NoOrder, R>>
    where T: IntoIterator<Item = U> {
        Consistent::new(self.inner.flatten_unordered())
    }

    pub fn flat_map_stream_blocking<U, S, F>(self, f: impl IntoQuotedMut<'a, F, L>) -> Consistent<Con, Stream<U, L, B, O, R>>
    where S: futures::Stream<Item = U>, F: Fn(T) -> S + 'a {
        Consistent::new(self.inner.flat_map_stream_blocking(f))
    }

    pub fn flatten_stream_blocking<U>(self) -> Consistent<Con, Stream<U, L, B, O, R>>
    where T: futures::Stream<Item = U> {
        Consistent::new(self.inner.flatten_stream_blocking())
    }

    pub fn resolve_futures_blocking(self) -> Consistent<Con, Stream<T::Output, L, B, NoOrder, R>>
    where T: std::future::Future {
        Consistent::new(self.inner.resolve_futures_blocking())
    }

    pub fn limit(self, n: impl QuotedWithContext<'a, usize, L> + Copy + 'a) -> Consistent<Con, Stream<T, L, B, TotalOrder, ExactlyOnce>>
    where O: IsOrdered, R: IsExactlyOnce {
        Consistent::new(self.inner.limit(n))
    }

    pub fn scan_async_blocking<A, U, I, F, Fut>(
        self,
        init: impl IntoQuotedMut<'a, I, L>,
        f: impl IntoQuotedMut<'a, F, L>,
    ) -> Consistent<Con, Stream<U, L, B, TotalOrder, ExactlyOnce>>
    where
        O: IsOrdered, R: IsExactlyOnce,
        I: Fn() -> A + 'a,
        F: Fn(&mut A, T) -> Fut + 'a,
        Fut: std::future::Future<Output = Option<U>> + 'a,
    {
        Consistent::new(self.inner.scan_async_blocking(init, f))
    }

    pub fn generator<A, U, I, F>(
        self,
        init: impl IntoQuotedMut<'a, I, L> + Copy,
        f: impl IntoQuotedMut<'a, F, L> + Copy,
    ) -> Consistent<Con, Stream<U, L, B, TotalOrder, ExactlyOnce>>
    where
        O: IsOrdered, R: IsExactlyOnce,
        I: Fn() -> A + 'a,
        F: Fn(&mut A, T) -> crate::live_collections::keyed_stream::Generate<U> + 'a,
    {
        Consistent::new(self.inner.generator(init, f))
    }

    pub fn sample_every(
        self,
        interval: impl QuotedWithContext<'a, std::time::Duration, L> + Copy + 'a,
        nondet: NonDet,
    ) -> Consistent<Con, Stream<T, L, Unbounded, O, AtLeastOnce>>
    where L: NoTick + crate::location::tick::NoAtomic {
        Consistent::new(self.inner.sample_every(interval, nondet))
    }

    pub fn atomic(self) -> Consistent<Con, Stream<T, crate::location::tick::Atomic<L>, B, O, R>>
    where L: NoTick {
        Consistent::new(self.inner.atomic())
    }

    pub fn timeout(
        self,
        duration: impl QuotedWithContext<'a, std::time::Duration, Tick<L>> + Copy + 'a,
        nondet: NonDet,
    ) -> Optional<(), L, Unbounded>
    where L: NoTick + crate::location::tick::NoAtomic {
        self.inner.timeout(duration, nondet)
    }
}

// ---- end_atomic: on Atomic<L> streams ----

impl<'a, Con: Consistency, T, L: Location<'a>, B: Boundedness, O: Ordering, R: Retries>
    Consistent<Con, Stream<T, crate::location::tick::Atomic<L>, B, O, R>>
{
    pub fn end_atomic(self) -> Consistent<Con, Stream<T, L, B, O, R>>
    where L: NoTick {
        Consistent::new(self.inner.end_atomic())
    }
}

// ---- Interleave / merge_ordered: two-input, Unbounded ----

impl<'a, C1: Consistency, T, L: Location<'a> + NoTick, O: Ordering, R: Retries>
    Consistent<C1, Stream<T, L, Unbounded, O, R>>
{
    pub fn interleave<C2: Consistency, O2: Ordering, R2: Retries>(
        self,
        other: Consistent<C2, Stream<T, L, Unbounded, O2, R2>>,
    ) -> Consistent<<C1 as MinConsistency<C2>>::Min, Stream<T, L, Unbounded, NoOrder, <R as MinRetries<R2>>::Min>>
    where R: MinRetries<R2>, C1: MinConsistency<C2> {
        Consistent::new(self.inner.interleave(other.inner))
    }
}

impl<'a, C1: Consistency, T, L: Location<'a> + NoTick, R: Retries>
    Consistent<C1, Stream<T, L, Unbounded, TotalOrder, R>>
{
    pub fn merge_ordered<C2: Consistency, R2: Retries>(
        self,
        other: Consistent<C2, Stream<T, L, Unbounded, TotalOrder, R2>>,
        nondet: NonDet,
    ) -> Consistent<<C1 as MinConsistency<C2>>::Min, Stream<T, L, Unbounded, TotalOrder, <R as MinRetries<R2>>::Min>>
    where R: MinRetries<R2>, C1: MinConsistency<C2> {
        Consistent::new(self.inner.merge_ordered(other.inner, nondet))
    }
}

// ---- Batch on NoTick locations ----

impl<'a, Con: Consistency, T, L: Location<'a> + NoTick, O: Ordering, R: Retries>
    Consistent<Con, Stream<T, L, Unbounded, O, R>>
{
    pub fn batch(self, tick: &Tick<L>, nondet: NonDet) -> Consistent<Con, Stream<T, Tick<L>, Bounded, O, R>> {
        Consistent::new(self.inner.batch(tick, nondet))
    }
}

// ---- Cloned on reference streams ----

impl<'a, Con: Consistency, T, L: Location<'a>, B: Boundedness, O: Ordering, R: Retries>
    Consistent<Con, Stream<&T, L, B, O, R>>
{
    pub fn cloned(self) -> Consistent<Con, Stream<T, L, B, O, R>>
    where T: Clone {
        Consistent::new(self.inner.cloned())
    }
}

// ---- Networking: erases consistency (dynamic membership) ----
// Users access networking via .into_inner() and re-label after if needed.
// Future: fixed_broadcast could preserve consistency.

// ---------------------------------------------------------------------------
// Entry point: wrap a source with its consistency label
// ---------------------------------------------------------------------------

impl<T, L, B: Boundedness, O: Ordering, R: Retries> Stream<T, L, B, O, R> {
    /// Wrap this stream with a consistency label.
    /// Typically called on sources: `process.source_iter(q!([1,2,3])).consistent::<Seq>()`
    pub fn consistent<C: Consistency>(self) -> Consistent<C, Self> {
        Consistent::new(self)
    }
}

// ---------------------------------------------------------------------------
// Networking operator forwarding
// ---------------------------------------------------------------------------

use crate::location::cluster::Cluster;
use crate::location::external_process::External;
use crate::networking::NetworkFor;
use serde::de::DeserializeOwned;
use serde::Serialize;

/// Process → Process send: preserves consistency (point-to-point).
impl<'a, Con: Consistency, T, L, B: Boundedness, O: Ordering, R: Retries>
    Consistent<Con, Stream<T, crate::prelude::Process<'a, L>, B, O, R>>
{
    pub fn send_bincode<L2>(
        self,
        other: &crate::prelude::Process<'a, L2>,
    ) -> Consistent<Con, Stream<T, crate::prelude::Process<'a, L2>, Unbounded, O, R>>
    where T: Serialize + DeserializeOwned {
        Consistent::new(self.inner.send_bincode(other))
    }

    pub fn send<L2, N: NetworkFor<T>>(
        self,
        to: &crate::prelude::Process<'a, L2>,
        via: N,
    ) -> Consistent<Con, Stream<T, crate::prelude::Process<'a, L2>, Unbounded, <O as MinOrder<N::OrderingGuarantee>>::Min, R>>
    where T: Serialize + DeserializeOwned, O: MinOrder<N::OrderingGuarantee> {
        Consistent::new(self.inner.send(to, via))
    }

    /// Broadcast from Process to Cluster: erases to Incon (dynamic membership).
    pub fn broadcast_bincode<L2: 'a>(
        self,
        other: &Cluster<'a, L2>,
        nondet_membership: NonDet,
    ) -> Consistent<Incon, Stream<T, Cluster<'a, L2>, Unbounded, O, R>>
    where T: Clone + Serialize + DeserializeOwned {
        Consistent::new(self.inner.broadcast_bincode(other, nondet_membership))
    }

    pub fn broadcast<L2: 'a, N: NetworkFor<T>>(
        self,
        to: &Cluster<'a, L2>,
        via: N,
        nondet_membership: NonDet,
    ) -> Consistent<Incon, Stream<T, Cluster<'a, L2>, Unbounded, <O as MinOrder<N::OrderingGuarantee>>::Min, R>>
    where T: Clone + Serialize + DeserializeOwned, O: MinOrder<N::OrderingGuarantee> {
        Consistent::new(self.inner.broadcast(to, via, nondet_membership))
    }

    pub fn send_bincode_external<L2>(
        self,
        other: &External<L2>,
    ) -> crate::location::external_process::ExternalBincodeStream<T, O, R>
    where T: Serialize + DeserializeOwned {
        self.inner.send_bincode_external(other)
    }
}

/// Cluster → Process send: preserves consistency.
impl<'a, Con: Consistency, T, L, B: Boundedness, O: Ordering, R: Retries>
    Consistent<Con, Stream<T, Cluster<'a, L>, B, O, R>>
{
    pub fn send_bincode<L2>(
        self,
        other: &crate::prelude::Process<'a, L2>,
    ) -> crate::live_collections::keyed_stream::KeyedStream<
        crate::location::MemberId<L>, T, crate::prelude::Process<'a, L2>, Unbounded, O, R,
    >
    where T: Serialize + DeserializeOwned {
        // KeyedStream doesn't carry consistency — erase at this boundary
        self.inner.send_bincode(other)
    }

    pub fn send<L2, N: NetworkFor<T>>(
        self,
        to: &crate::prelude::Process<'a, L2>,
        via: N,
    ) -> crate::live_collections::keyed_stream::KeyedStream<
        crate::location::MemberId<L>, T, crate::prelude::Process<'a, L2>, Unbounded,
        <O as MinOrder<N::OrderingGuarantee>>::Min, R,
    >
    where T: Serialize + DeserializeOwned, O: MinOrder<N::OrderingGuarantee> {
        self.inner.send(to, via)
    }

    /// Broadcast from Cluster to Cluster: erases to Incon (dynamic membership).
    pub fn broadcast_bincode<L2: 'a>(
        self,
        other: &Cluster<'a, L2>,
        nondet_membership: NonDet,
    ) -> Consistent<Incon, crate::live_collections::keyed_stream::KeyedStream<
        crate::location::MemberId<L>, T, Cluster<'a, L2>, Unbounded, O, R,
    >>
    where T: Clone + Serialize + DeserializeOwned {
        Consistent::new(self.inner.broadcast_bincode(other, nondet_membership))
    }
}


// ---------------------------------------------------------------------------
// KeyedStream, Singleton, Optional: entry points and essential forwarding
// ---------------------------------------------------------------------------

use crate::live_collections::keyed_stream::KeyedStream;
use crate::live_collections::keyed_singleton::KeyedSingleton;

impl<K, V, L, B: Boundedness, O: Ordering, R: Retries> KeyedStream<K, V, L, B, O, R> {
    pub fn consistent<C: Consistency>(self) -> Consistent<C, Self> {
        Consistent::new(self)
    }
}

impl<T, L, B: crate::live_collections::singleton::SingletonBound> Singleton<T, L, B> {
    pub fn consistent<C: Consistency>(self) -> Consistent<C, Self> {
        Consistent::new(self)
    }
}

impl<T, L, B: Boundedness> Optional<T, L, B> {
    pub fn consistent<C: Consistency>(self) -> Consistent<C, Self> {
        Consistent::new(self)
    }
}

// ---- Consistent<C, Stream<(K,V)>> → Consistent<C, KeyedStream> ----

impl<'a, Con: Consistency, K, V, L: Location<'a>, B: Boundedness, O: Ordering, R: Retries>
    Consistent<Con, Stream<(K, V), L, B, O, R>>
{
    pub fn into_keyed(self) -> Consistent<Con, KeyedStream<K, V, L, B, O, R>> {
        Consistent::new(self.inner.into_keyed())
    }
}

// ---- KeyedStream preserving operators ----

impl<'a, Con: Consistency, K, V, L: Location<'a>, B: Boundedness, O: Ordering, R: Retries>
    Consistent<Con, KeyedStream<K, V, L, B, O, R>>
{
    pub fn entries(self) -> Consistent<Con, Stream<(K, V), L, B, NoOrder, R>> {
        Consistent::new(self.inner.entries())
    }

    pub fn values(self) -> Consistent<Con, Stream<V, L, B, NoOrder, R>> {
        Consistent::new(self.inner.values())
    }

    pub fn keys(self) -> Consistent<Con, Stream<K, L, B, NoOrder, ExactlyOnce>>
    where K: Eq + std::hash::Hash {
        Consistent::new(self.inner.keys())
    }

    pub fn ir_node_named(self, name: &str) -> Consistent<Con, KeyedStream<K, V, L, B, O, R>> {
        Consistent::new(self.inner.ir_node_named(name))
    }

    /// Fold with commutative+idempotent proof → Conv.
    pub fn fold<A, I, F, Comm, Idemp, M, B2: crate::live_collections::keyed_singleton::KeyedSingletonBound>(
        self,
        init: impl IntoQuotedMut<'a, I, L>,
        comb: impl IntoQuotedMut<'a, F, L, AggFuncAlgebra<Comm, Idemp, M>>,
    ) -> Consistent<Conv, KeyedSingleton<K, A, L, B2>>
    where
        I: Fn() -> A + 'a,
        F: Fn(&mut A, V),
        K: Eq + std::hash::Hash,
        Comm: ValidCommutativityFor<O> + IsProved,
        Idemp: ValidIdempotenceFor<R> + IsProved,
        B: crate::properties::ApplyMonotoneKeyedStream<M, B2>,
    {
        Consistent::new(self.inner.fold(init, comb))
    }

    pub fn reduce<F, Comm, Idemp>(
        self,
        comb: impl IntoQuotedMut<'a, F, L, AggFuncAlgebra<Comm, Idemp>>,
    ) -> Consistent<Conv, KeyedSingleton<K, V, L, B>>
    where
        F: Fn(&mut V, V) + 'a,
        K: Eq + std::hash::Hash,
        Comm: ValidCommutativityFor<O> + IsProved,
        Idemp: ValidIdempotenceFor<R> + IsProved,
    {
        Consistent::new(self.inner.reduce(comb))
    }
}

// ---- Optional: into_stream preserves C ----

impl<'a, Con: Consistency, T, L: Location<'a>, B: Boundedness + IsBounded>
    Consistent<Con, Optional<T, L, B>>
{
    pub fn into_stream(self) -> Consistent<Con, Stream<T, L, Bounded, TotalOrder, ExactlyOnce>>
    where B: IsBounded {
        Consistent::new(self.inner.into_stream())
    }
}

#[cfg(test)]
#[cfg(feature = "build")]
mod tests {
    use super::*;
    use crate::compile::builder::FlowBuilder;
    use crate::prelude::*;

    fn build(f: impl FnOnce(&mut FlowBuilder<'_>)) {
        let mut flow = FlowBuilder::new();
        f(&mut flow);
        let _ = flow.finalize();
    }

    #[test]
    fn source_iter_seq_propagates_through_map() {
        build(|flow| {
            let p = flow.process::<()>();
            let _s: Consistent<Seq, Stream<i32, _, _, _, _>> =
                p.source_iter(q!(vec![1, 2, 3]))
                    .consistent::<Seq>()
                    .map(q!(|x| x + 1));
        });
    }

    #[test]
    fn filter_preserves_seq() {
        build(|flow| {
            let p = flow.process::<()>();
            let _s: Consistent<Seq, Stream<i32, _, _, _, _>> =
                p.source_iter(q!(vec![1, 2, 3]))
                    .consistent::<Seq>()
                    .filter(q!(|x| *x > 1));
        });
    }

    #[test]
    fn chain_seq_seq_is_seq() {
        build(|flow| {
            let p = flow.process::<()>();
            let a = p.source_iter(q!(vec![1, 2])).consistent::<Seq>();
            let b = p.source_iter(q!(vec![3, 4])).consistent::<Seq>();
            let _s: Consistent<Seq, Stream<i32, _, _, _, _>> = a.chain(b);
        });
    }

    #[test]
    fn chain_seq_conv_is_conv() {
        build(|flow| {
            let p = flow.process::<()>();
            let a = p.source_iter(q!(vec![1, 2])).consistent::<Seq>();
            let b = p.source_iter(q!(vec![3, 4])).consistent::<Conv>();
            let _s: Consistent<Conv, Stream<i32, _, _, _, _>> = a.chain(b);
        });
    }

    #[test]
    fn for_each_on_seq() {
        build(|flow| {
            let p = flow.process::<()>();
            p.source_iter(q!(vec![1, 2, 3]))
                .consistent::<Seq>()
                .map(q!(|x| x + 1))
                .for_each(q!(|_| {}));
        });
    }
}
