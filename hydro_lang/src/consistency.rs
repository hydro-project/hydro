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

impl<C: Consistency, S> std::ops::Deref for Consistent<C, S> {
    type Target = S;
    fn deref(&self) -> &S {
        &self.inner
    }
}

impl<C: Consistency, S> std::ops::DerefMut for Consistent<C, S> {
    fn deref_mut(&mut self) -> &mut S {
        &mut self.inner
    }
}

// ---------------------------------------------------------------------------
// Stream operator forwarding
// ---------------------------------------------------------------------------

use crate::live_collections::boundedness::{Bounded, Boundedness, IsBounded};
use crate::live_collections::stream::{
    ExactlyOnce, MinOrder, MinRetries, NoOrder, Ordering, Retries, Stream, TotalOrder,
};
use crate::location::Location;
use stageleft::IntoQuotedMut;

/// Preserving operators: map, filter, filter_map, inspect.
/// These pass the consistency level through unchanged.
impl<'a, C: Consistency, T, L: Location<'a>, B: Boundedness, O: Ordering, R: Retries>
    Consistent<C, Stream<T, L, B, O, R>>
{
    pub fn map<U, F>(self, f: impl IntoQuotedMut<'a, F, L>) -> Consistent<C, Stream<U, L, B, O, R>>
    where
        F: Fn(T) -> U + 'a,
    {
        Consistent::new(self.inner.map(f))
    }

    pub fn filter<F>(self, f: impl IntoQuotedMut<'a, F, L>) -> Consistent<C, Stream<T, L, B, O, R>>
    where
        F: Fn(&T) -> bool + 'a,
    {
        Consistent::new(self.inner.filter(f))
    }

    pub fn filter_map<U, F>(self, f: impl IntoQuotedMut<'a, F, L>) -> Consistent<C, Stream<U, L, B, O, R>>
    where
        F: Fn(T) -> Option<U> + 'a,
    {
        Consistent::new(self.inner.filter_map(f))
    }

    pub fn inspect<F>(self, f: impl IntoQuotedMut<'a, F, L>) -> Consistent<C, Stream<T, L, B, O, R>>
    where
        F: Fn(&T) + 'a,
    {
        Consistent::new(self.inner.inspect(f))
    }

    /// Terminal: consume the stream. The consistency level `C` is visible
    /// in the type of `self` before this call.
    pub fn for_each<F>(self, f: impl IntoQuotedMut<'a, F, L>)
    where
        F: Fn(T) + 'a,
        L: crate::location::NoTick,
        O: crate::live_collections::stream::IsOrdered,
        R: crate::live_collections::stream::IsExactlyOnce,
    {
        self.inner.for_each(f)
    }
}

/// Chain: computes MinConsistency of the two inputs.
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
}

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
