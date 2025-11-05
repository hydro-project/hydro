use std::marker::PhantomData;

use proc_macro2::Span;
use sealed::sealed;
use stageleft::{QuotedWithContext, q};

#[cfg(stageleft_runtime)]
use super::dynamic::DynLocation;
use super::{Cluster, Location, LocationId, Process};
use crate::compile::builder::FlowState;
use crate::compile::ir::{HydroNode, HydroSource};
#[cfg(stageleft_runtime)]
use crate::forward_handle::{CycleCollection, CycleCollectionWithInitial};
use crate::forward_handle::{ForwardHandle, ForwardRef, TickCycle, TickCycleHandle};
use crate::live_collections::boundedness::{Bounded, Unbounded};
use crate::live_collections::optional::Optional;
use crate::live_collections::singleton::Singleton;
use crate::live_collections::stream::{ExactlyOnce, Stream, TotalOrder};
use crate::nondet::nondet;

#[sealed]
pub trait NoTick {}
#[sealed]
impl<T> NoTick for Process<'_, T> {}
#[sealed]
impl<T> NoTick for Cluster<'_, T> {}

#[sealed]
pub trait NoAtomic {}
#[sealed]
impl<T> NoAtomic for Process<'_, T> {}
#[sealed]
impl<T> NoAtomic for Cluster<'_, T> {}
#[sealed]
impl<'a, L> NoAtomic for Tick<L> where L: Location<'a> {}

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

pub trait DeferTick {
    fn defer_tick(self) -> Self;
}

/// Marks the stream as being inside the single global clock domain.
#[derive(Clone)]
pub struct Tick<L> {
    pub(crate) id: usize,
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

impl<'a, L> Tick<Atomic<L>>
where
    L: Location<'a>,
{
    pub fn as_regular_tick(&self) -> Tick<L> {
        self.l.tick.clone()
    }
}

impl<'a, L> Tick<L>
where
    L: Location<'a>,
{
    pub fn outer(&self) -> &L {
        &self.l
    }

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
                metadata: self.new_node_metadata(Singleton::<T, Self, Bounded>::collection_kind()),
            },
        )
    }

    /// Creates an [`Optional`] which has a null value on every tick.
    ///
    /// # Example
    /// ```rust
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
    /// ```
    pub fn optional_first_tick<T: Clone>(
        &self,
        e: impl QuotedWithContext<'a, T, Tick<L>>,
    ) -> Optional<T, Self, Bounded> {
        let e_arr = q!([e]);
        let e = e_arr.splice_untyped_ctx(self);

        Optional::new(
            self.clone(),
            HydroNode::Batch {
                inner: Box::new(HydroNode::Source {
                    source: HydroSource::Iter(e.into()),
                    metadata: self
                        .outer()
                        .new_node_metadata(Optional::<T, L, Unbounded>::collection_kind()),
                }),
                metadata: self.new_node_metadata(Optional::<T, Self, Bounded>::collection_kind()),
            },
        )
    }

    #[expect(
        private_bounds,
        reason = "only Hydro collections can implement ReceiverComplete"
    )]
    pub fn forward_ref<S>(&self) -> (ForwardHandle<'a, S>, S)
    where
        S: CycleCollection<'a, ForwardRef, Location = Self>,
        L: NoTick,
    {
        let next_id = self.flow_state().borrow_mut().next_cycle_id();
        let ident = syn::Ident::new(&format!("cycle_{}", next_id), Span::call_site());

        (
            ForwardHandle {
                completed: false,
                ident: ident.clone(),
                expected_location: Location::id(self),
                _phantom: PhantomData,
            },
            S::create_source(ident, self.clone()),
        )
    }

    #[expect(
        private_bounds,
        reason = "only Hydro collections can implement ReceiverComplete"
    )]
    pub fn cycle<S>(&self) -> (TickCycleHandle<'a, S>, S)
    where
        S: CycleCollection<'a, TickCycle, Location = Self> + DeferTick,
        L: NoTick,
    {
        let next_id = self.flow_state().borrow_mut().next_cycle_id();
        let ident = syn::Ident::new(&format!("cycle_{}", next_id), Span::call_site());

        (
            TickCycleHandle {
                completed: false,
                ident: ident.clone(),
                expected_location: Location::id(self),
                _phantom: PhantomData,
            },
            S::create_source(ident, self.clone()).defer_tick(),
        )
    }

    #[expect(
        private_bounds,
        reason = "only Hydro collections can implement ReceiverComplete"
    )]
    pub fn cycle_with_initial<S>(&self, initial: S) -> (TickCycleHandle<'a, S>, S)
    where
        S: CycleCollectionWithInitial<'a, TickCycle, Location = Self>,
    {
        let next_id = self.flow_state().borrow_mut().next_cycle_id();
        let ident = syn::Ident::new(&format!("cycle_{}", next_id), Span::call_site());

        (
            TickCycleHandle {
                completed: false,
                ident: ident.clone(),
                expected_location: Location::id(self),
                _phantom: PhantomData,
            },
            // no need to defer_tick, create_source_with_initial does it for us
            S::create_source_with_initial(ident, initial, self.clone()),
        )
    }
}

#[cfg(test)]
mod tests {
    use stageleft::q;

    use crate::live_collections::sliced::sliced;
    use crate::location::Location;
    use crate::nondet::nondet;
    use crate::prelude::FlowBuilder;

    #[test]
    fn sim_atomic_stream() {
        let flow = FlowBuilder::new();
        let node = flow.process::<()>();
        let external = flow.external::<()>();

        let (input_write, write_req) = node.source_external_bincode(&external);
        let (input_read, read_req) = node.source_external_bincode::<_, (), _, _>(&external);

        let tick = node.tick();
        let atomic_write = write_req.atomic(&tick);
        let current_state = atomic_write.clone().fold(
            q!(|| 0),
            q!(|state: &mut i32, v: i32| {
                *state += v;
            }),
        );

        let write_ack = atomic_write.end_atomic().send_bincode_external(&external);
        let read_response = sliced!(|
            let batch_of_req = use(read_req, nondet!(/** test */));
            let latest_singleton = use::atomic(current_state, nondet!(/** test */));
        | {
            batch_of_req.cross_singleton(latest_singleton)
        })
        .send_bincode_external(&external);

        let sim_compiled = flow.sim().compiled();
        let instances = sim_compiled.exhaustive(async |mut compiled| {
            let write_send = compiled.connect(&input_write);
            let read_send = compiled.connect(&input_read);
            let mut write_ack_recv = compiled.connect(&write_ack);
            let mut read_response_recv = compiled.connect(&read_response);
            compiled.launch();

            write_send.send(1);
            write_ack_recv.assert_yields([1]).await;
            read_send.send(());
            assert!(read_response_recv.next().await.is_some_and(|(_, v)| v >= 1));
        });

        assert_eq!(instances, 1);

        let instances_read_before_write = sim_compiled.exhaustive(async |mut compiled| {
            let write_send = compiled.connect(&input_write);
            let read_send = compiled.connect(&input_read);
            let mut write_ack_recv = compiled.connect(&write_ack);
            let mut read_response_recv = compiled.connect(&read_response);
            compiled.launch();

            write_send.send(1);
            read_send.send(());
            write_ack_recv.assert_yields([1]).await;
            let _ = read_response_recv.next().await;
        });

        assert_eq!(instances_read_before_write, 3); // read before write, write before read, both in same tick
    }

    #[test]
    #[should_panic]
    fn sim_non_atomic_stream() {
        // shows that atomic is necessary
        let flow = FlowBuilder::new();
        let node = flow.process::<()>();
        let external = flow.external::<()>();

        let (input_write, write_req) = node.source_external_bincode(&external);
        let (input_read, read_req) = node.source_external_bincode::<_, (), _, _>(&external);

        let current_state = write_req.clone().fold(
            q!(|| 0),
            q!(|state: &mut i32, v: i32| {
                *state += v;
            }),
        );

        let write_ack = write_req.send_bincode_external(&external);

        let read_response = sliced!(|
            let batch_of_req = use(read_req, nondet!(/** test */));
            let latest_singleton = use(current_state, nondet!(/** test */));
        | {
            batch_of_req.cross_singleton(latest_singleton)
        })
        .send_bincode_external(&external);

        flow.sim().exhaustive(async |mut compiled| {
            let write_send = compiled.connect(&input_write);
            let read_send = compiled.connect(&input_read);
            let mut write_ack_recv = compiled.connect(&write_ack);
            let mut read_response_recv = compiled.connect(&read_response);
            compiled.launch();

            write_send.send(1);
            write_ack_recv.assert_yields([1]).await;
            read_send.send(());

            if let Some((_, v)) = read_response_recv.next().await {
                assert_eq!(v, 1);
            }
        });
    }
}
