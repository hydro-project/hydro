//! Clock domains for batching streaming data into discrete time steps.
//!
//! In Hydro, a [`Tick`] represents a logical clock that can be used to batch
//! unbounded streaming data into discrete, bounded time steps. This is essential
//! for implementing iterative algorithms, synchronizing data across multiple
//! streams, and performing aggregations over windows of data.
//!
//! A tick is created from a top-level location (such as [`super::Process`] or [`super::Cluster`])
//! using [`Location::tick`]. Once inside a tick, bounded live collections can be
//! manipulated with operations like fold, reduce, and cross-product, and the
//! results can be emitted back to the unbounded stream using methods like
//! `all_ticks()`.
//!
//! The [`Atomic`] wrapper provides atomicity guarantees within a tick, ensuring
//! that reads and writes within a tick are serialized.

use stageleft::{QuotedWithContext, q};

#[cfg(stageleft_runtime)]
use super::dynamic::DynLocation;
use super::{Location, LocationId};
use crate::compile::builder::{ClockId, FlowState};
use crate::compile::ir::{HydroNode, HydroSource};
#[cfg(stageleft_runtime)]
use crate::forward_handle::{CycleCollection, CycleCollectionWithInitial};
use crate::forward_handle::{TickCycle, TickCycleHandle};
#[cfg(feature = "tokio")]
use crate::live_collections::Singleton;
use crate::live_collections::boundedness::Bounded;
use crate::live_collections::optional::Optional;
use crate::live_collections::stream::{ExactlyOnce, Stream, TotalOrder};
use crate::location::TopLevel;
#[cfg(feature = "tokio")]
use crate::nondet::NonDet;
use crate::nondet::nondet;

/// A location wrapper that provides atomicity guarantees within a [`Tick`].
///
/// An `Atomic` context establishes a happens-before relationship between operations:
/// - Downstream computations from `atomic()` are associated with an internal tick
/// - Outputs from `end_atomic()` are held until all computations in the tick complete
/// - Snapshots via `use::atomic` are guaranteed to reflect all updates from associated `end_atomic()`
///
/// This ensures read-after-write consistency: if a client receives an acknowledgement
/// from `end_atomic()`, any subsequent `use::atomic` snapshot will include the effects
/// of that acknowledged operation.
#[derive(Clone)]
pub struct Atomic<Loc> {
    pub(crate) tick: Tick<Loc>,
}

impl<L: DynLocation> DynLocation for Atomic<L> {
    fn dyn_id(&self) -> LocationId {
        LocationId::Atomic(Box::new(self.tick.dyn_id()))
    }

    fn flow_state(&self) -> &FlowState {
        self.tick.flow_state()
    }

    fn is_top_level() -> bool {
        L::is_top_level()
    }

    fn multiversioned(&self) -> bool {
        self.tick.multiversioned()
    }

    fn cluster_consistency() -> Option<super::dynamic::ClusterConsistency> {
        L::cluster_consistency()
    }
}

impl<'a, L> Location<'a> for Atomic<L>
where
    L: Location<'a>,
{
    type Root = L::Root;

    type DropConsistency = Atomic<L::DropConsistency>;

    fn consistency() -> Option<super::dynamic::ClusterConsistency> {
        L::consistency()
    }

    fn root(&self) -> Self::Root {
        self.tick.root()
    }

    fn drop_consistency(&self) -> Self::DropConsistency {
        Atomic {
            tick: self.tick.drop_consistency(),
        }
    }

    fn from_drop_consistency(l2: Self::DropConsistency) -> Self {
        Atomic {
            tick: Tick::from_drop_consistency(l2.tick),
        }
    }
}

/// Trait for live collections that can be deferred by one tick.
///
/// When a collection implements `DeferTick`, calling `defer_tick` delays its
/// values by one clock cycle. This is primarily used internally to implement
/// tick-based cycles ([`Tick::cycle`]), ensuring that feedback loops advance
/// by one tick to avoid infinite recursion within a single tick.
pub trait DeferTick {
    /// Returns a new collection whose values are delayed by one tick.
    fn defer_tick(self) -> Self;
}

/// Marks the stream as being inside the single global clock domain.
#[derive(Clone)]
pub struct Tick<L> {
    pub(crate) id: ClockId,
    /// Location.
    pub(crate) l: L,
}

impl<L: DynLocation> DynLocation for Tick<L> {
    fn dyn_id(&self) -> LocationId {
        LocationId::Tick(self.id, Box::new(self.l.dyn_id()))
    }

    fn flow_state(&self) -> &FlowState {
        self.l.flow_state()
    }

    fn is_top_level() -> bool {
        false
    }

    fn multiversioned(&self) -> bool {
        self.l.multiversioned()
    }

    fn cluster_consistency() -> Option<super::dynamic::ClusterConsistency> {
        L::cluster_consistency()
    }
}

impl<'a, L> Location<'a> for Tick<L>
where
    L: Location<'a>,
{
    type Root = L::Root;

    type DropConsistency = Tick<L::DropConsistency>;

    fn consistency() -> Option<super::dynamic::ClusterConsistency> {
        L::consistency()
    }

    fn root(&self) -> Self::Root {
        self.l.root()
    }

    fn drop_consistency(&self) -> Self::DropConsistency {
        Tick {
            id: self.id,
            l: self.l.drop_consistency(),
        }
    }

    fn from_drop_consistency(l2: Self::DropConsistency) -> Self {
        Tick {
            id: l2.id,
            l: L::from_drop_consistency(l2.l),
        }
    }
}

impl<'a, L> Tick<L>
where
    L: Location<'a>,
{
    /// Returns a reference to the outer (parent) location that this tick is nested within.
    ///
    /// For example, if a `Tick` was created from a `Process`, this returns a reference
    /// to that `Process`.
    pub fn outer(&self) -> &L {
        &self.l
    }

    /// Creates a bounded stream of `()` values inside this tick, with a fixed batch size.
    ///
    /// This is useful for driving computations inside a tick that need to process
    /// a specific number of elements per tick. Each tick will produce exactly
    /// `batch_size` unit values.
    pub fn spin_batch(
        &self,
        batch_size: impl QuotedWithContext<'a, usize, L> + Copy + 'a,
    ) -> Stream<(), Self, Bounded, TotalOrder, ExactlyOnce>
    where
        L: TopLevel<'a>,
    {
        let out = self
            .l
            .spin()
            .flat_map_ordered(q!(move |_| 0..batch_size))
            .map(q!(|_| ()));

        let inner = out.batch(self, nondet!(/** at runtime, `spin` produces a single value per tick, so each batch is guaranteed to be the same size. */));
        Stream::new(self.clone(), inner.ir_node.replace(HydroNode::Placeholder))
    }

    /// Creates an [`Optional`] which has a null value on every tick.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let optional = tick.none::<i32>();
    /// optional.unwrap_or(tick.singleton(q!(123)))
    /// # .all_ticks()
    /// # }, |mut stream| async move {
    /// // 123
    /// # assert_eq!(stream.next().await.unwrap(), 123);
    /// # }));
    /// # }
    /// ```
    pub fn none<T>(&self) -> Optional<T, Self, Bounded> {
        let e = q!([]);
        let e = QuotedWithContext::<'a, [(); 0], Self>::splice_typed_ctx(e, self);

        let unit_optional: Optional<(), Self, Bounded> = Optional::new(
            self.clone(),
            HydroNode::Source {
                source: HydroSource::Iter(e.into()),
                metadata: self.new_node_metadata(Optional::<(), Self, Bounded>::collection_kind()),
            },
        );

        unit_optional.map(q!(|_| unreachable!())) // always empty
    }

    /// Creates an [`Optional`] which will have the provided static value on the first tick, and be
    /// null on all subsequent ticks.
    ///
    /// This is useful for bootstrapping stateful computations which need an initial value.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// // ticks are lazy by default, forces the second tick to run
    /// tick.spin_batch(q!(1)).all_ticks().for_each(q!(|_| {}));
    /// let optional = tick.optional_first_tick(q!(5));
    /// optional.unwrap_or(tick.singleton(q!(123))).all_ticks()
    /// # }, |mut stream| async move {
    /// // 5, 123, 123, 123, ...
    /// # assert_eq!(stream.next().await.unwrap(), 5);
    /// # assert_eq!(stream.next().await.unwrap(), 123);
    /// # assert_eq!(stream.next().await.unwrap(), 123);
    /// # assert_eq!(stream.next().await.unwrap(), 123);
    /// # }));
    /// # }
    /// ```
    pub fn optional_first_tick<T: Clone>(
        &self,
        e: impl QuotedWithContext<'a, T, Tick<L>>,
    ) -> Optional<T, Self, Bounded> {
        let e = e.splice_untyped_ctx(self);

        Optional::new(
            self.clone(),
            HydroNode::SingletonSource {
                value: e.into(),
                first_tick_only: true,
                metadata: self.new_node_metadata(Optional::<T, Self, Bounded>::collection_kind()),
            },
        )
    }

    /// Returns the current wall-clock time as a [`Singleton`] containing a
    /// [`tokio::time::Instant`].
    ///
    /// # Non-Determinism
    /// Reading wall-clock time is inherently non-deterministic because the
    /// value depends on when the tick executes. A [`NonDet`] guard is required
    /// to acknowledge this.
    #[cfg(feature = "tokio")]
    pub fn current_tick_instant(
        &self,
        _nondet: NonDet,
    ) -> Singleton<tokio::time::Instant, Tick<L::DropConsistency>, Bounded>
    where
        Self: Sized,
    {
        // TODO(shadaj): this is a simulator hole, should be reported as unsupported until it is
        self.singleton(q!(tokio::time::Instant::now()))
    }

    /// Creates a feedback cycle within this tick for implementing iterative computations.
    ///
    /// Returns a handle that must be completed with the actual collection, and a placeholder
    /// collection that represents the output of the previous tick (deferred by one tick).
    /// This is useful for implementing fixed-point computations where the output of one
    /// tick feeds into the input of the next.
    ///
    /// The cycle automatically defers values by one tick to prevent infinite recursion.
    #[expect(
        private_bounds,
        reason = "only Hydro collections can implement ReceiverComplete"
    )]
    pub fn cycle<S, L2: Location<'a, DropConsistency = Tick<L::DropConsistency>>>(
        &self,
    ) -> (TickCycleHandle<'a, S>, S)
    where
        S: CycleCollection<'a, TickCycle, Location = L2> + DeferTick,
    {
        let cycle_id = self.flow_state().borrow_mut().next_cycle_id();
        (
            TickCycleHandle::new(cycle_id, Location::id(self)),
            S::create_source(cycle_id, self.clone().with_consistency_of()).defer_tick(),
        )
    }

    /// Creates a feedback cycle with an initial value for the first tick.
    ///
    /// Similar to [`Tick::cycle`], but allows providing an initial collection
    /// that will be used as the value on the first tick before any feedback
    /// is available. This is useful for bootstrapping iterative computations
    /// that need a starting state.
    #[expect(
        private_bounds,
        reason = "only Hydro collections can implement ReceiverComplete"
    )]
    pub fn cycle_with_initial<S, L2: Location<'a, DropConsistency = Tick<L::DropConsistency>>>(
        &self,
        initial: S,
    ) -> (TickCycleHandle<'a, S>, S)
    where
        S: CycleCollectionWithInitial<'a, TickCycle, Location = L2>,
    {
        let cycle_id = self.flow_state().borrow_mut().next_cycle_id();
        (
            TickCycleHandle::new(cycle_id, Location::id(self)),
            // no need to defer_tick, create_source_with_initial does it for us
            S::create_source_with_initial(cycle_id, initial, self.clone().with_consistency_of()),
        )
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "sim")]
    use stageleft::q;

    #[cfg(feature = "sim")]
    use crate::live_collections::sliced::sliced;
    #[cfg(feature = "sim")]
    use crate::nondet::nondet;
    #[cfg(feature = "sim")]
    use crate::prelude::FlowBuilder;

    #[cfg(feature = "sim")]
    #[test]
    fn sim_atomic_stream() {
        let mut flow = FlowBuilder::new();
        let node = flow.process::<()>();

        let (write_send, write_req) = node.sim_input();
        let (read_send, read_req) = node.sim_input::<(), _, _>();

        let atomic_write = write_req.atomic();
        let current_state = atomic_write.clone().fold(
            q!(|| 0),
            q!(|state: &mut i32, v: i32| {
                *state += v;
            }),
        );

        let write_ack_recv = atomic_write.end_atomic().sim_output();
        let read_response_recv = sliced! {
            let batch_of_req = use(read_req, nondet!(/** test */));
            let latest_singleton = use::atomic(current_state, nondet!(/** test */));
            batch_of_req.cross_singleton(latest_singleton)
        }
        .sim_output();

        let sim_compiled = flow.sim().compiled();
        let instances = sim_compiled.exhaustive(async || {
            write_send.send(1);
            write_ack_recv.assert_yields([1]).await;
            read_send.send(());
            assert!(read_response_recv.next().await.is_some_and(|(_, v)| v >= 1));
        });

        assert_eq!(instances, 1);

        let instances_read_before_write = sim_compiled.exhaustive(async || {
            write_send.send(1);
            read_send.send(());
            write_ack_recv.assert_yields([1]).await;
            let _ = read_response_recv.next().await;
        });

        assert_eq!(instances_read_before_write, 3); // read before write, write before read, both in same tick
    }

    #[cfg(feature = "sim")]
    #[test]
    #[should_panic]
    fn sim_non_atomic_stream() {
        // shows that atomic is necessary
        let mut flow = FlowBuilder::new();
        let node = flow.process::<()>();

        let (write_send, write_req) = node.sim_input();
        let (read_send, read_req) = node.sim_input::<(), _, _>();

        let current_state = write_req.clone().fold(
            q!(|| 0),
            q!(|state: &mut i32, v: i32| {
                *state += v;
            }),
        );

        let write_ack_recv = write_req.sim_output();

        let read_response_recv = sliced! {
            let batch_of_req = use(read_req, nondet!(/** test */));
            let latest_singleton = use(current_state, nondet!(/** test */));
            batch_of_req.cross_singleton(latest_singleton)
        }
        .sim_output();

        flow.sim().exhaustive(async || {
            write_send.send(1);
            write_ack_recv.assert_yields([1]).await;
            read_send.send(());

            if let Some((_, v)) = read_response_recv.next().await {
                assert_eq!(v, 1);
            }
        });
    }

    /// Tests that sim_atomic_input allows the simulator to explore both:
    /// 1. The batch for the regular input is non-empty on the first tick (same tick as atomic write)
    /// 2. The batch for the regular input is empty on the first tick (read comes in a later tick)
    ///
    /// This is impossible with the ack pattern because awaiting the ack forces
    /// the atomic input to always be processed in a separate tick first.
    #[cfg(feature = "sim")]
    #[test]
    fn sim_atomic_input_same_tick() {
        let mut flow = FlowBuilder::new();
        let node = flow.process::<()>();

        let (write_send, atomic_write) = node.sim_atomic_input();
        let (read_send, read_req) = node.sim_input::<(), _, _>();

        let current_state = atomic_write.clone().fold(
            q!(|| 0),
            q!(|state: &mut i32, v: i32| {
                *state += v;
            }),
        );

        let read_response_recv = sliced! {
            let batch_of_req = use(read_req, nondet!(/** test */));
            let latest_singleton = use::atomic(current_state, nondet!(/** test */));
            batch_of_req.collect_vec().zip(latest_singleton).into_stream()
        }
        .sim_output();

        let sim_compiled = flow.sim().compiled();
        let mut saw_same_tick = false;
        let mut saw_separate_tick = false;
        let instances = sim_compiled.exhaustive(async || {
            write_send.send_atomic(1);
            read_send.send(());
            let (batch, state) = read_response_recv.next().await.unwrap();
            assert_eq!(state, 1);
            if batch.is_empty() {
                saw_separate_tick = true;
            } else {
                saw_same_tick = true;
            }
        });

        assert_eq!(instances, 2);
        assert!(
            saw_same_tick,
            "expected an instance where read is in the same tick as write"
        );
        assert!(
            saw_separate_tick,
            "expected an instance where read is in a later tick"
        );
    }
}
