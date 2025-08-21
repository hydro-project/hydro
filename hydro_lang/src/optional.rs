use std::cell::RefCell;
use std::marker::PhantomData;
use std::ops::Deref;
use std::rc::Rc;

use stageleft::{IntoQuotedMut, QuotedWithContext, q};
use syn::parse_quote;

use crate::builder::FLOW_USED_MESSAGE;
use crate::cycle::{CycleCollection, CycleComplete, DeferTick, ForwardRefMarker, TickCycleMarker};
use crate::ir::{HydroLeaf, HydroNode, HydroSource, TeeNode};
use crate::location::tick::{Atomic, NoAtomic};
use crate::location::{LocationId, NoTick, check_matching_location};
use crate::singleton::ZipResult;
use crate::stream::{AtLeastOnce, ExactlyOnce, NoOrder};
use crate::unsafety::NonDet;
use crate::{Bounded, Location, Singleton, Stream, Tick, TotalOrder, Unbounded};

pub struct Optional<Type, Loc, Bound> {
    pub(crate) location: Loc,
    pub(crate) ir_node: RefCell<HydroNode>,

    _phantom: PhantomData<(Type, Loc, Bound)>,
}

impl<'a, T, L> DeferTick for Optional<T, Tick<L>, Bounded>
where
    L: Location<'a>,
{
    fn defer_tick(self) -> Self {
        Optional::defer_tick(self)
    }
}

impl<'a, T, L> CycleCollection<'a, TickCycleMarker> for Optional<T, Tick<L>, Bounded>
where
    L: Location<'a>,
{
    type Location = Tick<L>;

    fn create_source(ident: syn::Ident, location: Tick<L>) -> Self {
        Optional::new(
            location.clone(),
            HydroNode::CycleSource {
                ident,
                metadata: location.new_node_metadata::<T>(),
            },
        )
    }
}

impl<'a, T, L> CycleComplete<'a, TickCycleMarker> for Optional<T, Tick<L>, Bounded>
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
                input: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata::<T>(),
            });
    }
}

impl<'a, T, L> CycleCollection<'a, ForwardRefMarker> for Optional<T, Tick<L>, Bounded>
where
    L: Location<'a>,
{
    type Location = Tick<L>;

    fn create_source(ident: syn::Ident, location: Tick<L>) -> Self {
        Optional::new(
            location.clone(),
            HydroNode::CycleSource {
                ident,
                metadata: location.new_node_metadata::<T>(),
            },
        )
    }
}

impl<'a, T, L> CycleComplete<'a, ForwardRefMarker> for Optional<T, Tick<L>, Bounded>
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
                input: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata::<T>(),
            });
    }
}

impl<'a, T, L, B> CycleCollection<'a, ForwardRefMarker> for Optional<T, L, B>
where
    L: Location<'a> + NoTick,
{
    type Location = L;

    fn create_source(ident: syn::Ident, location: L) -> Self {
        Optional::new(
            location.clone(),
            HydroNode::Persist {
                inner: Box::new(HydroNode::CycleSource {
                    ident,
                    metadata: location.new_node_metadata::<T>(),
                }),
                metadata: location.new_node_metadata::<T>(),
            },
        )
    }
}

impl<'a, T, L, B> CycleComplete<'a, ForwardRefMarker> for Optional<T, L, B>
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
                input: Box::new(HydroNode::Unpersist {
                    inner: Box::new(self.ir_node.into_inner()),
                    metadata: metadata.clone(),
                }),
                metadata,
            });
    }
}

impl<'a, T, L> From<Optional<T, L, Bounded>> for Optional<T, L, Unbounded>
where
    L: Location<'a>,
{
    fn from(singleton: Optional<T, L, Bounded>) -> Self {
        Optional::new(singleton.location, singleton.ir_node.into_inner())
    }
}

impl<'a, T, L, B> From<Singleton<T, L, B>> for Optional<T, L, B>
where
    L: Location<'a>,
{
    fn from(singleton: Singleton<T, L, B>) -> Self {
        Optional::some(singleton)
    }
}

impl<'a, T, L, B> Clone for Optional<T, L, B>
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
            Optional {
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

impl<'a, T, L, B> Optional<T, L, B>
where
    L: Location<'a>,
{
    pub(crate) fn new(location: L, ir_node: HydroNode) -> Self {
        Optional {
            location,
            ir_node: RefCell::new(ir_node),
            _phantom: PhantomData,
        }
    }

    pub fn some(singleton: Singleton<T, L, B>) -> Self {
        Optional::new(singleton.location, singleton.ir_node.into_inner())
    }

    /// Transforms the optional value by applying a function `f` to it,
    /// continuously as the input is updated.
    ///
    /// Whenever the optional is empty, the output optional is also empty.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let optional = tick.optional_first_tick(q!(1));
    /// optional.map(q!(|v| v + 1)).all_ticks()
    /// # }, |mut stream| async move {
    /// // 2
    /// # assert_eq!(stream.next().await.unwrap(), 2);
    /// # }));
    /// ```
    pub fn map<U, F>(self, f: impl IntoQuotedMut<'a, F, L>) -> Optional<U, L, B>
    where
        F: Fn(T) -> U + 'a,
    {
        let f = f.splice_fn1_ctx(&self.location).into();
        Optional::new(
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
    ) -> Stream<U, L, B, NoOrder, ExactlyOnce>
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

    pub fn flatten_ordered<U>(self) -> Stream<U, L, B, TotalOrder, ExactlyOnce>
    where
        T: IntoIterator<Item = U>,
    {
        self.flat_map_ordered(q!(|v| v))
    }

    pub fn flatten_unordered<U>(self) -> Stream<U, L, B, NoOrder, ExactlyOnce>
    where
        T: IntoIterator<Item = U>,
    {
        self.flat_map_unordered(q!(|v| v))
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

    pub fn union(self, other: Optional<T, L, B>) -> Optional<T, L, B> {
        check_matching_location(&self.location, &other.location);

        if L::is_top_level() {
            Optional::new(
                self.location.clone(),
                HydroNode::Persist {
                    inner: Box::new(HydroNode::Chain {
                        first: Box::new(HydroNode::Unpersist {
                            inner: Box::new(self.ir_node.into_inner()),
                            metadata: self.location.new_node_metadata::<T>(),
                        }),
                        second: Box::new(HydroNode::Unpersist {
                            inner: Box::new(other.ir_node.into_inner()),
                            metadata: self.location.new_node_metadata::<T>(),
                        }),
                        metadata: self.location.new_node_metadata::<T>(),
                    }),
                    metadata: self.location.new_node_metadata::<T>(),
                },
            )
        } else {
            Optional::new(
                self.location.clone(),
                HydroNode::Chain {
                    first: Box::new(self.ir_node.into_inner()),
                    second: Box::new(other.ir_node.into_inner()),
                    metadata: self.location.new_node_metadata::<T>(),
                },
            )
        }
    }

    pub fn zip<O>(self, other: impl Into<Optional<O, L, B>>) -> Optional<(T, O), L, B>
    where
        O: Clone,
    {
        let other: Optional<O, L, B> = other.into();
        check_matching_location(&self.location, &other.location);

        if L::is_top_level() {
            Optional::new(
                self.location.clone(),
                HydroNode::Persist {
                    inner: Box::new(HydroNode::CrossSingleton {
                        left: Box::new(HydroNode::Unpersist {
                            inner: Box::new(self.ir_node.into_inner()),
                            metadata: self.location.new_node_metadata::<T>(),
                        }),
                        right: Box::new(HydroNode::Unpersist {
                            inner: Box::new(other.ir_node.into_inner()),
                            metadata: self.location.new_node_metadata::<O>(),
                        }),
                        metadata: self.location.new_node_metadata::<(T, O)>(),
                    }),
                    metadata: self.location.new_node_metadata::<(T, O)>(),
                },
            )
        } else {
            Optional::new(
                self.location.clone(),
                HydroNode::CrossSingleton {
                    left: Box::new(self.ir_node.into_inner()),
                    right: Box::new(other.ir_node.into_inner()),
                    metadata: self.location.new_node_metadata::<(T, O)>(),
                },
            )
        }
    }

    pub fn unwrap_or(self, other: Singleton<T, L, B>) -> Singleton<T, L, B> {
        check_matching_location(&self.location, &other.location);

        if L::is_top_level() {
            Singleton::new(
                self.location.clone(),
                HydroNode::Persist {
                    inner: Box::new(HydroNode::Chain {
                        first: Box::new(HydroNode::Unpersist {
                            inner: Box::new(self.ir_node.into_inner()),
                            metadata: self.location.new_node_metadata::<T>(),
                        }),
                        second: Box::new(HydroNode::Unpersist {
                            inner: Box::new(other.ir_node.into_inner()),
                            metadata: self.location.new_node_metadata::<T>(),
                        }),
                        metadata: self.location.new_node_metadata::<T>(),
                    }),
                    metadata: self.location.new_node_metadata::<T>(),
                },
            )
        } else {
            Singleton::new(
                self.location.clone(),
                HydroNode::Chain {
                    first: Box::new(self.ir_node.into_inner()),
                    second: Box::new(other.ir_node.into_inner()),
                    metadata: self.location.new_node_metadata::<T>(),
                },
            )
        }
    }

    pub fn into_singleton(self) -> Singleton<Option<T>, L, B>
    where
        T: Clone,
    {
        let none: syn::Expr = parse_quote!([::std::option::Option::None]);
        let core_ir = HydroNode::Persist {
            inner: Box::new(HydroNode::Source {
                source: HydroSource::Iter(none.into()),
                metadata: self.location.root().new_node_metadata::<Option<T>>(),
            }),
            metadata: self.location.new_node_metadata::<Option<T>>(),
        };

        let none_singleton = if L::is_top_level() {
            Singleton::new(
                self.location.clone(),
                HydroNode::Persist {
                    inner: Box::new(core_ir),
                    metadata: self.location.new_node_metadata::<Option<T>>(),
                },
            )
        } else {
            Singleton::new(self.location.clone(), core_ir)
        };

        self.map(q!(|v| Some(v))).unwrap_or(none_singleton)
    }

    /// An operator which allows you to "name" a `HydroNode`.
    /// This is only used for testing, to correlate certain `HydroNode`s with IDs.
    pub fn ir_node_named(self, name: &str) -> Optional<T, L, B> {
        {
            let mut node = self.ir_node.borrow_mut();
            let metadata = node.metadata_mut();
            metadata.tag = Some(name.to_string());
        }
        self
    }
}

impl<'a, T, L> Optional<T, L, Bounded>
where
    L: Location<'a>,
{
    pub fn continue_if<U>(self, signal: Optional<U, L, Bounded>) -> Optional<T, L, Bounded> {
        self.zip(signal.map(q!(|_u| ()))).map(q!(|(d, _signal)| d))
    }

    pub fn continue_unless<U>(self, other: Optional<U, L, Bounded>) -> Optional<T, L, Bounded> {
        self.continue_if(other.into_stream().count().filter(q!(|c| *c == 0)))
    }

    pub fn then<U>(self, value: Singleton<U, L, Bounded>) -> Optional<U, L, Bounded>
    where
        Singleton<U, L, Bounded>: ZipResult<
                'a,
                Optional<(), L, Bounded>,
                Location = L,
                Out = Optional<(U, ()), L, Bounded>,
            >,
    {
        value.continue_if(self)
    }

    pub fn into_stream(self) -> Stream<T, L, Bounded, TotalOrder, ExactlyOnce> {
        if L::is_top_level() {
            panic!("Converting an optional to a stream is not yet supported at the top level");
        }

        Stream::new(self.location, self.ir_node.into_inner())
    }
}

impl<'a, T, L, B> Optional<T, Atomic<L>, B>
where
    L: Location<'a> + NoTick,
{
    /// Returns an optional value corresponding to the latest snapshot of the optional
    /// being atomically processed. The snapshot at tick `t + 1` is guaranteed to include
    /// at least all relevant data that contributed to the snapshot at tick `t`.
    ///
    /// # Non-Determinism
    /// Because this picks a snapshot of a optional whose value is continuously changing,
    /// the output optional has a non-deterministic value since the snapshot can be at an
    /// arbitrary point in time.
    pub fn snapshot(self, _nondet: NonDet) -> Optional<T, Tick<L>, Bounded> {
        Optional::new(
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

impl<'a, T, L, B> Optional<T, L, B>
where
    L: Location<'a> + NoTick + NoAtomic,
{
    pub fn atomic(self, tick: &Tick<L>) -> Optional<T, Atomic<L>, B> {
        Optional::new(Atomic { tick: tick.clone() }, self.ir_node.into_inner())
    }

    /// Given a tick, returns a optional value corresponding to a snapshot of the optional
    /// as of that tick. The snapshot at tick `t + 1` is guaranteed to include at least all
    /// relevant data that contributed to the snapshot at tick `t`.
    ///
    /// # Non-Determinism
    /// Because this picks a snapshot of a optional whose value is continuously changing,
    /// the output optional has a non-deterministic value since the snapshot can be at an
    /// arbitrary point in time.
    pub fn snapshot(self, tick: &Tick<L>, nondet: NonDet) -> Optional<T, Tick<L>, Bounded> {
        self.atomic(tick).snapshot(nondet)
    }

    /// Eagerly samples the optional as fast as possible, returning a stream of snapshots
    /// with order corresponding to increasing prefixes of data contributing to the optional.
    ///
    /// # Non-Determinism
    /// At runtime, the optional will be arbitrarily sampled as fast as possible, but due
    /// to non-deterministic batching and arrival of inputs, the output stream is
    /// non-deterministic.
    pub fn sample_eager(self, nondet: NonDet) -> Stream<T, L, Unbounded, TotalOrder, AtLeastOnce> {
        let tick = self.location.tick();
        self.snapshot(&tick, nondet).all_ticks().weakest_retries()
    }

    /// Given a time interval, returns a stream corresponding to snapshots of the optional
    /// value taken at various points in time. Because the input optional may be
    /// [`Unbounded`], there are no guarantees on what these snapshots are other than they
    /// represent the value of the optional given some prefix of the streams leading up to
    /// it.
    ///
    /// # Non-Determinism
    /// The output stream is non-deterministic in which elements are sampled, since this
    /// is controlled by a clock.
    pub fn sample_every(
        self,
        interval: impl QuotedWithContext<'a, std::time::Duration, L> + Copy + 'a,
        nondet: NonDet,
    ) -> Stream<T, L, Unbounded, TotalOrder, AtLeastOnce> {
        let samples = self.location.source_interval(interval, nondet);
        let tick = self.location.tick();

        self.snapshot(&tick, nondet)
            .continue_if(samples.batch(&tick, nondet).first())
            .all_ticks()
            .weakest_retries()
    }
}

impl<'a, T, L> Optional<T, Tick<L>, Bounded>
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

    pub fn latest(self) -> Optional<T, L, Unbounded> {
        Optional::new(
            self.location.outer().clone(),
            HydroNode::Persist {
                inner: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata::<T>(),
            },
        )
    }

    pub fn latest_atomic(self) -> Optional<T, Atomic<L>, Unbounded> {
        Optional::new(
            Atomic {
                tick: self.location.clone(),
            },
            HydroNode::Persist {
                inner: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata::<T>(),
            },
        )
    }

    pub fn defer_tick(self) -> Optional<T, Tick<L>, Bounded> {
        Optional::new(
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
}
