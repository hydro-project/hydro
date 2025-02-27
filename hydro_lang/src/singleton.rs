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
use crate::{Bounded, Optional, Stream, Unbounded};

pub struct Singleton<T, L, B> {
    pub(crate) location: L,
    pub(crate) ir_node: RefCell<HydroNode>,

    _phantom: PhantomData<(T, L, B)>,
}

impl<'a, T, L: Location<'a>, B> Singleton<T, L, B> {
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

impl<'a, T, L: Location<'a>> From<Singleton<T, L, Bounded>> for Singleton<T, L, Unbounded> {
    fn from(singleton: Singleton<T, L, Bounded>) -> Self {
        Singleton::new(singleton.location, singleton.ir_node.into_inner())
    }
}

impl<'a, T, L: Location<'a>> DeferTick for Singleton<T, Tick<L>, Bounded> {
    fn defer_tick(self) -> Self {
        Singleton::defer_tick(self)
    }
}

impl<'a, T, L: Location<'a>> CycleCollectionWithInitial<'a, TickCycleMarker>
    for Singleton<T, Tick<L>, Bounded>
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
                second: initial.ir_node.into_inner().into(),
                metadata: location.new_node_metadata::<T>(),
            },
        )
    }
}

impl<'a, T, L: Location<'a>> CycleComplete<'a, TickCycleMarker> for Singleton<T, Tick<L>, Bounded> {
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

impl<'a, T, L: Location<'a>> CycleCollection<'a, ForwardRefMarker>
    for Singleton<T, Tick<L>, Bounded>
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

impl<'a, T, L: Location<'a>> CycleComplete<'a, ForwardRefMarker>
    for Singleton<T, Tick<L>, Bounded>
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

impl<'a, T, L: Location<'a> + NoTick, B> CycleCollection<'a, ForwardRefMarker>
    for Singleton<T, L, B>
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

impl<'a, T, L: Location<'a> + NoTick, B> CycleComplete<'a, ForwardRefMarker>
    for Singleton<T, L, B>
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

impl<'a, T: Clone, L: Location<'a>, B> Clone for Singleton<T, L, B> {
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

impl<'a, T, L: Location<'a>, B> Singleton<T, L, B> {
    pub fn map<U, F: Fn(T) -> U + 'a>(self, f: impl IntoQuotedMut<'a, F, L>) -> Singleton<U, L, B> {
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

    pub fn flat_map_ordered<U, I: IntoIterator<Item = U>, F: Fn(T) -> I + 'a>(
        self,
        f: impl IntoQuotedMut<'a, F, L>,
    ) -> Stream<U, L, B> {
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

    pub fn flat_map_unordered<U, I: IntoIterator<Item = U>, F: Fn(T) -> I + 'a>(
        self,
        f: impl IntoQuotedMut<'a, F, L>,
    ) -> Stream<U, L, B> {
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

    pub fn filter<F: Fn(&T) -> bool + 'a>(
        self,
        f: impl IntoQuotedMut<'a, F, L>,
    ) -> Optional<T, L, B> {
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

    pub fn filter_map<U, F: Fn(T) -> Option<U> + 'a>(
        self,
        f: impl IntoQuotedMut<'a, F, L>,
    ) -> Optional<U, L, B> {
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

    pub fn zip<Other>(self, other: Other) -> <Self as ZipResult<'a, Other>>::Out
    where
        Self: ZipResult<'a, Other, Location = L>,
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
                            .new_node_metadata::<<Self as ZipResult<'a, Other>>::ElementType>(),
                    }),
                    metadata: self
                        .location
                        .new_node_metadata::<<Self as ZipResult<'a, Other>>::ElementType>(),
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
                        .new_node_metadata::<<Self as ZipResult<'a, Other>>::ElementType>(),
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

impl<'a, T, L: Location<'a> + NoTick, B> Singleton<T, Atomic<L>, B> {
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

impl<'a, T, L: Location<'a> + NoTick + NoAtomic, B> Singleton<T, L, B> {
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
    pub unsafe fn sample_eager(self) -> Stream<T, L, Unbounded> {
        let tick = self.location.tick();

        unsafe {
            // SAFETY: source of intentional non-determinism
            self.latest_tick(&tick).all_ticks()
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
    ) -> Stream<T, L, Unbounded>
    where
        L: NoAtomic,
    {
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
        }
    }
}

impl<'a, T, L: Location<'a>> Singleton<T, Tick<L>, Bounded> {
    pub fn all_ticks(self) -> Stream<T, L, Unbounded> {
        Stream::new(
            self.location.outer().clone(),
            HydroNode::Persist {
                inner: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata::<T>(),
            },
        )
    }

    pub fn all_ticks_atomic(self) -> Stream<T, Atomic<L>, Unbounded> {
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

    pub fn persist(self) -> Stream<T, Tick<L>, Bounded> {
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

    pub fn into_stream(self) -> Stream<T, Tick<L>, Bounded> {
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

impl<'a, T, U: Clone, L: Location<'a>, B> ZipResult<'a, Singleton<U, Tick<L>, B>>
    for Singleton<T, Tick<L>, B>
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

impl<'a, T, U: Clone, L: Location<'a>, B> ZipResult<'a, Optional<U, Tick<L>, B>>
    for Singleton<T, Tick<L>, B>
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
