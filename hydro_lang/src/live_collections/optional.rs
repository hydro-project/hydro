//! Definitions for the [`Optional`] live collection.

use std::cell::RefCell;
use std::marker::PhantomData;
use std::ops::Deref;
use std::rc::Rc;

use stageleft::{IntoQuotedMut, QuotedWithContext, q};
use syn::parse_quote;

use super::boundedness::{Bounded, Boundedness, Unbounded};
use super::singleton::Singleton;
use super::stream::{AtLeastOnce, ExactlyOnce, NoOrder, Stream, TotalOrder};
use crate::compile::ir::{HydroIrOpMetadata, HydroNode, HydroRoot, HydroSource, TeeNode};
#[cfg(stageleft_runtime)]
use crate::forward_handle::{CycleCollection, ReceiverComplete};
use crate::forward_handle::{ForwardRef, TickCycle};
#[cfg(stageleft_runtime)]
use crate::location::dynamic::{DynLocation, LocationId};
use crate::location::tick::{Atomic, DeferTick, NoAtomic};
use crate::location::{Location, NoTick, Tick, check_matching_location};
use crate::nondet::NonDet;

/// A *nullable* Rust value that can asynchronously change over time.
///
/// Optionals are the live collection equivalent of [`Option`]. If the optional is [`Bounded`],
/// the value is frozen and will not change. But if it is [`Unbounded`], the value will
/// asynchronously change over time, including becoming present of uninhabited.
///
/// Optionals are used in many of the same places as [`Singleton`], but when the value may be
/// nullable. For example, the first element of a [`Stream`] is exposed as an [`Optional`].
///
/// Type Parameters:
/// - `Type`: the type of the value in this optional (when it is not null)
/// - `Loc`: the [`Location`] where the optional is materialized
/// - `Bound`: tracks whether the value is [`Bounded`] (fixed) or [`Unbounded`] (changing asynchronously)
pub struct Optional<Type, Loc, Bound: Boundedness> {
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

impl<'a, T, L> CycleCollection<'a, TickCycle> for Optional<T, Tick<L>, Bounded>
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

impl<'a, T, L> ReceiverComplete<'a, TickCycle> for Optional<T, Tick<L>, Bounded>
where
    L: Location<'a>,
{
    fn complete(self, ident: syn::Ident, expected_location: LocationId) {
        assert_eq!(
            Location::id(&self.location),
            expected_location,
            "locations do not match"
        );
        self.location
            .flow_state()
            .borrow_mut()
            .push_root(HydroRoot::CycleSink {
                ident,
                input: Box::new(self.ir_node.into_inner()),
                out_location: Location::id(&self.location),
                op_metadata: HydroIrOpMetadata::new(),
            });
    }
}

impl<'a, T, L> CycleCollection<'a, ForwardRef> for Optional<T, Tick<L>, Bounded>
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

impl<'a, T, L> ReceiverComplete<'a, ForwardRef> for Optional<T, Tick<L>, Bounded>
where
    L: Location<'a>,
{
    fn complete(self, ident: syn::Ident, expected_location: LocationId) {
        assert_eq!(
            Location::id(&self.location),
            expected_location,
            "locations do not match"
        );
        self.location
            .flow_state()
            .borrow_mut()
            .push_root(HydroRoot::CycleSink {
                ident,
                input: Box::new(self.ir_node.into_inner()),
                out_location: Location::id(&self.location),
                op_metadata: HydroIrOpMetadata::new(),
            });
    }
}

impl<'a, T, L, B: Boundedness> CycleCollection<'a, ForwardRef> for Optional<T, L, B>
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

impl<'a, T, L, B: Boundedness> ReceiverComplete<'a, ForwardRef> for Optional<T, L, B>
where
    L: Location<'a> + NoTick,
{
    fn complete(self, ident: syn::Ident, expected_location: LocationId) {
        assert_eq!(
            Location::id(&self.location),
            expected_location,
            "locations do not match"
        );
        let metadata = self.location.new_node_metadata::<T>();
        self.location
            .flow_state()
            .borrow_mut()
            .push_root(HydroRoot::CycleSink {
                ident,
                input: Box::new(HydroNode::Unpersist {
                    inner: Box::new(self.ir_node.into_inner()),
                    metadata: metadata.clone(),
                }),
                out_location: Location::id(&self.location),
                op_metadata: HydroIrOpMetadata::new(),
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

impl<'a, T, L, B: Boundedness> From<Singleton<T, L, B>> for Optional<T, L, B>
where
    L: Location<'a>,
{
    fn from(singleton: Singleton<T, L, B>) -> Self {
        Optional::new(singleton.location, singleton.ir_node.into_inner())
    }
}

impl<'a, T, L, B: Boundedness> Clone for Optional<T, L, B>
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

impl<'a, T, L, B: Boundedness> Optional<T, L, B>
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

    /// Transforms the optional value by applying a function `f` to it,
    /// continuously as the input is updated.
    ///
    /// Whenever the optional is empty, the output optional is also empty.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
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

    /// Transforms the optional value by applying a function `f` to it and then flattening
    /// the result into a stream, preserving the order of elements.
    ///
    /// If the optional is empty, the output stream is also empty. If the optional contains
    /// a value, `f` is applied to produce an iterator, and all items from that iterator
    /// are emitted in the output stream in deterministic order.
    ///
    /// The implementation of [`Iterator`] for the output type `I` must produce items in a
    /// **deterministic** order. For example, `I` could be a `Vec`, but not a `HashSet`.
    /// If the order is not deterministic, use [`Optional::flat_map_unordered`] instead.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let optional = tick.optional_first_tick(q!(vec![1, 2, 3]));
    /// optional.flat_map_ordered(q!(|v| v)).all_ticks()
    /// # }, |mut stream| async move {
    /// // 1, 2, 3
    /// # for w in vec![1, 2, 3] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// ```
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

    /// Like [`Optional::flat_map_ordered`], but allows the implementation of [`Iterator`]
    /// for the output type `I` to produce items in any order.
    ///
    /// If the optional is empty, the output stream is also empty. If the optional contains
    /// a value, `f` is applied to produce an iterator, and all items from that iterator
    /// are emitted in the output stream in non-deterministic order.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::{prelude::*, live_collections::stream::{NoOrder, ExactlyOnce}};
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test::<_, _, NoOrder, ExactlyOnce>(|process| {
    /// let tick = process.tick();
    /// let optional = tick.optional_first_tick(q!(
    ///     std::collections::HashSet::<i32>::from_iter(vec![1, 2, 3])
    /// ));
    /// optional.flat_map_unordered(q!(|v| v)).all_ticks()
    /// # }, |mut stream| async move {
    /// // 1, 2, 3, but in no particular order
    /// # let mut results = Vec::new();
    /// # for _ in 0..3 {
    /// #     results.push(stream.next().await.unwrap());
    /// # }
    /// # results.sort();
    /// # assert_eq!(results, vec![1, 2, 3]);
    /// # }));
    /// ```
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

    /// Flattens the optional value into a stream, preserving the order of elements.
    ///
    /// If the optional is empty, the output stream is also empty. If the optional contains
    /// a value that implements [`IntoIterator`], all items from that iterator are emitted
    /// in the output stream in deterministic order.
    ///
    /// The implementation of [`Iterator`] for the element type `T` must produce items in a
    /// **deterministic** order. For example, `T` could be a `Vec`, but not a `HashSet`.
    /// If the order is not deterministic, use [`Optional::flatten_unordered`] instead.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let optional = tick.optional_first_tick(q!(vec![1, 2, 3]));
    /// optional.flatten_ordered().all_ticks()
    /// # }, |mut stream| async move {
    /// // 1, 2, 3
    /// # for w in vec![1, 2, 3] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// ```
    pub fn flatten_ordered<U>(self) -> Stream<U, L, B, TotalOrder, ExactlyOnce>
    where
        T: IntoIterator<Item = U>,
    {
        self.flat_map_ordered(q!(|v| v))
    }

    /// Like [`Optional::flatten_ordered`], but allows the implementation of [`Iterator`]
    /// for the element type `T` to produce items in any order.
    ///
    /// If the optional is empty, the output stream is also empty. If the optional contains
    /// a value that implements [`IntoIterator`], all items from that iterator are emitted
    /// in the output stream in non-deterministic order.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::{prelude::*, live_collections::stream::{NoOrder, ExactlyOnce}};
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test::<_, _, NoOrder, ExactlyOnce>(|process| {
    /// let tick = process.tick();
    /// let optional = tick.optional_first_tick(q!(
    ///     std::collections::HashSet::<i32>::from_iter(vec![1, 2, 3])
    /// ));
    /// optional.flatten_unordered().all_ticks()
    /// # }, |mut stream| async move {
    /// // 1, 2, 3, but in no particular order
    /// # let mut results = Vec::new();
    /// # for _ in 0..3 {
    /// #     results.push(stream.next().await.unwrap());
    /// # }
    /// # results.sort();
    /// # assert_eq!(results, vec![1, 2, 3]);
    /// # }));
    /// ```
    pub fn flatten_unordered<U>(self) -> Stream<U, L, B, NoOrder, ExactlyOnce>
    where
        T: IntoIterator<Item = U>,
    {
        self.flat_map_unordered(q!(|v| v))
    }

    /// Creates an optional containing only the value if it satisfies a predicate `f`.
    ///
    /// If the optional is empty, the output optional is also empty. If the optional contains
    /// a value and the predicate returns `true`, the output optional contains the same value.
    /// If the predicate returns `false`, the output optional is empty.
    ///
    /// The closure `f` receives a reference `&T` rather than an owned value `T` because filtering does
    /// not modify or take ownership of the value. If you need to modify the value while filtering
    /// use [`Optional::filter_map`] instead.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let optional = tick.optional_first_tick(q!(5));
    /// optional.filter(q!(|&x| x > 3)).all_ticks()
    /// # }, |mut stream| async move {
    /// // 5
    /// # assert_eq!(stream.next().await.unwrap(), 5);
    /// # }));
    /// ```
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

    /// An operator that both filters and maps. It yields only the value if the supplied
    /// closure `f` returns `Some(value)`.
    ///
    /// If the optional is empty, the output optional is also empty. If the optional contains
    /// a value and the closure returns `Some(new_value)`, the output optional contains `new_value`.
    /// If the closure returns `None`, the output optional is empty.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let optional = tick.optional_first_tick(q!("42"));
    /// optional
    ///     .filter_map(q!(|s| s.parse::<i32>().ok()))
    ///     .all_ticks()
    /// # }, |mut stream| async move {
    /// // 42
    /// # assert_eq!(stream.next().await.unwrap(), 42);
    /// # }));
    /// ```
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

    /// Combines this singleton with another [`Singleton`] or [`Optional`] by tupling their values.
    ///
    /// If the other value is a [`Optional`], the output will be non-null only if the argument is
    /// non-null. This is useful for combining several pieces of state together.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let numbers = process
    ///   .source_iter(q!(vec![123, 456, 789]))
    ///   .batch(&tick, nondet!(/** test */));
    /// let min = numbers.clone().min(); // Optional
    /// let max = numbers.max(); // Optional
    /// min.zip(max).all_ticks()
    /// # }, |mut stream| async move {
    /// // [(123, 789)]
    /// # for w in vec![(123, 789)] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// ```
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

    #[expect(missing_docs, reason = "TODO")]
    pub fn or(self, other: Optional<T, L, B>) -> Optional<T, L, B> {
        check_matching_location(&self.location, &other.location);

        if L::is_top_level() {
            Optional::new(
                self.location.clone(),
                HydroNode::Persist {
                    inner: Box::new(HydroNode::ChainFirst {
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
                HydroNode::ChainFirst {
                    first: Box::new(self.ir_node.into_inner()),
                    second: Box::new(other.ir_node.into_inner()),
                    metadata: self.location.new_node_metadata::<T>(),
                },
            )
        }
    }

    #[expect(missing_docs, reason = "TODO")]
    pub fn unwrap_or(self, other: Singleton<T, L, B>) -> Singleton<T, L, B> {
        let res_option = self.or(other.into());
        Singleton::new(res_option.location, res_option.ir_node.into_inner())
    }

    #[expect(missing_docs, reason = "TODO")]
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
    /// Filters this optional, passing through the optional value if it is non-null **and** the
    /// argument (a [`Bounded`] [`Optional`]`) is non-null, otherwise the output is null.
    ///
    /// Useful for conditionally processing, such as only emitting an optional's value outside
    /// a tick if some other condition is satisfied.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// // ticks are lazy by default, forces the second tick to run
    /// tick.spin_batch(q!(1)).all_ticks().for_each(q!(|_| {}));
    ///
    /// let batch_first_tick = process
    ///   .source_iter(q!(vec![]))
    ///   .batch(&tick, nondet!(/** test */));
    /// let batch_second_tick = process
    ///   .source_iter(q!(vec![456]))
    ///   .batch(&tick, nondet!(/** test */))
    ///   .defer_tick(); // appears on the second tick
    /// let some_on_first_tick = tick.optional_first_tick(q!(()));
    /// batch_first_tick.chain(batch_second_tick).first()
    ///   .filter_if_some(some_on_first_tick)
    ///   .unwrap_or(tick.singleton(q!(789)))
    ///   .all_ticks()
    /// # }, |mut stream| async move {
    /// // [789, 789]
    /// # for w in vec![789, 789] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// ```
    pub fn filter_if_some<U>(self, signal: Optional<U, L, Bounded>) -> Optional<T, L, Bounded> {
        self.zip(signal.map(q!(|_u| ()))).map(q!(|(d, _signal)| d))
    }

    /// Filters this optional, passing through the optional value if it is non-null **and** the
    /// argument (a [`Bounded`] [`Optional`]`) is _null_, otherwise the output is null.
    ///
    /// Useful for conditionally processing, such as only emitting an optional's value outside
    /// a tick if some other condition is satisfied.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// // ticks are lazy by default, forces the second tick to run
    /// tick.spin_batch(q!(1)).all_ticks().for_each(q!(|_| {}));
    ///
    /// let batch_first_tick = process
    ///   .source_iter(q!(vec![]))
    ///   .batch(&tick, nondet!(/** test */));
    /// let batch_second_tick = process
    ///   .source_iter(q!(vec![456]))
    ///   .batch(&tick, nondet!(/** test */))
    ///   .defer_tick(); // appears on the second tick
    /// let some_on_first_tick = tick.optional_first_tick(q!(()));
    /// batch_first_tick.chain(batch_second_tick).first()
    ///   .filter_if_none(some_on_first_tick)
    ///   .unwrap_or(tick.singleton(q!(789)))
    ///   .all_ticks()
    /// # }, |mut stream| async move {
    /// // [789, 789]
    /// # for w in vec![789, 456] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// ```
    pub fn filter_if_none<U>(self, other: Optional<U, L, Bounded>) -> Optional<T, L, Bounded> {
        self.filter_if_some(
            other
                .map(q!(|_| ()))
                .into_singleton()
                .filter(q!(|o| o.is_none())),
        )
    }

    #[expect(missing_docs, reason = "TODO")]
    pub fn then<U>(self, value: Singleton<U, L, Bounded>) -> Optional<U, L, Bounded> {
        value.filter_if_some(self)
    }
}

impl<'a, T, L, B: Boundedness> Optional<T, Atomic<L>, B>
where
    L: Location<'a> + NoTick,
{
    /// Returns an optional value corresponding to the latest snapshot of the optional
    /// being atomically processed. The snapshot at tick `t + 1` is guaranteed to include
    /// at least all relevant data that contributed to the snapshot at tick `t`. Furthermore,
    /// all snapshots of this optional into the atomic-associated tick will observe the
    /// same value each tick.
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

    /// Returns this optional back into a top-level, asynchronous execution context where updates
    /// to the value will be asynchronously propagated.
    pub fn end_atomic(self) -> Optional<T, L, B> {
        Optional::new(self.location.tick.l, self.ir_node.into_inner())
    }
}

impl<'a, T, L, B: Boundedness> Optional<T, L, B>
where
    L: Location<'a> + NoTick + NoAtomic,
{
    /// Shifts this optional into an atomic context, which guarantees that any downstream logic
    /// will observe the same version of the value and will be executed synchronously before any
    /// outputs are yielded (in [`Optional::end_atomic`]).
    ///
    /// This is useful to enforce local consistency constraints, such as ensuring that several readers
    /// see a consistent version of local state (since otherwise each [`Optional::snapshot`] may pick
    /// a different version).
    ///
    /// Entering an atomic section requires a [`Tick`] argument that declares where the optional will
    /// be atomically processed. Snapshotting an optional into the _same_ [`Tick`] will preserve the
    /// synchronous execution, and all such snapshots in the same [`Tick`] will have the same value.
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
            .filter_if_some(samples.batch(&tick, nondet).first())
            .all_ticks()
            .weakest_retries()
    }
}

impl<'a, T, L> Optional<T, Tick<L>, Bounded>
where
    L: Location<'a>,
{
    /// Asynchronously yields the value of this singleton outside the tick as an unbounded stream,
    /// which will stream the value computed in _each_ tick as a separate stream element (skipping
    /// null values).
    ///
    /// Unlike [`Optional::latest`], the value computed in each tick is emitted separately,
    /// producing one element in the output for each (non-null) tick. This is useful for batched
    /// computations, where the results from each tick must be combined together.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// # let tick = process.tick();
    /// # // ticks are lazy by default, forces the second tick to run
    /// # tick.spin_batch(q!(1)).all_ticks().for_each(q!(|_| {}));
    /// # let batch_first_tick = process
    /// #   .source_iter(q!(vec![]))
    /// #   .batch(&tick, nondet!(/** test */));
    /// # let batch_second_tick = process
    /// #   .source_iter(q!(vec![1, 2, 3]))
    /// #   .batch(&tick, nondet!(/** test */))
    /// #   .defer_tick(); // appears on the second tick
    /// # let input_batch = batch_first_tick.chain(batch_second_tick);
    /// input_batch // first tick: [], second tick: [1, 2, 3]
    ///     .max()
    ///     .all_ticks()
    /// # }, |mut stream| async move {
    /// // [3]
    /// # for w in vec![3] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// ```
    pub fn all_ticks(self) -> Stream<T, L, Unbounded, TotalOrder, ExactlyOnce> {
        self.into_stream().all_ticks()
    }

    /// Synchronously yields the value of this optional outside the tick as an unbounded stream,
    /// which will stream the value computed in _each_ tick as a separate stream element.
    ///
    /// Unlike [`Optional::all_ticks`], this preserves synchronous execution, as the output stream
    /// is emitted in an [`Atomic`] context that will process elements synchronously with the input
    /// optional's [`Tick`] context.
    pub fn all_ticks_atomic(self) -> Stream<T, Atomic<L>, Unbounded, TotalOrder, ExactlyOnce> {
        self.into_stream().all_ticks_atomic()
    }

    /// Asynchronously yields this optional outside the tick as an unbounded optional, which will
    /// be asynchronously updated with the latest value of the optional inside the tick, including
    /// whether the optional is null or not.
    ///
    /// This converts a bounded value _inside_ a tick into an asynchronous value outside the
    /// tick that tracks the inner value. This is useful for getting the value as of the
    /// "most recent" tick, but note that updates are propagated asynchronously outside the tick.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// # let tick = process.tick();
    /// # // ticks are lazy by default, forces the second tick to run
    /// # tick.spin_batch(q!(1)).all_ticks().for_each(q!(|_| {}));
    /// # let batch_first_tick = process
    /// #   .source_iter(q!(vec![]))
    /// #   .batch(&tick, nondet!(/** test */));
    /// # let batch_second_tick = process
    /// #   .source_iter(q!(vec![1, 2, 3]))
    /// #   .batch(&tick, nondet!(/** test */))
    /// #   .defer_tick(); // appears on the second tick
    /// # let input_batch = batch_first_tick.chain(batch_second_tick);
    /// input_batch // first tick: [], second tick: [1, 2, 3]
    ///     .max()
    ///     .latest()
    /// # .into_singleton()
    /// # .sample_eager(nondet!(/** test */))
    /// # }, |mut stream| async move {
    /// // asynchronously changes from None ~> 3
    /// # for w in vec![None, Some(3)] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// ```
    pub fn latest(self) -> Optional<T, L, Unbounded> {
        Optional::new(
            self.location.outer().clone(),
            HydroNode::Persist {
                inner: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata::<T>(),
            },
        )
    }

    /// Synchronously yields this optional outside the tick as an unbounded optional, which will
    /// be updated with the latest value of the optional inside the tick.
    ///
    /// Unlike [`Optional::latest`], this preserves synchronous execution, as the output optional
    /// is emitted in an [`Atomic`] context that will process elements synchronously with the input
    /// optional's [`Tick`] context.
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

    #[expect(missing_docs, reason = "TODO")]
    pub fn defer_tick(self) -> Optional<T, Tick<L>, Bounded> {
        Optional::new(
            self.location.clone(),
            HydroNode::DeferTick {
                input: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata::<T>(),
            },
        )
    }

    #[deprecated(note = "use .into_stream().persist()")]
    #[expect(missing_docs, reason = "deprecated")]
    pub fn persist(self) -> Stream<T, Tick<L>, Bounded, TotalOrder, ExactlyOnce> {
        Stream::new(
            self.location.clone(),
            HydroNode::Persist {
                inner: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata::<T>(),
            },
        )
    }

    #[expect(missing_docs, reason = "TODO")]
    pub fn delta(self) -> Optional<T, Tick<L>, Bounded> {
        Optional::new(
            self.location.clone(),
            HydroNode::Delta {
                inner: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata::<T>(),
            },
        )
    }

    /// Converts this optional into a [`Stream`] containing a single element, the value, if it is
    /// non-null. Otherwise, the stream is empty.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// # let tick = process.tick();
    /// # // ticks are lazy by default, forces the second tick to run
    /// # tick.spin_batch(q!(1)).all_ticks().for_each(q!(|_| {}));
    /// # let batch_first_tick = process
    /// #   .source_iter(q!(vec![]))
    /// #   .batch(&tick, nondet!(/** test */));
    /// # let batch_second_tick = process
    /// #   .source_iter(q!(vec![123, 456]))
    /// #   .batch(&tick, nondet!(/** test */))
    /// #   .defer_tick(); // appears on the second tick
    /// # let input_batch = batch_first_tick.chain(batch_second_tick);
    /// input_batch // first tick: [], second tick: [123, 456]
    ///     .clone()
    ///     .max()
    ///     .into_stream()
    ///     .chain(input_batch)
    ///     .all_ticks()
    /// # }, |mut stream| async move {
    /// // [456, 123, 456]
    /// # for w in vec![456, 123, 456] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// ```
    pub fn into_stream(self) -> Stream<T, Tick<L>, Bounded, TotalOrder, ExactlyOnce> {
        Stream::new(self.location, self.ir_node.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use futures::StreamExt;
    use hydro_deploy::Deployment;
    use stageleft::q;

    use super::Optional;
    use crate::compile::builder::FlowBuilder;
    use crate::location::Location;

    #[tokio::test]
    async fn optional_or_cardinality() {
        let mut deployment = Deployment::new();

        let flow = FlowBuilder::new();
        let node = flow.process::<()>();
        let external = flow.external::<()>();

        let node_tick = node.tick();
        let tick_singleton = node_tick.singleton(q!(123));
        let tick_optional_inhabited: Optional<_, _, _> = tick_singleton.into();
        let counts = tick_optional_inhabited
            .clone()
            .or(tick_optional_inhabited)
            .into_stream()
            .count()
            .all_ticks()
            .send_bincode_external(&external);

        let nodes = flow
            .with_process(&node, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let mut external_out = nodes.connect_source_bincode(counts).await;

        deployment.start().await.unwrap();

        assert_eq!(external_out.next().await.unwrap(), 1);
    }
}
