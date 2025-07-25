use std::cell::RefCell;
use std::marker::PhantomData;
use std::ops::Deref;
use std::rc::Rc;

use stageleft::{IntoQuotedMut, QuotedWithContext, q};

use crate::builder::FLOW_USED_MESSAGE;
use crate::cycle::{
    CycleCollection, CycleCollectionWithInitial, CycleComplete, DeferTick, ForwardRefMarker,
    TickCycleMarker,
};
use crate::ir::{HydroLeaf, HydroNode, TeeNode};
use crate::location::tick::{Atomic, NoAtomic};
use crate::location::{Location, LocationId, NoTick, Tick, check_matching_location};
use crate::stream::{AtLeastOnce, ExactlyOnce};
use crate::{Bounded, Optional, Stream, TotalOrder, Unbounded};

pub struct Singleton<Type, Loc, Bound> {
    pub(crate) location: Loc,
    pub(crate) ir_node: RefCell<HydroNode>,

    _phantom: PhantomData<(Type, Loc, Bound)>,
}

impl<'a, T, L, B> Singleton<T, L, B>
where
    L: Location<'a>,
{
    pub(crate) fn new(location: L, ir_node: HydroNode) -> Self {
        Singleton {
            location,
            ir_node: RefCell::new(ir_node),
            _phantom: PhantomData,
        }
    }

    fn location_kind(&self) -> LocationId {
        self.location.id()
    }
}

impl<'a, T, L> From<Singleton<T, L, Bounded>> for Singleton<T, L, Unbounded>
where
    L: Location<'a>,
{
    fn from(singleton: Singleton<T, L, Bounded>) -> Self {
        Singleton::new(singleton.location, singleton.ir_node.into_inner())
    }
}

impl<'a, T, L> DeferTick for Singleton<T, Tick<L>, Bounded>
where
    L: Location<'a>,
{
    fn defer_tick(self) -> Self {
        Singleton::defer_tick(self)
    }
}

impl<'a, T, L> CycleCollectionWithInitial<'a, TickCycleMarker> for Singleton<T, Tick<L>, Bounded>
where
    L: Location<'a>,
{
    type Location = Tick<L>;

    fn create_source(ident: syn::Ident, initial: Self, location: Tick<L>) -> Self {
        let location_id = location.id();
        Singleton::new(
            location.clone(),
            HydroNode::Chain {
                first: Box::new(HydroNode::CycleSource {
                    ident,
                    location_kind: location_id,
                    metadata: location.new_node_metadata::<T>(),
                }),
                second: initial
                    .continue_if(location.optional_first_tick(q!(())))
                    .ir_node
                    .into_inner()
                    .into(),
                metadata: location.new_node_metadata::<T>(),
            },
        )
    }
}

impl<'a, T, L> CycleComplete<'a, TickCycleMarker> for Singleton<T, Tick<L>, Bounded>
where
    L: Location<'a>,
{
    fn complete(self, ident: syn::Ident, expected_location: LocationId) {
        assert_eq!(
            self.location.id(),
            expected_location,
            "locations do not match"
        );
        self.location
            .flow_state()
            .borrow_mut()
            .leaves
            .as_mut()
            .expect(FLOW_USED_MESSAGE)
            .push(HydroLeaf::CycleSink {
                ident,
                location_kind: self.location_kind(),
                input: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata::<T>(),
            });
    }
}

impl<'a, T, L> CycleCollection<'a, ForwardRefMarker> for Singleton<T, Tick<L>, Bounded>
where
    L: Location<'a>,
{
    type Location = Tick<L>;

    fn create_source(ident: syn::Ident, location: Tick<L>) -> Self {
        let location_id = location.id();
        Singleton::new(
            location.clone(),
            HydroNode::CycleSource {
                ident,
                location_kind: location_id,
                metadata: location.new_node_metadata::<T>(),
            },
        )
    }
}

impl<'a, T, L> CycleComplete<'a, ForwardRefMarker> for Singleton<T, Tick<L>, Bounded>
where
    L: Location<'a>,
{
    fn complete(self, ident: syn::Ident, expected_location: LocationId) {
        assert_eq!(
            self.location.id(),
            expected_location,
            "locations do not match"
        );
        self.location
            .flow_state()
            .borrow_mut()
            .leaves
            .as_mut()
            .expect(FLOW_USED_MESSAGE)
            .push(HydroLeaf::CycleSink {
                ident,
                location_kind: self.location_kind(),
                input: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata::<T>(),
            });
    }
}

impl<'a, T, L, B> CycleCollection<'a, ForwardRefMarker> for Singleton<T, L, B>
where
    L: Location<'a> + NoTick,
{
    type Location = L;

    fn create_source(ident: syn::Ident, location: L) -> Self {
        let location_id = location.id();
        Singleton::new(
            location.clone(),
            HydroNode::Persist {
                inner: Box::new(HydroNode::CycleSource {
                    ident,
                    location_kind: location_id,
                    metadata: location.new_node_metadata::<T>(),
                }),
                metadata: location.new_node_metadata::<T>(),
            },
        )
    }
}

impl<'a, T, L, B> CycleComplete<'a, ForwardRefMarker> for Singleton<T, L, B>
where
    L: Location<'a> + NoTick,
{
    fn complete(self, ident: syn::Ident, expected_location: LocationId) {
        assert_eq!(
            self.location.id(),
            expected_location,
            "locations do not match"
        );
        let metadata = self.location.new_node_metadata::<T>();
        self.location
            .flow_state()
            .borrow_mut()
            .leaves
            .as_mut()
            .expect(FLOW_USED_MESSAGE)
            .push(HydroLeaf::CycleSink {
                ident,
                location_kind: self.location_kind(),
                input: Box::new(HydroNode::Unpersist {
                    inner: Box::new(self.ir_node.into_inner()),
                    metadata: metadata.clone(),
                }),
                metadata,
            });
    }
}

impl<'a, T, L, B> Clone for Singleton<T, L, B>
where
    T: Clone,
    L: Location<'a>,
{
    fn clone(&self) -> Self {
        if !matches!(self.ir_node.borrow().deref(), HydroNode::Tee { .. }) {
            let orig_ir_node = self.ir_node.replace(HydroNode::Placeholder);
            *self.ir_node.borrow_mut() = HydroNode::Tee {
                inner: TeeNode(Rc::new(RefCell::new(orig_ir_node))),
                metadata: self.location.new_node_metadata::<T>(),
            };
        }

        if let HydroNode::Tee { inner, metadata } = self.ir_node.borrow().deref() {
            Singleton {
                location: self.location.clone(),
                ir_node: HydroNode::Tee {
                    inner: TeeNode(inner.0.clone()),
                    metadata: metadata.clone(),
                }
                .into(),
                _phantom: PhantomData,
            }
        } else {
            unreachable!()
        }
    }
}

impl<'a, T, L, B> Singleton<T, L, B>
where
    L: Location<'a>,
{
    pub fn map<U, F>(self, f: impl IntoQuotedMut<'a, F, L>) -> Singleton<U, L, B>
    where
        F: Fn(T) -> U + 'a,
    {
        let f = f.splice_fn1_ctx(&self.location).into();
        Singleton::new(
            self.location.clone(),
            HydroNode::Map {
                f,
                input: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata::<U>(),
            },
        )
    }

    pub fn flat_map_ordered<U, I, F>(
        self,
        f: impl IntoQuotedMut<'a, F, L>,
    ) -> Stream<U, L, B, TotalOrder, ExactlyOnce>
    where
        I: IntoIterator<Item = U>,
        F: Fn(T) -> I + 'a,
    {
        let f = f.splice_fn1_ctx(&self.location).into();
        Stream::new(
            self.location.clone(),
            HydroNode::FlatMap {
                f,
                input: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata::<U>(),
            },
        )
    }

    pub fn flat_map_unordered<U, I, F>(
        self,
        f: impl IntoQuotedMut<'a, F, L>,
    ) -> Stream<U, L, B, TotalOrder, ExactlyOnce>
    where
        I: IntoIterator<Item = U>,
        F: Fn(T) -> I + 'a,
    {
        let f = f.splice_fn1_ctx(&self.location).into();
        Stream::new(
            self.location.clone(),
            HydroNode::FlatMap {
                f,
                input: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata::<U>(),
            },
        )
    }

    pub fn filter<F>(self, f: impl IntoQuotedMut<'a, F, L>) -> Optional<T, L, B>
    where
        F: Fn(&T) -> bool + 'a,
    {
        let f = f.splice_fn1_borrow_ctx(&self.location).into();
        Optional::new(
            self.location.clone(),
            HydroNode::Filter {
                f,
                input: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata::<T>(),
            },
        )
    }

    pub fn filter_map<U, F>(self, f: impl IntoQuotedMut<'a, F, L>) -> Optional<U, L, B>
    where
        F: Fn(T) -> Option<U> + 'a,
    {
        let f = f.splice_fn1_ctx(&self.location).into();
        Optional::new(
            self.location.clone(),
            HydroNode::FilterMap {
                f,
                input: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata::<U>(),
            },
        )
    }

    pub fn zip<O>(self, other: O) -> <Self as ZipResult<'a, O>>::Out
    where
        Self: ZipResult<'a, O, Location = L>,
    {
        check_matching_location(&self.location, &Self::other_location(&other));

        if L::is_top_level() {
            let left_ir_node = self.ir_node.into_inner();
            let left_ir_node_metadata = left_ir_node.metadata().clone();
            let right_ir_node = Self::other_ir_node(other);
            let right_ir_node_metadata = right_ir_node.metadata().clone();

            Self::make(
                self.location.clone(),
                HydroNode::Persist {
                    inner: Box::new(HydroNode::CrossSingleton {
                        left: Box::new(HydroNode::Unpersist {
                            inner: Box::new(left_ir_node),
                            metadata: left_ir_node_metadata,
                        }),
                        right: Box::new(HydroNode::Unpersist {
                            inner: Box::new(right_ir_node),
                            metadata: right_ir_node_metadata,
                        }),
                        metadata: self
                            .location
                            .new_node_metadata::<<Self as ZipResult<'a, O>>::ElementType>(),
                    }),
                    metadata: self
                        .location
                        .new_node_metadata::<<Self as ZipResult<'a, O>>::ElementType>(),
                },
            )
        } else {
            Self::make(
                self.location.clone(),
                HydroNode::CrossSingleton {
                    left: Box::new(self.ir_node.into_inner()),
                    right: Box::new(Self::other_ir_node(other)),
                    metadata: self
                        .location
                        .new_node_metadata::<<Self as ZipResult<'a, O>>::ElementType>(),
                },
            )
        }
    }

    pub fn continue_if<U>(self, signal: Optional<U, L, Bounded>) -> Optional<T, L, Bounded>
    where
        Self: ZipResult<
                'a,
                Optional<(), L, Bounded>,
                Location = L,
                Out = Optional<(T, ()), L, Bounded>,
            >,
    {
        self.zip(signal.map(q!(|_u| ()))).map(q!(|(d, _signal)| d))
    }

    pub fn continue_unless<U>(self, other: Optional<U, L, Bounded>) -> Optional<T, L, Bounded>
    where
        Singleton<T, L, B>: ZipResult<
                'a,
                Optional<(), L, Bounded>,
                Location = L,
                Out = Optional<(T, ()), L, Bounded>,
            >,
    {
        self.continue_if(other.into_stream().count().filter(q!(|c| *c == 0)))
    }
}

impl<'a, T, L, B> Singleton<T, Atomic<L>, B>
where
    L: Location<'a> + NoTick,
{
    /// Returns a singleton value corresponding to the latest snapshot of the singleton
    /// being atomically processed. The snapshot at tick `t + 1` is guaranteed to include
    /// at least all relevant data that contributed to the snapshot at tick `t`.
    ///
    /// # Safety
    /// Because this picks a snapshot of a singleton whose value is continuously changing,
    /// the output singleton has a non-deterministic value since the snapshot can be at an
    /// arbitrary point in time.
    pub unsafe fn latest_tick(self) -> Singleton<T, Tick<L>, Bounded> {
        Singleton::new(
            self.location.clone().tick,
            HydroNode::Unpersist {
                inner: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata::<T>(),
            },
        )
    }

    pub fn end_atomic(self) -> Optional<T, L, B> {
        Optional::new(self.location.tick.l, self.ir_node.into_inner())
    }
}

impl<'a, T, L, B> Singleton<T, L, B>
where
    L: Location<'a> + NoTick + NoAtomic,
{
    pub fn atomic(self, tick: &Tick<L>) -> Singleton<T, Atomic<L>, B> {
        Singleton::new(Atomic { tick: tick.clone() }, self.ir_node.into_inner())
    }

    /// Given a tick, returns a singleton value corresponding to a snapshot of the singleton
    /// as of that tick. The snapshot at tick `t + 1` is guaranteed to include at least all
    /// relevant data that contributed to the snapshot at tick `t`.
    ///
    /// # Safety
    /// Because this picks a snapshot of a singleton whose value is continuously changing,
    /// the output singleton has a non-deterministic value since the snapshot can be at an
    /// arbitrary point in time.
    pub unsafe fn latest_tick(self, tick: &Tick<L>) -> Singleton<T, Tick<L>, Bounded>
    where
        L: NoTick,
    {
        unsafe { self.atomic(tick).latest_tick() }
    }

    /// Eagerly samples the singleton as fast as possible, returning a stream of snapshots
    /// with order corresponding to increasing prefixes of data contributing to the singleton.
    ///
    /// # Safety
    /// At runtime, the singleton will be arbitrarily sampled as fast as possible, but due
    /// to non-deterministic batching and arrival of inputs, the output stream is
    /// non-deterministic.
    pub unsafe fn sample_eager(self) -> Stream<T, L, Unbounded, TotalOrder, AtLeastOnce> {
        let tick = self.location.tick();

        unsafe {
            // SAFETY: source of intentional non-determinism
            self.latest_tick(&tick).all_ticks().weakest_retries()
        }
    }

    /// Given a time interval, returns a stream corresponding to snapshots of the singleton
    /// value taken at various points in time. Because the input singleton may be
    /// [`Unbounded`], there are no guarantees on what these snapshots are other than they
    /// represent the value of the singleton given some prefix of the streams leading up to
    /// it.
    ///
    /// # Safety
    /// The output stream is non-deterministic in which elements are sampled, since this
    /// is controlled by a clock.
    pub unsafe fn sample_every(
        self,
        interval: impl QuotedWithContext<'a, std::time::Duration, L> + Copy + 'a,
    ) -> Stream<T, L, Unbounded, TotalOrder, AtLeastOnce> {
        let samples = unsafe {
            // SAFETY: source of intentional non-determinism
            self.location.source_interval(interval)
        };
        let tick = self.location.tick();

        unsafe {
            // SAFETY: source of intentional non-determinism
            self.latest_tick(&tick)
                .continue_if(samples.tick_batch(&tick).first())
                .all_ticks()
                .weakest_retries()
        }
    }
}

impl<'a, T, L> Singleton<T, Tick<L>, Bounded>
where
    L: Location<'a>,
{
    pub fn all_ticks(self) -> Stream<T, L, Unbounded, TotalOrder, ExactlyOnce> {
        Stream::new(
            self.location.outer().clone(),
            HydroNode::Persist {
                inner: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata::<T>(),
            },
        )
    }

    pub fn all_ticks_atomic(self) -> Stream<T, Atomic<L>, Unbounded, TotalOrder, ExactlyOnce> {
        Stream::new(
            Atomic {
                tick: self.location.clone(),
            },
            HydroNode::Persist {
                inner: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata::<T>(),
            },
        )
    }

    pub fn latest(self) -> Singleton<T, L, Unbounded> {
        Singleton::new(
            self.location.outer().clone(),
            HydroNode::Persist {
                inner: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata::<T>(),
            },
        )
    }

    pub fn latest_atomic(self) -> Singleton<T, Atomic<L>, Unbounded> {
        Singleton::new(
            Atomic {
                tick: self.location.clone(),
            },
            HydroNode::Persist {
                inner: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata::<T>(),
            },
        )
    }

    pub fn defer_tick(self) -> Singleton<T, Tick<L>, Bounded> {
        Singleton::new(
            self.location.clone(),
            HydroNode::DeferTick {
                input: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata::<T>(),
            },
        )
    }

    pub fn persist(self) -> Stream<T, Tick<L>, Bounded, TotalOrder, ExactlyOnce> {
        Stream::new(
            self.location.clone(),
            HydroNode::Persist {
                inner: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata::<T>(),
            },
        )
    }

    pub fn delta(self) -> Optional<T, Tick<L>, Bounded> {
        Optional::new(
            self.location.clone(),
            HydroNode::Delta {
                inner: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata::<T>(),
            },
        )
    }

    pub fn into_stream(self) -> Stream<T, Tick<L>, Bounded, TotalOrder, ExactlyOnce> {
        Stream::new(self.location, self.ir_node.into_inner())
    }
}

pub trait ZipResult<'a, Other> {
    type Out;
    type ElementType;
    type Location;

    fn other_location(other: &Other) -> Self::Location;
    fn other_ir_node(other: Other) -> HydroNode;

    fn make(location: Self::Location, ir_node: HydroNode) -> Self::Out;
}

impl<'a, T, U, L, B> ZipResult<'a, Singleton<U, Tick<L>, B>> for Singleton<T, Tick<L>, B>
where
    U: Clone,
    L: Location<'a>,
{
    type Out = Singleton<(T, U), Tick<L>, B>;
    type ElementType = (T, U);
    type Location = Tick<L>;

    fn other_location(other: &Singleton<U, Tick<L>, B>) -> Tick<L> {
        other.location.clone()
    }

    fn other_ir_node(other: Singleton<U, Tick<L>, B>) -> HydroNode {
        other.ir_node.into_inner()
    }

    fn make(location: Tick<L>, ir_node: HydroNode) -> Self::Out {
        Singleton::new(location, ir_node)
    }
}

impl<'a, T, U, L, B> ZipResult<'a, Optional<U, Tick<L>, B>> for Singleton<T, Tick<L>, B>
where
    U: Clone,
    L: Location<'a>,
{
    type Out = Optional<(T, U), Tick<L>, B>;
    type ElementType = (T, U);
    type Location = Tick<L>;

    fn other_location(other: &Optional<U, Tick<L>, B>) -> Tick<L> {
        other.location.clone()
    }

    fn other_ir_node(other: Optional<U, Tick<L>, B>) -> HydroNode {
        other.ir_node.into_inner()
    }

    fn make(location: Tick<L>, ir_node: HydroNode) -> Self::Out {
        Optional::new(location, ir_node)
    }
}

#[cfg(test)]
mod tests {
    use futures::{SinkExt, StreamExt};
    use hydro_deploy::Deployment;
    use stageleft::q;

    use crate::{FlowBuilder, Location};

    #[tokio::test]
    async fn tick_cycle_cardinality() {
        let mut deployment = Deployment::new();

        let flow = FlowBuilder::new();
        let node = flow.process::<()>();
        let external = flow.external_process::<()>();

        let (input_send, input) = external.source_external_bincode(&node);

        let node_tick = node.tick();
        let (complete_cycle, singleton) = node_tick.cycle_with_initial(node_tick.singleton(q!(0)));
        let counts = singleton
            .clone()
            .into_stream()
            .count()
            .continue_if(unsafe { input.tick_batch(&node_tick).first() })
            .all_ticks()
            .send_bincode_external(&external);
        complete_cycle.complete_next_tick(singleton);

        let nodes = flow
            .with_process(&node, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let mut tick_trigger = nodes.connect_sink_bincode(input_send).await;
        let mut external_out = nodes.connect_source_bincode(counts).await;

        deployment.start().await.unwrap();

        tick_trigger.send(()).await.unwrap();

        assert_eq!(external_out.next().await.unwrap(), 1);

        tick_trigger.send(()).await.unwrap();

        assert_eq!(external_out.next().await.unwrap(), 1);
    }
}
