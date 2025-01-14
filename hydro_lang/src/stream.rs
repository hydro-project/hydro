use std::cell::RefCell;
use std::hash::Hash;
use std::marker::PhantomData;
use std::ops::Deref;
use std::rc::Rc;

use dfir_rs::bytes::Bytes;
use dfir_rs::futures;
use serde::de::DeserializeOwned;
use serde::Serialize;
use stageleft::{q, IntoQuotedMut, QuotedWithContext};
use syn::parse_quote;
use tokio::time::Instant;

use crate::builder::FLOW_USED_MESSAGE;
use crate::cycle::{CycleCollection, CycleComplete, DeferTick, ForwardRefMarker, TickCycleMarker};
use crate::ir::{DebugInstantiate, HydroLeaf, HydroNode, TeeNode};
use crate::location::cluster::CLUSTER_SELF_ID;
use crate::location::external_process::{ExternalBincodeStream, ExternalBytesPort};
use crate::location::tick::{NoTimestamp, Timestamped};
use crate::location::{
    check_matching_location, CanSend, ExternalProcess, Location, LocationId, NoTick, Tick,
};
use crate::staging_util::get_this_crate;
use crate::{Bounded, Cluster, ClusterId, Optional, Process, Singleton, Unbounded};

/// Marks the stream as being totally ordered, which means that there are
/// no sources of non-determinism (other than intentional ones) that will
/// affect the order of elements.
pub struct TotalOrder {}

/// Marks the stream as having no order, which means that the order of
/// elements may be affected by non-determinism.
///
/// This restricts certain operators, such as `fold` and `reduce`, to only
/// be used with commutative aggregation functions.
pub struct NoOrder {}

/// Helper trait for determining the weakest of two orderings.
#[sealed::sealed]
pub trait MinOrder<Other> {
    /// The weaker of the two orderings.
    type Min;
}

#[sealed::sealed]
impl<T> MinOrder<T> for T {
    type Min = T;
}

#[sealed::sealed]
impl MinOrder<NoOrder> for TotalOrder {
    type Min = NoOrder;
}

#[sealed::sealed]
impl MinOrder<TotalOrder> for NoOrder {
    type Min = NoOrder;
}

/// An ordered sequence stream of elements of type `T`.
///
/// Type Parameters:
/// - `T`: the type of elements in the stream
/// - `L`: the location where the stream is being materialized
/// - `B`: the boundedness of the stream, which is either [`Bounded`]
///   or [`Unbounded`]
/// - `Order`: the ordering of the stream, which is either [`TotalOrder`]
///   or [`NoOrder`] (default is [`TotalOrder`])
pub struct Stream<T, L, B, Order = TotalOrder> {
    location: L,
    pub(crate) ir_node: RefCell<HydroNode>,

    _phantom: PhantomData<(T, L, B, Order)>,
}

impl<'a, T, L: Location<'a>, B> From<Stream<T, L, B, TotalOrder>> for Stream<T, L, B, NoOrder> {
    fn from(stream: Stream<T, L, B, TotalOrder>) -> Stream<T, L, B, NoOrder> {
        Stream {
            location: stream.location,
            ir_node: stream.ir_node,
            _phantom: PhantomData,
        }
    }
}

impl<'a, T, L: Location<'a>, B, Order> Stream<T, L, B, Order> {
    fn location_kind(&self) -> LocationId {
        self.location.id()
    }
}

impl<'a, T, L: Location<'a>, Order> DeferTick for Stream<T, Tick<L>, Bounded, Order> {
    fn defer_tick(self) -> Self {
        Stream::defer_tick(self)
    }
}

impl<'a, T, L: Location<'a>, Order> CycleCollection<'a, TickCycleMarker>
    for Stream<T, Tick<L>, Bounded, Order>
{
    type Location = Tick<L>;

    fn create_source(ident: syn::Ident, location: Tick<L>) -> Self {
        let location_id = location.id();
        Stream::new(
            location,
            HydroNode::CycleSource {
                ident,
                location_kind: location_id,
            },
        )
    }
}

impl<'a, T, L: Location<'a>, Order> CycleComplete<'a, TickCycleMarker>
    for Stream<T, Tick<L>, Bounded, Order>
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
            });
    }
}

impl<'a, T, L: Location<'a> + NoTick, B, Order> CycleCollection<'a, ForwardRefMarker>
    for Stream<T, L, B, Order>
{
    type Location = L;

    fn create_source(ident: syn::Ident, location: L) -> Self {
        let location_id = location.id();
        Stream::new(
            location,
            HydroNode::Persist(Box::new(HydroNode::CycleSource {
                ident,
                location_kind: location_id,
            })),
        )
    }
}

impl<'a, T, L: Location<'a> + NoTick, B, Order> CycleComplete<'a, ForwardRefMarker>
    for Stream<T, L, B, Order>
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
                input: Box::new(HydroNode::Unpersist(Box::new(self.ir_node.into_inner()))),
            });
    }
}

impl<'a, T, L: Location<'a>, B, Order> Stream<T, L, B, Order> {
    pub(crate) fn new(location: L, ir_node: HydroNode) -> Self {
        Stream {
            location,
            ir_node: RefCell::new(ir_node),
            _phantom: PhantomData,
        }
    }
}

impl<'a, T: Clone, L: Location<'a>, B, Order> Clone for Stream<T, L, B, Order> {
    fn clone(&self) -> Self {
        if !matches!(self.ir_node.borrow().deref(), HydroNode::Tee { .. }) {
            let orig_ir_node = self.ir_node.replace(HydroNode::Placeholder);
            *self.ir_node.borrow_mut() = HydroNode::Tee {
                inner: TeeNode(Rc::new(RefCell::new(orig_ir_node))),
            };
        }

        if let HydroNode::Tee { inner } = self.ir_node.borrow().deref() {
            Stream {
                location: self.location.clone(),
                ir_node: HydroNode::Tee {
                    inner: TeeNode(inner.0.clone()),
                }
                .into(),
                _phantom: PhantomData,
            }
        } else {
            unreachable!()
        }
    }
}

impl<'a, T, L: Location<'a>, B, Order> Stream<T, L, B, Order> {
    /// Takes a closure and produces a stream based on invoking that closure on each element in order.
    /// If you do not want to modify the stream and instead only want to view
    /// each item use the [`inspect`](#inspect) operator instead.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::*;
    /// # use dfir_rs::futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test(|process| {
    /// let words = process.source_iter(q!(vec!["hello", "world"]));
    /// let mapped = words.map(q!(|x| x.to_uppercase()));
    /// # mapped
    /// # }, |mut stream| async move {
    /// // HELLO, WORLD
    /// # for w in vec!["HELLO", "WORLD"] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// ```
    pub fn map<U, F: Fn(T) -> U + 'a>(
        self,
        f: impl IntoQuotedMut<'a, F, L>,
    ) -> Stream<U, L, B, Order> {
        let f = f.splice_fn1_ctx(&self.location).into();
        Stream::new(
            self.location,
            HydroNode::Map {
                f,
                input: Box::new(self.ir_node.into_inner()),
            },
        )
    }

    /// For each item passed in, return a clone of the item; akin to `map(q!(|d| d.clone()))`.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::*;
    /// # use dfir_rs::futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test(|process| {
    /// let cloned = process.source_iter(q!(vec![1..3])).cloned();
    /// # cloned
    /// # }, |mut stream| async move {
    /// // 1, 2, 3
    /// # for w in vec![1..3] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// ```
    pub fn cloned(self) -> Stream<T, L, B, Order>
    where
        T: Clone,
    {
        self.map(q!(|d| d.clone()))
    }

    /// For each item `i` in the input stream, treat `i` as an iterator and map the closure to that
    /// iterator to produce items one by one. The type of the input items must be iterable.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::*;
    /// # use dfir_rs::futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test(|process| {
    /// process
    ///     .source_iter(q!(vec![vec![1, 2], vec![3, 4]]))
    ///     .flat_map_ordered(q!(|x| x.into_iter()))
    /// # }, |mut stream| async move {
    /// # // 1, 2, 3, 4
    /// # for w in (1..5) {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// ```
    pub fn flat_map_ordered<U, I: IntoIterator<Item = U>, F: Fn(T) -> I + 'a>(
        self,
        f: impl IntoQuotedMut<'a, F, L>,
    ) -> Stream<U, L, B, Order> {
        let f = f.splice_fn1_ctx(&self.location).into();
        Stream::new(
            self.location,
            HydroNode::FlatMap {
                f,
                input: Box::new(self.ir_node.into_inner()),
            },
        )
    }

    /// Like [`flat_map_ordered`](#flat_map_ordered), but allows the closure to return items in any order -- even
    /// non-deterministic order.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::*;
    /// # use dfir_rs::futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test::<_, _, NoOrder>(|process| {
    /// process
    ///     .source_iter(q!(if std::process::id() % 2 == 0 {
    ///         vec![vec![1, 2], vec![3, 4]]
    ///     } else {
    ///         vec![vec![3, 4], vec![1, 2]]
    ///     }))
    ///     .flat_map_unordered(q!(|x| x.into_iter()))
    /// # }, |mut stream| async move {
    /// # // 1, 2, 3, 4, but in no particular order
    /// # let mut results = Vec::new();
    /// # for w in (1..5) {
    /// #     results.push(stream.next().await.unwrap());
    /// # }
    /// # results.sort();
    /// # assert_eq!(results, vec![1, 2, 3, 4]);
    /// # }));
    /// ```
    pub fn flat_map_unordered<U, I: IntoIterator<Item = U>, F: Fn(T) -> I + 'a>(
        self,
        f: impl IntoQuotedMut<'a, F, L>,
    ) -> Stream<U, L, B, NoOrder> {
        let f = f.splice_fn1_ctx(&self.location).into();
        Stream::new(
            self.location,
            HydroNode::FlatMap {
                f,
                input: Box::new(self.ir_node.into_inner()),
            },
        )
    }

    /// For each item `i` in the input stream, treat `i` as an iterator and produce its items one by one.
    /// The type of the input items must be iterable.
    /// ```rust
    /// # use hydro_lang::*;
    /// # use dfir_rs::futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test(|process| {
    /// process
    ///     .source_iter(q!(vec![vec![1, 2], vec![3, 4]]))
    ///     .flatten_ordered()
    /// # }, |mut stream| async move {
    /// # // 1, 2, 3, 4
    /// # for w in (1..5) {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// ```
    pub fn flatten_ordered<U>(self) -> Stream<U, L, B, Order>
    where
        T: IntoIterator<Item = U>,
    {
        self.flat_map_ordered(q!(|d| d))
    }

    /// Like [`flatten_ordered`](#flatten_ordered), but allows the `IntoIter` implementation of (either outer or inner) collections to return things in any order.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::*;
    /// # use dfir_rs::futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test::<_, _, NoOrder>(|process| {
    /// process
    ///     .source_iter(q!(std::collections::HashSet::<Vec<u16>>::from_iter([vec![1, 2], vec![3, 4]])))
    ///     .flatten_unordered()
    /// # }, |mut stream| async move {
    /// # // 1, 2, 3, 4, but in no particular order
    /// # let mut results = Vec::new();
    /// # for w in (1..5) {
    /// #     results.push(stream.next().await.unwrap());
    /// # }
    /// # results.sort();
    /// # assert_eq!(results, vec![1, 2, 3, 4]);
    /// # }));
    pub fn flatten_unordered<U>(self) -> Stream<U, L, B, NoOrder>
    where
        T: IntoIterator<Item = U>,
    {
        self.flat_map_unordered(q!(|d| d))
    }

    /// Filter outputs a subsequence of the items it receives at its input, according to a
    /// Rust boolean closure passed in as an argument.
    ///
    /// The closure receives a reference `&T` rather than an owned value `T` because filtering does
    /// not modify or take ownership of the values. If you need to modify the values while filtering
    /// use [`filter_map`](#filter_map) instead.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::*;
    /// # use dfir_rs::futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test(|process| {
    /// process
    ///     .source_iter(q!(vec![1, 2, 3, 4]))
    ///     .filter(q!(|&x| x > 2))
    /// # }, |mut stream| async move {
    /// # // 3, 4
    /// # for w in (3..5) {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// ```
    pub fn filter<F: Fn(&T) -> bool + 'a>(
        self,
        f: impl IntoQuotedMut<'a, F, L>,
    ) -> Stream<T, L, B, Order> {
        let f = f.splice_fn1_borrow_ctx(&self.location).into();
        Stream::new(
            self.location,
            HydroNode::Filter {
                f,
                input: Box::new(self.ir_node.into_inner()),
            },
        )
    }

    /// An operator that both filters and maps. It yields only the items for which the supplied closure returns `Some(value)`.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::*;
    /// # use dfir_rs::futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test(|process| {
    /// process
    ///     .source_iter(q!(vec!["1", "hello", "world", "2"]))
    ///     .filter_map(q!(|s| s.parse::<usize>().ok()))
    /// # }, |mut stream| async move {
    /// # // 1, 2
    /// # for w in (1..3) {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    pub fn filter_map<U, F: Fn(T) -> Option<U> + 'a>(
        self,
        f: impl IntoQuotedMut<'a, F, L>,
    ) -> Stream<U, L, B, Order> {
        let f = f.splice_fn1_ctx(&self.location).into();
        Stream::new(
            self.location,
            HydroNode::FilterMap {
                f,
                input: Box::new(self.ir_node.into_inner()),
            },
        )
    }

    /// combine each element of type `T` from the stream with a singleton value of type `O`
    /// to produce a stream of pairs `(T, O)`. Both the stream and the singleton need to be `Bounded`.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::*;
    /// # use dfir_rs::futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let batch = unsafe {
    ///     process
    ///         .source_iter(q!(vec![1, 2, 3, 4]))
    ///         .timestamped(&tick)
    ///         .tick_batch()
    /// };
    /// let count = batch.clone().count();
    /// batch.cross_singleton(count).all_ticks().drop_timestamp()
    ///
    /// # }, |mut stream| async move {
    /// # // (1, 4) and (2, 4)
    /// # for w in vec![(1, 4), (2, 4), (3, 4), (4, 4)] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    pub fn cross_singleton<O>(
        self,
        other: impl Into<Optional<O, L, Bounded>>,
    ) -> Stream<(T, O), L, B, Order>
    where
        O: Clone,
    {
        let other: Optional<O, L, Bounded> = other.into();
        check_matching_location(&self.location, &other.location);

        Stream::new(
            self.location,
            HydroNode::CrossSingleton(
                Box::new(self.ir_node.into_inner()),
                Box::new(other.ir_node.into_inner()),
            ),
        )
    }

    /// Allow this stream through if the argument (a Bounded Optional) is non-empty, otherwise the output is empty.
    pub fn continue_if<U>(self, signal: Optional<U, L, Bounded>) -> Stream<T, L, B, Order> {
        self.cross_singleton(signal.map(q!(|_u| ())))
            .map(q!(|(d, _signal)| d))
    }

    /// Allow this stream through if the other stream is empty, otherwise the output is empty.
    pub fn continue_unless<U>(self, other: Optional<U, L, Bounded>) -> Stream<T, L, B, Order> {
        self.continue_if(other.into_stream().count().filter(q!(|c| *c == 0)))
    }

    /// Forms the cross-product (Cartesian product, cross-join) of the items in the 2 input streams, returning all
    /// tupled pairs.
    pub fn cross_product<O>(self, other: Stream<O, L, B, Order>) -> Stream<(T, O), L, B, Order>
    where
        T: Clone,
        O: Clone,
    {
        check_matching_location(&self.location, &other.location);

        Stream::new(
            self.location,
            HydroNode::CrossProduct(
                Box::new(self.ir_node.into_inner()),
                Box::new(other.ir_node.into_inner()),
            ),
        )
    }

    /// Takes one stream as input and filters out any duplicate occurrences. The output
    /// contains all unique values from the input.
    pub fn unique(self) -> Stream<T, L, B, Order>
    where
        T: Eq + Hash,
    {
        Stream::new(
            self.location,
            HydroNode::Unique(Box::new(self.ir_node.into_inner())),
        )
    }

    /// outputs everything in this stream that is *not*
    /// contained in the other (bounded) stream.
    pub fn filter_not_in<O2>(self, other: Stream<T, L, Bounded, O2>) -> Stream<T, L, Bounded, Order>
    where
        T: Eq + Hash,
    {
        check_matching_location(&self.location, &other.location);

        Stream::new(
            self.location,
            HydroNode::Difference(
                Box::new(self.ir_node.into_inner()),
                Box::new(other.ir_node.into_inner()),
            ),
        )
    }

    /// An operator which allows you to "inspect" each element of a stream without
    /// modifying it. The closure is called on a reference to each item. This is
    /// mainly useful for debugging as in the example below, and it is generally an
    /// anti-pattern to provide a closure with side effects.
    pub fn inspect<F: Fn(&T) + 'a>(
        self,
        f: impl IntoQuotedMut<'a, F, L>,
    ) -> Stream<T, L, B, Order> {
        let f = f.splice_fn1_borrow_ctx(&self.location).into();

        if L::is_top_level() {
            Stream::new(
                self.location,
                HydroNode::Persist(Box::new(HydroNode::Inspect {
                    f,
                    input: Box::new(HydroNode::Unpersist(Box::new(self.ir_node.into_inner()))),
                })),
            )
        } else {
            Stream::new(
                self.location,
                HydroNode::Inspect {
                    f,
                    input: Box::new(self.ir_node.into_inner()),
                },
            )
        }
    }

    /// Explicitly "casts" the stream to a type with a different ordering
    /// guarantee. Useful in unsafe code where the ordering cannot be proven
    /// by the type-system.
    ///
    /// # Safety
    /// This function is used as an escape hatch, and any mistakes in the
    /// provided ordering guarantee will propogate into the guarantees
    /// for the rest of the program.
    pub unsafe fn assume_ordering<O>(self) -> Stream<T, L, B, O> {
        Stream::new(self.location, self.ir_node.into_inner())
    }
}

impl<'a, T, L: Location<'a>, B, Order> Stream<T, L, B, Order>
where
    Order: MinOrder<NoOrder, Min = NoOrder>,
{
    /// > Arguments: two arguments, both closures. The first closure is used to create the initial
    /// > value for the accumulator, and the second is used to combine new items with the existing
    /// > accumulator value. The second closure takes two two arguments: an `&mut Accum` accumulated
    /// > value, and an `Item`.
    ///
    /// > Note: The second (combining) closure must be commutative, as the order of input items is not guaranteed.
    ///
    /// Akin to Rust's built-in [`fold`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.fold)
    /// operator, except that it takes the accumulator by `&mut` instead of by value. Folds every item
    /// into an accumulator by applying a closure, returning the final result.
    pub fn fold_commutative<A, I: Fn() -> A + 'a, F: Fn(&mut A, T)>(
        self,
        init: impl IntoQuotedMut<'a, I, L>,
        comb: impl IntoQuotedMut<'a, F, L>,
    ) -> Singleton<A, L, B> {
        let init = init.splice_fn0_ctx(&self.location).into();
        let comb = comb.splice_fn2_borrow_mut_ctx(&self.location).into();

        let mut core = HydroNode::Fold {
            init,
            acc: comb,
            input: Box::new(self.ir_node.into_inner()),
        };

        if L::is_top_level() {
            // top-level (possibly unbounded) singletons are represented as
            // a stream which produces all values from all ticks every tick,
            // so Unpersist will always give the lastest aggregation
            core = HydroNode::Persist(Box::new(core));
        }

        Singleton::new(self.location, core)
    }

    /// > Arguments: a closure which itself takes two arguments:
    /// > an `&mut Accum` accumulator mutable reference, and an `Item`. The closure should merge the item
    /// > into the accumulator.
    ///
    /// > Note: The closure must be commutative, as the order of input items is not guaranteed.
    ///
    /// Akin to Rust's built-in [`reduce`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.reduce)
    /// operator, except that it takes the accumulator by `&mut` instead of by value. Reduces every
    /// item into an accumulator by applying a closure, returning the final result.
    pub fn reduce_commutative<F: Fn(&mut T, T) + 'a>(
        self,
        comb: impl IntoQuotedMut<'a, F, L>,
    ) -> Optional<T, L, B> {
        let f = comb.splice_fn2_borrow_mut_ctx(&self.location).into();
        let mut core = HydroNode::Reduce {
            f,
            input: Box::new(self.ir_node.into_inner()),
        };

        if L::is_top_level() {
            core = HydroNode::Persist(Box::new(core));
        }

        Optional::new(self.location, core)
    }

    /// produces a singleton, namely the maximum value in the stream.
    pub fn max(self) -> Optional<T, L, B>
    where
        T: Ord,
    {
        self.reduce_commutative(q!(|curr, new| {
            if new > *curr {
                *curr = new;
            }
        }))
    }

    /// given a closure that produces a "key" from each item in the stream, produces a singleton,
    /// namely that value in the stream with the maximum value produced by the key closure.
    ///
    /// Typical usage:
    /// `max_by_key(q!(|t| t.0))`
    pub fn max_by_key<K: Ord, F: Fn(&T) -> K + 'a>(
        self,
        key: impl IntoQuotedMut<'a, F, L> + Copy,
    ) -> Optional<T, L, B> {
        let f = key.splice_fn1_borrow_ctx(&self.location);

        let wrapped: syn::Expr = parse_quote!({
            let key_fn = #f;
            move |curr, new| {
                if key_fn(&new) > key_fn(&*curr) {
                    *curr = new;
                }
            }
        });

        let mut core = HydroNode::Reduce {
            f: wrapped.into(),
            input: Box::new(self.ir_node.into_inner()),
        };

        if L::is_top_level() {
            core = HydroNode::Persist(Box::new(core));
        }

        Optional::new(self.location, core)
    }

    /// produces a singleton, namely the minimum value in the stream.
    pub fn min(self) -> Optional<T, L, B>
    where
        T: Ord,
    {
        self.reduce_commutative(q!(|curr, new| {
            if new < *curr {
                *curr = new;
            }
        }))
    }

    /// produces a singleton, namely the count of elements in the stream.
    pub fn count(self) -> Singleton<usize, L, B> {
        self.fold_commutative(q!(|| 0usize), q!(|count, _| *count += 1))
    }
}

impl<'a, T, L: Location<'a>, B> Stream<T, L, B, TotalOrder> {
    pub fn enumerate(self) -> Stream<(usize, T), L, B, TotalOrder> {
        if L::is_top_level() {
            Stream::new(
                self.location,
                HydroNode::Persist(Box::new(HydroNode::Enumerate {
                    is_static: true,
                    input: Box::new(HydroNode::Unpersist(Box::new(self.ir_node.into_inner()))),
                })),
            )
        } else {
            Stream::new(
                self.location,
                HydroNode::Enumerate {
                    is_static: false,
                    input: Box::new(self.ir_node.into_inner()),
                },
            )
        }
    }

    /// produces a singleton, namely the first value in the stream.
    pub fn first(self) -> Optional<T, L, B> {
        Optional::new(self.location, self.ir_node.into_inner())
    }

    /// produces a singleton, namely the last value in the stream.
    pub fn last(self) -> Optional<T, L, B> {
        self.reduce(q!(|curr, new| *curr = new))
    }

    /// > Arguments: two arguments, both closures. The first closure is used to create the initial
    /// > value for the accumulator, and the second is used to combine new items with the existing
    /// > accumulator value. The second closure takes two two arguments: an `&mut Accum` accumulated
    /// > value, and an `Item`.
    ///
    /// Akin to Rust's built-in [`fold`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.fold)
    /// operator, except that it takes the accumulator by `&mut` instead of by value. Folds every item
    /// into an accumulator by applying a closure, returning the final result.
    pub fn fold<A, I: Fn() -> A + 'a, F: Fn(&mut A, T)>(
        self,
        init: impl IntoQuotedMut<'a, I, L>,
        comb: impl IntoQuotedMut<'a, F, L>,
    ) -> Singleton<A, L, B> {
        let init = init.splice_fn0_ctx(&self.location).into();
        let comb = comb.splice_fn2_borrow_mut_ctx(&self.location).into();

        let mut core = HydroNode::Fold {
            init,
            acc: comb,
            input: Box::new(self.ir_node.into_inner()),
        };

        if L::is_top_level() {
            // top-level (possibly unbounded) singletons are represented as
            // a stream which produces all values from all ticks every tick,
            // so Unpersist will always give the lastest aggregation
            core = HydroNode::Persist(Box::new(core));
        }

        Singleton::new(self.location, core)
    }

    /// > Arguments: a closure which itself takes two arguments:
    /// > an `&mut Accum` accumulator mutable reference, and an `Item`. The closure should merge the item
    /// > into the accumulator.
    ///
    /// Akin to Rust's built-in [`reduce`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.reduce)
    /// operator, except that it takes the accumulator by `&mut` instead of by value. Reduces every
    /// item into an accumulator by applying a closure, returning the final result.
    pub fn reduce<F: Fn(&mut T, T) + 'a>(
        self,
        comb: impl IntoQuotedMut<'a, F, L>,
    ) -> Optional<T, L, B> {
        let f = comb.splice_fn2_borrow_mut_ctx(&self.location).into();
        let mut core = HydroNode::Reduce {
            f,
            input: Box::new(self.ir_node.into_inner()),
        };

        if L::is_top_level() {
            core = HydroNode::Persist(Box::new(core));
        }

        Optional::new(self.location, core)
    }
}

/// Akin to Rust's built-in [`chain`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.chain)
/// operator, takes two Bounded streams, and creates a new stream over both in sequence.
impl<'a, T, L: Location<'a>> Stream<T, L, Bounded, TotalOrder> {
    pub fn chain(
        self,
        other: Stream<T, L, Bounded, TotalOrder>,
    ) -> Stream<T, L, Bounded, TotalOrder> {
        check_matching_location(&self.location, &other.location);

        Stream::new(
            self.location,
            HydroNode::Chain(
                Box::new(self.ir_node.into_inner()),
                Box::new(other.ir_node.into_inner()),
            ),
        )
    }
}

/// Produces a new stream that interleaves the elements of the two input streams.
/// The result has NoOrder because the order of interleaving is not guaranteed.
impl<'a, T, L: Location<'a> + NoTick + NoTimestamp> Stream<T, L, Unbounded, NoOrder> {
    pub fn union(
        self,
        other: Stream<T, L, Unbounded, NoOrder>,
    ) -> Stream<T, L, Unbounded, NoOrder> {
        let tick = self.location.tick();
        unsafe {
            // SAFETY: Because the inputs and outputs are unordered,
            // we can interleave batches from both streams.
            self.timestamped(&tick)
                .tick_batch()
                .union(other.timestamped(&tick).tick_batch())
                .all_ticks()
                .drop_timestamp()
        }
    }
}

impl<'a, T, L: Location<'a>, Order> Stream<T, L, Bounded, Order> {
    /// takes a Bounded stream of elements with an Ord implementation
    /// and produces a new stream with the elements sorted in ascending order.
    pub fn sort(self) -> Stream<T, L, Bounded, TotalOrder>
    where
        T: Ord,
    {
        Stream::new(
            self.location,
            HydroNode::Sort(Box::new(self.ir_node.into_inner())),
        )
    }

    /// given a bounded input stream and another ordered input stream, produces a new stream
    /// with the elements of the first stream followed by the elements of the second stream in order
    pub fn union<B2, O2>(self, other: Stream<T, L, B2, O2>) -> Stream<T, L, B2, Order::Min>
    where
        Order: MinOrder<O2>,
    {
        check_matching_location(&self.location, &other.location);

        Stream::new(
            self.location,
            HydroNode::Chain(
                Box::new(self.ir_node.into_inner()),
                Box::new(other.ir_node.into_inner()),
            ),
        )
    }
}

/// Given two streams of pairs `(K, V1)` and `(K, V2)`, produces a new stream of nested pairs `(K, (V1, V2))`
/// by equi-joining the two streams on the key attribute `K`.
impl<'a, K, V1, L: Location<'a>, B, Order> Stream<(K, V1), L, B, Order> {
    pub fn join<V2, O2>(self, n: Stream<(K, V2), L, B, O2>) -> Stream<(K, (V1, V2)), L, B, NoOrder>
    where
        K: Eq + Hash,
    {
        check_matching_location(&self.location, &n.location);

        Stream::new(
            self.location,
            HydroNode::Join(
                Box::new(self.ir_node.into_inner()),
                Box::new(n.ir_node.into_inner()),
            ),
        )
    }

    /// Given two streams of pairs `(K, V1)` and `(K, V2)`,
    /// computes the anti-join of the items in the input -- i.e. returns
    /// unique items in the first input that do not have a matching key
    /// in the second input.
    pub fn anti_join<O2>(self, n: Stream<K, L, Bounded, O2>) -> Stream<(K, V1), L, B, Order>
    where
        K: Eq + Hash,
    {
        check_matching_location(&self.location, &n.location);

        Stream::new(
            self.location,
            HydroNode::AntiJoin(
                Box::new(self.ir_node.into_inner()),
                Box::new(n.ir_node.into_inner()),
            ),
        )
    }
}

impl<'a, K: Eq + Hash, V, L: Location<'a>> Stream<(K, V), Tick<L>, Bounded> {
    /// A special case of `fold`, in the spirit of SQL's GROUP BY and aggregation constructs. The input
    /// is partitioned into groups by the first field ("keys"), and for each group the values in the second
    /// field are accumulated via the closures in the arguments.
    ///
    /// If the input and output value types are the same and do not require initialization then use
    /// [`reduce_keyed`](#reduce_keyed).
    ///
    /// > Arguments: two Rust closures. The first generates an initial value per group. The second
    /// > itself takes two arguments: an 'accumulator', and an element. The second closure returns the
    /// > value that the accumulator should have for the next iteration.
    pub fn fold_keyed<A, I: Fn() -> A + 'a, F: Fn(&mut A, V) + 'a>(
        self,
        init: impl IntoQuotedMut<'a, I, Tick<L>>,
        comb: impl IntoQuotedMut<'a, F, Tick<L>>,
    ) -> Stream<(K, A), Tick<L>, Bounded> {
        let init = init.splice_fn0_ctx(&self.location).into();
        let comb = comb.splice_fn2_borrow_mut_ctx(&self.location).into();

        Stream::new(
            self.location,
            HydroNode::FoldKeyed {
                init,
                acc: comb,
                input: Box::new(self.ir_node.into_inner()),
            },
        )
    }

    /// A special case of `reduce`, in the spirit of SQL's GROUP BY and aggregation constructs. The input
    /// is partitioned into groups by the first field, and for each group the values in the second
    /// field are accumulated via the closures in the arguments.
    ///
    /// If you need the accumulated value to have a different type than the input, use [`fold_keyed`](#keyed_fold).
    ///
    /// > Arguments: one Rust closures. The closure takes two arguments: an `&mut` 'accumulator', and
    /// > an element. Accumulator should be updated based on the element.
    pub fn reduce_keyed<F: Fn(&mut V, V) + 'a>(
        self,
        comb: impl IntoQuotedMut<'a, F, Tick<L>>,
    ) -> Stream<(K, V), Tick<L>, Bounded> {
        let f = comb.splice_fn2_borrow_mut_ctx(&self.location).into();

        Stream::new(
            self.location,
            HydroNode::ReduceKeyed {
                f,
                input: Box::new(self.ir_node.into_inner()),
            },
        )
    }
}

impl<'a, K: Eq + Hash, V, L: Location<'a>, Order> Stream<(K, V), Tick<L>, Bounded, Order> {
    /// A special case of `fold`, in the spirit of SQL's GROUP BY and aggregation constructs. The input
    /// is partitioned into groups by the first field ("keys"), and for each group the values in the second
    /// field are accumulated via the closures in the arguments.
    ///
    /// If the input and output value types are the same and do not require initialization then use
    /// [`reduce_keyed`](#reduce_keyed).
    ///
    /// > Arguments: two Rust closures. The first generates an initial value per group. The second
    /// > itself takes two arguments: an 'accumulator', and an element. The second closure returns the
    /// > value that the accumulator should have for the next iteration.
    ///
    /// > Note: The second (combining) closure must be commutative, as the order of input items is not guaranteed.
    pub fn fold_keyed_commutative<A, I: Fn() -> A + 'a, F: Fn(&mut A, V) + 'a>(
        self,
        init: impl IntoQuotedMut<'a, I, Tick<L>>,
        comb: impl IntoQuotedMut<'a, F, Tick<L>>,
    ) -> Stream<(K, A), Tick<L>, Bounded, Order> {
        let init = init.splice_fn0_ctx(&self.location).into();
        let comb = comb.splice_fn2_borrow_mut_ctx(&self.location).into();

        Stream::new(
            self.location,
            HydroNode::FoldKeyed {
                init,
                acc: comb,
                input: Box::new(self.ir_node.into_inner()),
            },
        )
    }

    /// Given a stream of pairs `(K, V)`, produces a new stream of unique keys `K`.
    pub fn keys(self) -> Stream<K, Tick<L>, Bounded, Order> {
        self.fold_keyed_commutative(q!(|| ()), q!(|_, _| {}))
            .map(q!(|(k, _)| k))
    }

    /// A special case of `reduce`, in the spirit of SQL's GROUP BY and aggregation constructs. The input
    /// is partitioned into groups by the first field, and for each group the values in the second
    /// field are accumulated via the closures in the arguments.
    ///
    /// If you need the accumulated value to have a different type than the input, use [`fold_keyed`](#keyed_fold).
    ///
    /// > Arguments: one Rust closures. The closure takes two arguments: an `&mut` 'accumulator', and
    /// > an element. Accumulator should be updated based on the element.
    ///
    /// > Note: The closure must be commutative, as the order of input items is not guaranteed.
    pub fn reduce_keyed_commutative<F: Fn(&mut V, V) + 'a>(
        self,
        comb: impl IntoQuotedMut<'a, F, Tick<L>>,
    ) -> Stream<(K, V), Tick<L>, Bounded, Order> {
        let f = comb.splice_fn2_borrow_mut_ctx(&self.location).into();

        Stream::new(
            self.location,
            HydroNode::ReduceKeyed {
                f,
                input: Box::new(self.ir_node.into_inner()),
            },
        )
    }
}

impl<'a, T, L: Location<'a> + NoTick, B, Order> Stream<T, Timestamped<L>, B, Order> {
    /// Given a tick, returns a stream corresponding to a batch of elements for that tick.
    /// These batches are guaranteed to be contiguous across ticks and preserve the order
    /// of the input.
    ///
    /// # Safety
    /// The batch boundaries are non-deterministic and may change across executions.
    pub unsafe fn tick_batch(self) -> Stream<T, Tick<L>, Bounded, Order> {
        Stream::new(
            self.location.tick,
            HydroNode::Unpersist(Box::new(self.ir_node.into_inner())),
        )
    }

    pub fn drop_timestamp(self) -> Stream<T, L, B, Order> {
        Stream::new(self.location.tick.l, self.ir_node.into_inner())
    }

    pub fn timestamp_source(&self) -> Tick<L> {
        self.location.tick.clone()
    }
}

impl<'a, T, L: Location<'a> + NoTick + NoTimestamp, B, Order> Stream<T, L, B, Order> {
    pub fn timestamped(self, tick: &Tick<L>) -> Stream<T, Timestamped<L>, B, Order> {
        Stream::new(
            Timestamped { tick: tick.clone() },
            self.ir_node.into_inner(),
        )
    }

    /// Given a time interval, returns a stream corresponding to samples taken from the
    /// stream roughly at that interval. The output will have elements in the same order
    /// as the input, but with arbitrary elements skipped between samples. There is also
    /// no guarantee on the exact timing of the samples.
    ///
    /// # Safety
    /// The output stream is non-deterministic in which elements are sampled, since this
    /// is controlled by a clock.
    pub unsafe fn sample_every(
        self,
        interval: impl QuotedWithContext<'a, std::time::Duration, L> + Copy + 'a,
    ) -> Stream<T, L, Unbounded, Order> {
        let samples = unsafe {
            // SAFETY: source of intentional non-determinism
            self.location.source_interval(interval)
        };

        let tick = self.location.tick();
        unsafe {
            // SAFETY: source of intentional non-determinism
            self.timestamped(&tick)
                .tick_batch()
                .continue_if(samples.timestamped(&tick).tick_batch().first())
                .all_ticks()
                .drop_timestamp()
        }
    }

    /// Given a timeout duration, returns an [`Optional`]  which will have a value if the
    /// stream has not emitted a value since that duration.
    ///
    /// # Safety
    /// Timeout relies on non-deterministic sampling of the stream, so depending on when
    /// samples take place, timeouts may be non-deterministically generated or missed,
    /// and the notification of the timeout may be delayed as well. There is also no
    /// guarantee on how long the [`Optional`] will have a value after the timeout is
    /// detected based on when the next sample is taken.
    pub unsafe fn timeout(
        self,
        duration: impl QuotedWithContext<'a, std::time::Duration, Tick<L>> + Copy + 'a,
    ) -> Optional<(), L, Unbounded>
    where
        Order: MinOrder<NoOrder, Min = NoOrder>,
    {
        let tick = self.location.tick();

        let latest_received = self.fold_commutative(
            q!(|| None),
            q!(|latest, _| {
                // Note: May want to check received ballot against our own?
                *latest = Some(Instant::now());
            }),
        );

        unsafe {
            // SAFETY: Non-deterministic delay in detecting a timeout is expected.
            latest_received.timestamped(&tick).latest_tick()
        }
        .filter_map(q!(move |latest_received| {
            if let Some(latest_received) = latest_received {
                if Instant::now().duration_since(latest_received) > duration {
                    Some(())
                } else {
                    None
                }
            } else {
                Some(())
            }
        }))
        .latest()
        .drop_timestamp()
    }
}

impl<'a, T, L: Location<'a> + NoTick, B, Order> Stream<T, L, B, Order> {
    pub fn for_each<F: Fn(T) + 'a>(self, f: impl IntoQuotedMut<'a, F, L>) {
        let f = f.splice_fn1_ctx(&self.location).into();
        self.location
            .flow_state()
            .borrow_mut()
            .leaves
            .as_mut()
            .expect(FLOW_USED_MESSAGE)
            .push(HydroLeaf::ForEach {
                input: Box::new(HydroNode::Unpersist(Box::new(self.ir_node.into_inner()))),
                f,
            });
    }

    pub fn dest_sink<S: Unpin + futures::Sink<T> + 'a>(
        self,
        sink: impl QuotedWithContext<'a, S, L>,
    ) {
        self.location
            .flow_state()
            .borrow_mut()
            .leaves
            .as_mut()
            .expect(FLOW_USED_MESSAGE)
            .push(HydroLeaf::DestSink {
                sink: sink.splice_typed_ctx(&self.location).into(),
                input: Box::new(self.ir_node.into_inner()),
            });
    }
}

impl<'a, T, L: Location<'a>, Order> Stream<T, Tick<L>, Bounded, Order> {
    pub fn all_ticks(self) -> Stream<T, Timestamped<L>, Unbounded, Order> {
        Stream::new(
            Timestamped {
                tick: self.location.clone(),
            },
            HydroNode::Persist(Box::new(self.ir_node.into_inner())),
        )
    }

    pub fn persist(self) -> Stream<T, Tick<L>, Bounded, Order>
    where
        T: Clone,
    {
        Stream::new(
            self.location,
            HydroNode::Persist(Box::new(self.ir_node.into_inner())),
        )
    }

    pub fn defer_tick(self) -> Stream<T, Tick<L>, Bounded, Order> {
        Stream::new(
            self.location,
            HydroNode::DeferTick(Box::new(self.ir_node.into_inner())),
        )
    }

    pub fn delta(self) -> Stream<T, Tick<L>, Bounded, Order> {
        Stream::new(
            self.location,
            HydroNode::Delta(Box::new(self.ir_node.into_inner())),
        )
    }
}

fn serialize_bincode<T: Serialize>(is_demux: bool) -> syn::Expr {
    let root = get_this_crate();

    let t_type: syn::Type = stageleft::quote_type::<T>();

    if is_demux {
        parse_quote! {
            |(id, data): (#root::ClusterId<_>, #t_type)| {
                (id.raw_id, #root::runtime_support::bincode::serialize::<#t_type>(&data).unwrap().into())
            }
        }
    } else {
        parse_quote! {
            |data| {
                #root::runtime_support::bincode::serialize::<#t_type>(&data).unwrap().into()
            }
        }
    }
}

pub(super) fn deserialize_bincode<T: DeserializeOwned>(tagged: Option<syn::Type>) -> syn::Expr {
    let root = get_this_crate();

    let t_type: syn::Type = stageleft::quote_type::<T>();

    if let Some(c_type) = tagged {
        parse_quote! {
            |res| {
                let (id, b) = res.unwrap();
                (#root::ClusterId::<#c_type>::from_raw(id), #root::runtime_support::bincode::deserialize::<#t_type>(&b).unwrap())
            }
        }
    } else {
        parse_quote! {
            |res| {
                #root::runtime_support::bincode::deserialize::<#t_type>(&res.unwrap()).unwrap()
            }
        }
    }
}

impl<'a, T, C1, B, Order> Stream<T, Cluster<'a, C1>, B, Order> {
    pub fn decouple_cluster<C2: 'a, Tag>(
        self,
        other: &Cluster<'a, C2>,
    ) -> Stream<T, Cluster<'a, C2>, Unbounded, Order>
    where
        Cluster<'a, C1>: Location<'a, Root = Cluster<'a, C1>>,
        Cluster<'a, C1>:
            CanSend<'a, Cluster<'a, C2>, In<T> = (ClusterId<C2>, T), Out<T> = (Tag, T)>,
        T: Clone + Serialize + DeserializeOwned,
        Order:
            MinOrder<<Cluster<'a, C1> as CanSend<'a, Cluster<'a, C2>>>::OutStrongestOrder<Order>>,
    {
        let sent = self
            .map(q!(move |b| (
                ClusterId::from_raw(CLUSTER_SELF_ID.raw_id),
                b.clone()
            )))
            .send_bincode_interleaved(other);

        unsafe {
            // SAFETY: this is safe because we are mapping clusters 1:1
            sent.assume_ordering()
        }
    }
}

impl<'a, T, L: Location<'a> + NoTick, B, Order> Stream<T, L, B, Order> {
    pub fn decouple_process<P2>(
        self,
        other: &Process<'a, P2>,
    ) -> Stream<T, Process<'a, P2>, Unbounded, Order>
    where
        L::Root: CanSend<'a, Process<'a, P2>, In<T> = T, Out<T> = T>,
        T: Clone + Serialize + DeserializeOwned,
        Order: MinOrder<
            <L::Root as CanSend<'a, Process<'a, P2>>>::OutStrongestOrder<Order>,
            Min = Order,
        >,
    {
        self.send_bincode::<Process<'a, P2>, T>(other)
    }

    pub fn send_bincode<L2: Location<'a>, CoreType>(
        self,
        other: &L2,
    ) -> Stream<<L::Root as CanSend<'a, L2>>::Out<CoreType>, L2, Unbounded, Order::Min>
    where
        L::Root: CanSend<'a, L2, In<CoreType> = T>,
        CoreType: Serialize + DeserializeOwned,
        Order: MinOrder<<L::Root as CanSend<'a, L2>>::OutStrongestOrder<Order>>,
    {
        let serialize_pipeline = Some(serialize_bincode::<CoreType>(L::Root::is_demux()));

        let deserialize_pipeline = Some(deserialize_bincode::<CoreType>(L::Root::tagged_type()));

        Stream::new(
            other.clone(),
            HydroNode::Network {
                from_location: self.location.root().id(),
                from_key: None,
                to_location: other.id(),
                to_key: None,
                serialize_fn: serialize_pipeline.map(|e| e.into()),
                instantiate_fn: DebugInstantiate::Building(),
                deserialize_fn: deserialize_pipeline.map(|e| e.into()),
                input: Box::new(self.ir_node.into_inner()),
            },
        )
    }

    pub fn send_bincode_external<L2: 'a, CoreType>(
        self,
        other: &ExternalProcess<L2>,
    ) -> ExternalBincodeStream<L::Out<CoreType>>
    where
        L: CanSend<'a, ExternalProcess<'a, L2>, In<CoreType> = T, Out<CoreType> = CoreType>,
        CoreType: Serialize + DeserializeOwned,
        // for now, we restirct Out<CoreType> to be CoreType, which means no tagged cluster -> external
    {
        let serialize_pipeline = Some(serialize_bincode::<CoreType>(L::is_demux()));

        let mut flow_state_borrow = self.location.flow_state().borrow_mut();

        let external_key = flow_state_borrow.next_external_out;
        flow_state_borrow.next_external_out += 1;

        let leaves = flow_state_borrow.leaves.as_mut().expect("Attempted to add a leaf to a flow that has already been finalized. No leaves can be added after the flow has been compiled()");

        let dummy_f: syn::Expr = syn::parse_quote!(());

        leaves.push(HydroLeaf::ForEach {
            f: dummy_f.into(),
            input: Box::new(HydroNode::Network {
                from_location: self.location.root().id(),
                from_key: None,
                to_location: other.id(),
                to_key: Some(external_key),
                serialize_fn: serialize_pipeline.map(|e| e.into()),
                instantiate_fn: DebugInstantiate::Building(),
                deserialize_fn: None,
                input: Box::new(self.ir_node.into_inner()),
            }),
        });

        ExternalBincodeStream {
            process_id: other.id,
            port_id: external_key,
            _phantom: PhantomData,
        }
    }

    pub fn send_bytes<L2: Location<'a>>(
        self,
        other: &L2,
    ) -> Stream<<L::Root as CanSend<'a, L2>>::Out<Bytes>, L2, Unbounded, Order::Min>
    where
        L::Root: CanSend<'a, L2, In<Bytes> = T>,
        Order: MinOrder<<L::Root as CanSend<'a, L2>>::OutStrongestOrder<Order>>,
    {
        let root = get_this_crate();
        Stream::new(
            other.clone(),
            HydroNode::Network {
                from_location: self.location.root().id(),
                from_key: None,
                to_location: other.id(),
                to_key: None,
                serialize_fn: None,
                instantiate_fn: DebugInstantiate::Building(),
                deserialize_fn: if let Some(c_type) = L::Root::tagged_type() {
                    let expr: syn::Expr = parse_quote!(|(id, b)| (#root::ClusterId<#c_type>::from_raw(id), b.unwrap().freeze()));
                    Some(expr.into())
                } else {
                    let expr: syn::Expr = parse_quote!(|b| b.unwrap().freeze());
                    Some(expr.into())
                },
                input: Box::new(self.ir_node.into_inner()),
            },
        )
    }

    pub fn send_bytes_external<L2: 'a>(self, other: &ExternalProcess<L2>) -> ExternalBytesPort
    where
        L::Root: CanSend<'a, ExternalProcess<'a, L2>, In<Bytes> = T, Out<Bytes> = Bytes>,
    {
        let mut flow_state_borrow = self.location.flow_state().borrow_mut();
        let external_key = flow_state_borrow.next_external_out;
        flow_state_borrow.next_external_out += 1;

        let leaves = flow_state_borrow.leaves.as_mut().expect("Attempted to add a leaf to a flow that has already been finalized. No leaves can be added after the flow has been compiled()");

        let dummy_f: syn::Expr = syn::parse_quote!(());

        leaves.push(HydroLeaf::ForEach {
            f: dummy_f.into(),
            input: Box::new(HydroNode::Network {
                from_location: self.location.root().id(),
                from_key: None,
                to_location: other.id(),
                to_key: Some(external_key),
                serialize_fn: None,
                instantiate_fn: DebugInstantiate::Building(),
                deserialize_fn: None,
                input: Box::new(self.ir_node.into_inner()),
            }),
        });

        ExternalBytesPort {
            process_id: other.id,
            port_id: external_key,
        }
    }

    pub fn send_bincode_interleaved<L2: Location<'a>, Tag, CoreType>(
        self,
        other: &L2,
    ) -> Stream<CoreType, L2, Unbounded, Order::Min>
    where
        L::Root: CanSend<'a, L2, In<CoreType> = T, Out<CoreType> = (Tag, CoreType)>,
        CoreType: Serialize + DeserializeOwned,
        Order: MinOrder<<L::Root as CanSend<'a, L2>>::OutStrongestOrder<Order>>,
    {
        self.send_bincode::<L2, CoreType>(other).map(q!(|(_, b)| b))
    }

    pub fn send_bytes_interleaved<L2: Location<'a>, Tag>(
        self,
        other: &L2,
    ) -> Stream<Bytes, L2, Unbounded, Order::Min>
    where
        L::Root: CanSend<'a, L2, In<Bytes> = T, Out<Bytes> = (Tag, Bytes)>,
        Order: MinOrder<<L::Root as CanSend<'a, L2>>::OutStrongestOrder<Order>>,
    {
        self.send_bytes::<L2>(other).map(q!(|(_, b)| b))
    }

    #[expect(clippy::type_complexity, reason = "ordering semantics for broadcast")]
    pub fn broadcast_bincode<C2: 'a>(
        self,
        other: &Cluster<'a, C2>,
    ) -> Stream<
        <L::Root as CanSend<'a, Cluster<'a, C2>>>::Out<T>,
        Cluster<'a, C2>,
        Unbounded,
        Order::Min,
    >
    where
        L::Root: CanSend<'a, Cluster<'a, C2>, In<T> = (ClusterId<C2>, T)>,
        T: Clone + Serialize + DeserializeOwned,
        Order: MinOrder<<L::Root as CanSend<'a, Cluster<'a, C2>>>::OutStrongestOrder<Order>>,
    {
        let ids = other.members();

        self.flat_map_ordered(q!(|b| ids.iter().map(move |id| (
            ::std::clone::Clone::clone(id),
            ::std::clone::Clone::clone(&b)
        ))))
        .send_bincode(other)
    }

    pub fn broadcast_bincode_interleaved<C2: 'a, Tag>(
        self,
        other: &Cluster<'a, C2>,
    ) -> Stream<T, Cluster<'a, C2>, Unbounded, Order::Min>
    where
        L::Root: CanSend<'a, Cluster<'a, C2>, In<T> = (ClusterId<C2>, T), Out<T> = (Tag, T)> + 'a,
        T: Clone + Serialize + DeserializeOwned,
        Order: MinOrder<<L::Root as CanSend<'a, Cluster<'a, C2>>>::OutStrongestOrder<Order>>,
    {
        self.broadcast_bincode(other).map(q!(|(_, b)| b))
    }

    #[expect(clippy::type_complexity, reason = "ordering semantics for broadcast")]
    pub fn broadcast_bytes<C2: 'a>(
        self,
        other: &Cluster<'a, C2>,
    ) -> Stream<
        <L::Root as CanSend<'a, Cluster<'a, C2>>>::Out<Bytes>,
        Cluster<'a, C2>,
        Unbounded,
        Order::Min,
    >
    where
        L::Root: CanSend<'a, Cluster<'a, C2>, In<Bytes> = (ClusterId<C2>, T)> + 'a,
        T: Clone,
        Order: MinOrder<<L::Root as CanSend<'a, Cluster<'a, C2>>>::OutStrongestOrder<Order>>,
    {
        let ids = other.members();

        self.flat_map_ordered(q!(|b| ids.iter().map(move |id| (
            ::std::clone::Clone::clone(id),
            ::std::clone::Clone::clone(&b)
        ))))
        .send_bytes(other)
    }

    pub fn broadcast_bytes_interleaved<C2: 'a, Tag>(
        self,
        other: &Cluster<'a, C2>,
    ) -> Stream<Bytes, Cluster<'a, C2>, Unbounded, Order::Min>
    where
        L::Root: CanSend<'a, Cluster<'a, C2>, In<Bytes> = (ClusterId<C2>, T), Out<Bytes> = (Tag, Bytes)>
            + 'a,
        T: Clone,
        Order: MinOrder<<L::Root as CanSend<'a, Cluster<'a, C2>>>::OutStrongestOrder<Order>>,
    {
        self.broadcast_bytes(other).map(q!(|(_, b)| b))
    }
}

#[expect(clippy::type_complexity, reason = "ordering semantics for round-robin")]
impl<'a, T, L: Location<'a> + NoTick, B> Stream<T, L, B, TotalOrder> {
    pub fn round_robin_bincode<C2: 'a>(
        self,
        other: &Cluster<'a, C2>,
    ) -> Stream<
        <L::Root as CanSend<'a, Cluster<'a, C2>>>::Out<T>,
        Cluster<'a, C2>,
        Unbounded,
        <TotalOrder as MinOrder<
            <L::Root as CanSend<'a, Cluster<'a, C2>>>::OutStrongestOrder<TotalOrder>,
        >>::Min,
    >
    where
        L::Root: CanSend<'a, Cluster<'a, C2>, In<T> = (ClusterId<C2>, T)>,
        T: Clone + Serialize + DeserializeOwned,
        TotalOrder:
            MinOrder<<L::Root as CanSend<'a, Cluster<'a, C2>>>::OutStrongestOrder<TotalOrder>>,
    {
        let ids = other.members();

        self.enumerate()
            .map(q!(|(i, w)| (ids[i % ids.len()], w)))
            .send_bincode(other)
    }

    pub fn round_robin_bincode_interleaved<C2: 'a, Tag>(
        self,
        other: &Cluster<'a, C2>,
    ) -> Stream<
        T,
        Cluster<'a, C2>,
        Unbounded,
        <TotalOrder as MinOrder<
            <L::Root as CanSend<'a, Cluster<'a, C2>>>::OutStrongestOrder<TotalOrder>,
        >>::Min,
    >
    where
        L::Root: CanSend<'a, Cluster<'a, C2>, In<T> = (ClusterId<C2>, T), Out<T> = (Tag, T)> + 'a,
        T: Clone + Serialize + DeserializeOwned,
        TotalOrder:
            MinOrder<<L::Root as CanSend<'a, Cluster<'a, C2>>>::OutStrongestOrder<TotalOrder>>,
    {
        self.round_robin_bincode(other).map(q!(|(_, b)| b))
    }

    pub fn round_robin_bytes<C2: 'a>(
        self,
        other: &Cluster<'a, C2>,
    ) -> Stream<
        <L::Root as CanSend<'a, Cluster<'a, C2>>>::Out<Bytes>,
        Cluster<'a, C2>,
        Unbounded,
        <TotalOrder as MinOrder<
            <L::Root as CanSend<'a, Cluster<'a, C2>>>::OutStrongestOrder<TotalOrder>,
        >>::Min,
    >
    where
        L::Root: CanSend<'a, Cluster<'a, C2>, In<Bytes> = (ClusterId<C2>, T)> + 'a,
        T: Clone,
        TotalOrder:
            MinOrder<<L::Root as CanSend<'a, Cluster<'a, C2>>>::OutStrongestOrder<TotalOrder>>,
    {
        let ids = other.members();

        self.enumerate()
            .map(q!(|(i, w)| (ids[i % ids.len()], w)))
            .send_bytes(other)
    }

    pub fn round_robin_bytes_interleaved<C2: 'a, Tag>(
        self,
        other: &Cluster<'a, C2>,
    ) -> Stream<
        Bytes,
        Cluster<'a, C2>,
        Unbounded,
        <TotalOrder as MinOrder<
            <L::Root as CanSend<'a, Cluster<'a, C2>>>::OutStrongestOrder<TotalOrder>,
        >>::Min,
    >
    where
        L::Root: CanSend<'a, Cluster<'a, C2>, In<Bytes> = (ClusterId<C2>, T), Out<Bytes> = (Tag, Bytes)>
            + 'a,
        T: Clone,
        TotalOrder:
            MinOrder<<L::Root as CanSend<'a, Cluster<'a, C2>>>::OutStrongestOrder<TotalOrder>>,
    {
        self.round_robin_bytes(other).map(q!(|(_, b)| b))
    }
}

#[cfg(test)]
mod tests {
    use dfir_rs::futures::StreamExt;
    use hydro_deploy::Deployment;
    use serde::{Deserialize, Serialize};
    use stageleft::q;

    use crate::location::Location;
    use crate::FlowBuilder;

    struct P1 {}
    struct P2 {}

    #[derive(Serialize, Deserialize, Debug)]
    struct SendOverNetwork {
        n: u32,
    }

    #[tokio::test]
    async fn first_ten_distributed() {
        let mut deployment = Deployment::new();

        let flow = FlowBuilder::new();
        let first_node = flow.process::<P1>();
        let second_node = flow.process::<P2>();
        let external = flow.external_process::<P2>();

        let numbers = first_node.source_iter(q!(0..10));
        let out_port = numbers
            .map(q!(|n| SendOverNetwork { n }))
            .send_bincode(&second_node)
            .send_bincode_external(&external);

        let nodes = flow
            .with_process(&first_node, deployment.Localhost())
            .with_process(&second_node, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let mut external_out = nodes.connect_source_bincode(out_port).await;

        deployment.start().await.unwrap();

        for i in 0..10 {
            assert_eq!(external_out.next().await.unwrap().n, i);
        }
    }
}
