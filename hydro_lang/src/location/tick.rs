//! Clock domains for batching streaming data into discrete time steps.
//!
//! In Hydro, a [`Tick`] represents a logical clock that can be used to batch
//! unbounded streaming data into discrete, bounded time steps. This is essential
//! for implementing iterative algorithms, synchronizing data across multiple
//! streams, and performing aggregations over windows of data.
//!
//! A tick is created from a top-level location (such as [`Process`] or [`Cluster`])
//! using [`Location::tick`]. Once inside a tick, bounded live collections can be
//! manipulated with operations like fold, reduce, and cross-product, and the
//! results can be emitted back to the unbounded stream using methods like
//! `all_ticks()`.
//!
//! The [`Atomic`] wrapper provides atomicity guarantees within a tick, ensuring
//! that reads and writes within a tick are serialized.
//!
//! The [`NoTick`] marker trait is used to constrain APIs that should only be
//! called on top-level locations (not inside a tick), while [`NoAtomic`] constrains
//! APIs that should not be called inside an atomic context.

use sealed::sealed;
use stageleft::{QuotedWithContext, q};

#[cfg(stageleft_runtime)]
use super::dynamic::DynLocation;
use super::{Cluster, Location, LocationId, Process};
use crate::compile::builder::{ClockId, FlowState};
use crate::compile::ir::{HydroNode, HydroSource};
#[cfg(stageleft_runtime)]
use crate::forward_handle::{CycleCollection, CycleCollectionWithInitial};
use crate::forward_handle::{TickCycle, TickCycleHandle};
use crate::live_collections::boundedness::Bounded;
use crate::live_collections::optional::Optional;
use crate::live_collections::singleton::Singleton;
use crate::live_collections::stream::{ExactlyOnce, Stream, TotalOrder};
use crate::nondet::nondet;

/// Marker trait for locations that are **not** inside a [`Tick`] clock domain.
///
/// This trait is implemented by top-level locations such as [`Process`] and [`Cluster`],
/// as well as [`Atomic`]. It is used to constrain APIs that should only be called
/// outside of a tick context (e.g., creating a new tick or sourcing external data).
#[sealed]
pub trait NoTick {}
#[sealed]
impl<T> NoTick for Process<'_, T> {}
#[sealed]
impl<T> NoTick for Cluster<'_, T> {}

/// Marker trait for locations that are **not** inside an [`Atomic`] context.
///
/// This trait is implemented by top-level locations ([`Process`], [`Cluster`]) and
/// by [`Tick`]. It is used to constrain APIs that should not be called from within
/// an atomic block.
#[sealed]
pub trait NoAtomic {}
#[sealed]
impl<T> NoAtomic for Process<'_, T> {}
#[sealed]
impl<T> NoAtomic for Cluster<'_, T> {}
#[sealed]
impl<'a, L> NoAtomic for Tick<L> where L: Location<'a> {}

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
    fn id(&self) -> LocationId {
        LocationId::Atomic(Box::new(self.tick.id()))
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
}

impl<'a, L> Location<'a> for Atomic<L>
where
    L: Location<'a>,
{
    type Root = L::Root;

    fn root(&self) -> Self::Root {
        self.tick.root()
    }
}

#[sealed]
impl<L> NoTick for Atomic<L> {}

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
    fn id(&self) -> LocationId {
        LocationId::Tick(self.id, Box::new(self.l.id()))
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
}

impl<'a, L> Location<'a> for Tick<L>
where
    L: Location<'a>,
{
    type Root = L::Root;

    fn root(&self) -> Self::Root {
        self.l.root()
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
        L: NoTick,
    {
        let out = self
            .l
            .spin()
            .flat_map_ordered(q!(move |_| 0..batch_size))
            .map(q!(|_| ()));

        out.batch(self, nondet!(/** at runtime, `spin` produces a single value per tick, so each batch is guaranteed to be the same size. */))
    }

    /// Constructs a [`Singleton`] materialized inside this tick with the given static value.
    ///
    /// The singleton will have the provided value on every tick. This is useful
    /// for providing constant values to computations inside a tick.
    ///
    /// See also: [`Location::singleton`], for creating a singleton _not_ inside a tick.
    pub fn singleton<T>(
        &self,
        e: impl QuotedWithContext<'a, T, Tick<L>>,
    ) -> Singleton<T, Self, Bounded>
    where
        T: Clone,
    {
        let e = e.splice_untyped_ctx(self);

        Singleton::new(
            self.clone(),
            HydroNode::SingletonSource {
                value: e.into(),
                first_tick_only: false,
                metadata: self.new_node_metadata(Singleton::<T, Self, Bounded>::collection_kind()),
            },
        )
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
    pub fn cycle<S>(&self) -> (TickCycleHandle<'a, S>, S)
    where
        S: CycleCollection<'a, TickCycle, Location = Self> + DeferTick,
        L: NoTick,
    {
        let cycle_id = self.flow_state().borrow_mut().next_cycle_id();
        (
            TickCycleHandle::new(cycle_id, Location::id(self)),
            S::create_source(cycle_id, self.clone()).defer_tick(),
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
    pub fn cycle_with_initial<S>(&self, initial: S) -> (TickCycleHandle<'a, S>, S)
    where
        S: CycleCollectionWithInitial<'a, TickCycle, Location = Self>,
    {
        let cycle_id = self.flow_state().borrow_mut().next_cycle_id();
        (
            TickCycleHandle::new(cycle_id, Location::id(self)),
            // no need to defer_tick, create_source_with_initial does it for us
            S::create_source_with_initial(cycle_id, initial, self.clone()),
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
    use crate::location::Location;
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
}
