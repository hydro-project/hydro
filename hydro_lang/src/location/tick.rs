use std::marker::PhantomData;

use proc_macro2::Span;
use sealed::sealed;
use stageleft::{QuotedWithContext, q};

#[cfg(stageleft_runtime)]
use super::dynamic::DynLocation;
use super::{Cluster, Location, LocationId, Process};
use crate::builder::FlowState;
use crate::builder::ir::{HydroNode, HydroSource};
#[cfg(stageleft_runtime)]
use crate::forward_handle::{CycleCollection, CycleCollectionWithInitial};
use crate::forward_handle::{ForwardHandle, ForwardRef, TickCycle, TickCycleHandle};
use crate::live_collections::boundedness::Bounded;
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
        self.tick.id()
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
        L: NoTick + NoAtomic,
    {
        let out = self
            .l
            .spin()
            .flat_map_ordered(q!(move |_| 0..batch_size))
            .map(q!(|_| ()));

        out.batch(self, nondet!(/** at runtime, `spin` produces a single value per tick, so each batch is guaranteed to be the same size. */))
    }

    pub fn singleton<T>(&self, e: impl QuotedWithContext<'a, T, L>) -> Singleton<T, Self, Bounded>
    where
        T: Clone,
        L: NoTick + NoAtomic,
    {
        self.outer().singleton(e).snapshot(
            self,
            nondet!(/** a top-level singleton produces the same value each tick */),
        )
    }

    pub fn optional_first_tick<T: Clone>(
        &self,
        e: impl QuotedWithContext<'a, T, Tick<L>>,
    ) -> Optional<T, Self, Bounded> {
        let e_arr = q!([e]);
        let e = e_arr.splice_untyped_ctx(self);

        Optional::new(
            self.clone(),
            HydroNode::Source {
                source: HydroSource::Iter(e.into()),
                metadata: self.new_node_metadata::<T>(),
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
            // no need to defer_tick, create_source_with_initial does it for us
            S::create_source_with_initial(ident, initial, self.clone()),
        )
    }
}
