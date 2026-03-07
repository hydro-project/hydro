//! Pull-based stream combinators for dataflow pipelines.
//!
//! This crate provides a [`Pull`] trait and a collection of composable operators
//! for building pull-based data pipelines. Operators are chained via method calls
//! on [`Pull`], similar to iterator adapters.
#![no_std]
#![cfg_attr(nightly, feature(extend_one))]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs, clippy::missing_const_for_fn)]

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

/// Type-level `false` for [`Toggle`].
///
/// Indicates that a capability is absent (e.g., the pull cannot pend or cannot end).
///
/// A type alias for `core::convert::Infallible`, representing a type that can never be constructed.
///
/// Used in `Step` variants that are statically impossible (e.g., `Pending` when `CanPend = No`).
pub use core::convert::Infallible as No;
use core::pin::Pin;
use core::task::{Poll, Waker};

use futures_sink::Sink;
pub use itertools::{self, Either, EitherOrBoth};
use sealed::sealed;

#[cfg(feature = "std")]
mod accumulator;
mod chain;
mod collect;
mod cross_singleton;
mod either;
mod empty;
mod enumerate;
mod filter;
mod filter_map;
mod flat_map;
mod flatten;
mod for_each;
mod from_fn;
mod fuse;
#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
pub mod half_join_state;
mod inspect;
mod iter;
mod map;
mod next;
mod once;
mod pending;
mod poll_fn;
mod repeat;
mod send_sink;
mod skip;
mod skip_while;
mod stream;
mod stream_ready;
#[cfg(feature = "std")]
mod symmetric_hash_join;
mod take;
mod take_while;
#[cfg(test)]
mod test_utils;
mod zip_longest;

#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
pub use accumulator::{AccumulateAll, Accumulator, Fold, FoldFrom, Reduce, accumulate_all};
pub use chain::Chain;
pub use collect::Collect;
pub use cross_singleton::CrossSingleton;
pub use empty::Empty;
pub use enumerate::Enumerate;
pub use filter::Filter;
pub use filter_map::FilterMap;
pub use flat_map::FlatMap;
pub use flatten::Flatten;
pub use for_each::ForEach;
pub use from_fn::FromFn;
pub use fuse::Fuse;
#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
pub use half_join_state::{HalfJoinState, HalfMultisetJoinState, HalfSetJoinState};
pub use inspect::Inspect;
pub use iter::Iter;
pub use map::Map;
pub use next::Next;
pub use once::Once;
pub use pending::Pending;
pub use poll_fn::PollFn;
pub use repeat::Repeat;
pub use send_sink::SendSink;
pub use skip::Skip;
pub use skip_while::SkipWhile;
pub use stream::Stream;
pub use stream_ready::StreamReady;
#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
pub use symmetric_hash_join::{
    NewTickJoinIter, NewTickJoinPull, SymmetricHashJoin, SymmetricHashJoinEither,
    symmetric_hash_join,
};
pub use take::Take;
pub use take_while::TakeWhile;
pub use zip_longest::ZipLongest;

/// A sealed trait for type-level booleans used to track pull capabilities.
///
/// `Toggle` is used to statically encode whether a pull can pend (`CanPend`) or end (`CanEnd`).
/// This enables compile-time guarantees about pull behavior and allows the type system to
/// optimize away impossible code paths.
#[sealed]
pub trait Toggle: Sized {
    /// Attempts to create this type, returning `Err(())` if `Self` is `No`.
    fn try_create() -> Option<Self>;

    /// Attempts to create this type, panicking if `Self` is `No`.
    fn create() -> Self {
        Self::try_create().unwrap()
    }

    /// The result of OR-ing two toggles. `Yes.or(T) = Yes`, `No.or(T) = T`.
    type Or<T: Toggle>: Toggle;
    /// The result of AND-ing two toggles. `Yes.and(T) = T`, `No.and(T) = No`.
    type And<T: Toggle>: Toggle;
}

/// Type-level `true` for [`Toggle`].
///
/// Indicates that a capability is present (e.g., the pull can pend or can end).
#[derive(Default)]
pub struct Yes;

#[sealed]
impl Toggle for Yes {
    fn try_create() -> Option<Self> {
        Some(Yes)
    }

    type Or<T: Toggle> = Yes;
    type And<T: Toggle> = T;
}

#[sealed]
impl Toggle for No {
    fn try_create() -> Option<Self> {
        None
    }

    type Or<T: Toggle> = T;
    type And<T: Toggle> = No;
}

const fn mut_unit<'a>() -> &'a mut () {
    // SAFETY: `UNIT` is a zero-sized type (ZST), so its pointer cannot dangle.
    // https://doc.rust-lang.org/reference/behavior-considered-undefined.html#r-undefined.dangling.zero-size
    unsafe { core::ptr::NonNull::dangling().as_mut() }
}

/// Context trait for pull-based streams, allowing operators to be generic over
/// synchronous (`()`) and asynchronous ([`core::task::Context`]) execution contexts.
#[sealed]
pub trait Context<'ctx>: Sized {
    /// The merged context type when combining two pulls.
    type Merged<Other: Context<'ctx>>: Context<'ctx>;

    /// Creates a context reference from a [`core::task::Context`].
    fn from_task<'s>(task_ctx: &'s mut core::task::Context<'ctx>) -> &'s mut Self;

    /// Extracts the self-side context from a merged context.
    fn unmerge_self<'s, Other: Context<'ctx>>(merged: &'s mut Self::Merged<Other>) -> &'s mut Self;
    /// Extracts the other-side context from a merged context.
    fn unmerge_other<'s, Other: Context<'ctx>>(
        merged: &'s mut Self::Merged<Other>,
    ) -> &'s mut Other;
}

#[sealed]
impl<'ctx> Context<'ctx> for () {
    type Merged<Other: Context<'ctx>> = Other;

    fn from_task<'s>(_task_ctx: &'s mut core::task::Context<'ctx>) -> &'s mut Self {
        mut_unit()
    }

    fn unmerge_self<'s, Other: Context<'ctx>>(
        _merged: &'s mut Self::Merged<Other>,
    ) -> &'s mut Self {
        mut_unit()
    }
    fn unmerge_other<'s, Other: Context<'ctx>>(
        merged: &'s mut Self::Merged<Other>,
    ) -> &'s mut Other {
        merged
    }
}

#[sealed]
impl<'ctx> Context<'ctx> for core::task::Context<'ctx> {
    type Merged<Other: Context<'ctx>> = core::task::Context<'ctx>;

    fn from_task<'s>(task_ctx: &'s mut core::task::Context<'ctx>) -> &'s mut Self {
        task_ctx
    }

    fn unmerge_self<'s, Other: Context<'ctx>>(merged: &'s mut Self::Merged<Other>) -> &'s mut Self {
        merged
    }
    fn unmerge_other<'s, Other: Context<'ctx>>(
        merged: &'s mut Self::Merged<Other>,
    ) -> &'s mut Other {
        Other::from_task(merged)
    }
}

/// The result of polling a [`Pull`].
///
/// `Step` represents the three possible outcomes when pulling from a stream:
/// - `Ready(item, meta)`: An item is available along with associated metadata.
/// - `Pending(can_pend)`: No item is available yet, but more may come (async).
/// - `Ended(can_end)`: The stream has terminated and will produce no more items.
///
/// The `CanPend` and `CanEnd` type parameters use [`Toggle`] to statically encode
/// which variants are possible. When a variant is impossible (e.g., `CanPend = No`),
/// its payload type becomes [`No`], making it a compile error to construct.
pub enum Step<Item, Meta, CanPend: Toggle, CanEnd: Toggle> {
    /// An item is ready with associated metadata.
    Ready(Item, Meta),
    /// The pull is not ready yet (only possible when `CanPend = Yes`).
    Pending(CanPend),
    /// The pull has ended (only possible when `CanEnd = Yes`).
    Ended(CanEnd),
}

impl<Item, Meta, CanPend: Toggle, CanEnd: Toggle> Step<Item, Meta, CanPend, CanEnd> {
    /// Creates a new `Step::Ended`, or panics if `CanEnd = No`.
    pub fn ended() -> Self {
        Step::Ended(Toggle::create())
    }

    /// Creates a new `Step::Pending`, or panics if `CanPend = No`.
    pub fn pending() -> Self {
        Step::Pending(Toggle::create())
    }

    /// Returns `true` if the step is a [`Step::Ended`].
    pub const fn is_ended(&self) -> bool {
        matches!(self, Step::Ended(_))
    }

    /// Returns `true` if the step is a [`Step::Pending`].
    pub const fn is_pending(&self) -> bool {
        matches!(self, Step::Pending(_))
    }

    /// Tries to convert the `CanPend` and `CanEnd` type parameters, returning `None` if the conversion is invalid.
    pub fn try_convert_into<NewPend: Toggle, NewEnd: Toggle>(
        self,
    ) -> Option<Step<Item, Meta, NewPend, NewEnd>> {
        Some(match self {
            Self::Ready(item, meta) => Step::Ready(item, meta),
            Self::Pending(_) => Step::Pending(Toggle::try_create()?),
            Self::Ended(_) => Step::Ended(Toggle::try_create()?),
        })
    }

    /// Converts the `CanPend` and `CanEnd` type parameters, panicking if the conversion is invalid.
    pub fn convert_into<NewPend: Toggle, NewEnd: Toggle>(
        self,
    ) -> Step<Item, Meta, NewPend, NewEnd> {
        match self {
            Self::Ready(item, meta) => Step::Ready(item, meta),
            Self::Pending(_) => Step::pending(),
            Self::Ended(_) => Step::ended(),
        }
    }

    /// Converts this `Step` into a [`Poll`]`<Option<(Item, Meta)>>`.
    pub fn into_poll(self) -> Poll<Option<(Item, Meta)>> {
        match self {
            Step::Ready(item, meta) => Poll::Ready(Some((item, meta))),
            Step::Pending(_) => Poll::Pending,
            Step::Ended(_) => Poll::Ready(None),
        }
    }
}

/// The `Pull` trait represents a pull-based stream that can be polled for items.
///
/// The `Ctx` type parameter allows operators to be generic over the context type.
/// Most operators don't use the context and just forward it to their predecessor,
/// so they can be generic over `Ctx`. Operators that need `std::task::Context`
/// (like `StreamReady`) will use `Ctx = &mut Context<'_>`.
///
/// Setting `Ctx = ()` allows most pull pipelines to be used without any context.
pub trait Pull {
    /// The context type required to poll this pull.
    type Ctx<'ctx>: Context<'ctx>;

    /// The type of items yielded by this pull.
    type Item;
    /// The metadata type associated with each item.
    type Meta: Copy;
    /// Whether this pull can return [`Step::Pending`].
    type CanPend: Toggle;
    /// Whether this pull can return [`Step::Ended`].
    type CanEnd: Toggle;

    /// Attempts to pull the next item from this stream.
    fn pull(
        self: Pin<&mut Self>,
        ctx: &mut Self::Ctx<'_>,
    ) -> Step<Self::Item, Self::Meta, Self::CanPend, Self::CanEnd>;

    /// Returns the bounds on the remaining length of the pull.
    ///
    /// Specifically, `size_hint()` returns a tuple where the first element
    /// is the lower bound, and the second element is the upper bound.
    ///
    /// The second half of the tuple that is returned is an [`Option`]`<`[`usize`]`>`.
    /// A [`None`] here means that either there is no known upper bound, or the
    /// upper bound is larger than [`usize`].
    ///
    /// # Implementation notes
    ///
    /// It is not enforced that a pull implementation yields the declared
    /// number of elements. A buggy pull may yield less than the lower bound
    /// or more than the upper bound of elements.
    ///
    /// `size_hint()` is primarily intended to be used for optimizations such as
    /// reserving space for the elements of the pull, but must not be trusted
    /// to e.g., omit bounds checks in unsafe code. An incorrect implementation
    /// of `size_hint()` should not lead to memory safety violations.
    ///
    /// That said, the implementation should provide a correct estimation,
    /// because otherwise it would be a violation of the trait's protocol.
    ///
    /// The default implementation returns `(0, None)` which is correct for any
    /// pull.
    #[inline]
    fn size_hint(self: Pin<&Self>) -> (usize, Option<usize>) {
        (0, None)
    }

    /// Borrows this pull, allowing it to be used by reference.
    fn by_ref(&mut self) -> &mut Self {
        self
    }

    /// Takes two pulls and creates a new pull over both in sequence.
    ///
    /// `chain()` will return a new pull which will first iterate over
    /// values from the first pull and then over values from the second pull.
    ///
    /// The first pull must be finite (`CanEnd = Yes`) and fused ([`FusedPull`]).
    fn chain<U>(self, other: U) -> Chain<Self, U>
    where
        Self: Sized + FusedPull<CanEnd = Yes>,
        U: Pull<Item = Self::Item, Meta = Self::Meta>,
    {
        Chain::new(self, other)
    }

    /// Creates a pull which gives the current iteration count as well as the next value.
    ///
    /// The pull returned yields pairs `(i, val)`, where `i` is the current index
    /// of iteration and `val` is the value returned by the pull.
    fn enumerate(self) -> Enumerate<Self>
    where
        Self: Sized,
    {
        Enumerate::new(self)
    }

    /// Creates a pull which uses a closure to determine if an element should be yielded.
    ///
    /// Given an element the closure must return `true` or `false`. The returned pull
    /// will yield only the elements for which the closure returns `true`.
    fn filter<P>(self, predicate: P) -> Filter<Self, P>
    where
        Self: Sized,
        P: FnMut(&Self::Item) -> bool,
    {
        Filter::new(self, predicate)
    }

    /// Creates a pull that both filters and maps.
    ///
    /// The returned pull yields only the values for which the supplied closure
    /// returns `Some(value)`.
    fn filter_map<B, F>(self, f: F) -> FilterMap<Self, F>
    where
        Self: Sized,
        F: FnMut(Self::Item) -> Option<B>,
    {
        FilterMap::new(self, f)
    }

    /// Creates a pull that works like map, but flattens nested structure.
    ///
    /// The `flat_map()` method is useful when you have a pull of items, and you
    /// want to apply a function that returns an iterator for each item, then
    /// flatten all those iterators into a single pull.
    fn flat_map<U, F>(self, f: F) -> FlatMap<Self, F, U::IntoIter, Self::Meta>
    where
        Self: Sized,
        F: FnMut(Self::Item) -> U,
        U: IntoIterator,
    {
        FlatMap::new(self, f)
    }

    /// Creates a pull that flattens nested structure.
    ///
    /// This is useful when you have a pull of iterables, and you want to
    /// flatten them into a single pull.
    fn flatten(self) -> Flatten<Self, <Self::Item as IntoIterator>::IntoIter, Self::Meta>
    where
        Self: Sized,
        Self::Item: IntoIterator,
    {
        Flatten::new(self)
    }

    /// Creates a future which runs the given function on each element of a pull.
    fn for_each<F>(self, f: F) -> ForEach<Self, F>
    where
        Self: Sized,
        F: FnMut(Self::Item),
    {
        ForEach::new(self, f)
    }

    /// Creates a future which collects all elements of a pull into a collection.
    ///
    /// The collection type `C` must implement `Default` and `Extend<Item>`.
    fn collect<C>(self) -> Collect<Self, C>
    where
        Self: Sized,
        C: Default + Extend<Self::Item>,
    {
        Collect::new(self)
    }

    /// Creates a pull that ends after the first `None`.
    ///
    /// After a pull returns `Ended` for the first time, the behavior of calling
    /// `pull` again is implementation-defined. `fuse()` adapts any pull,
    /// ensuring that after `Ended` is given once, it will always return `Ended`
    /// forever.
    fn fuse(
        self,
    ) -> impl for<'ctx> FusedPull<
        Ctx<'ctx> = Self::Ctx<'ctx>,
        Item = Self::Item,
        Meta = Self::Meta,
        CanPend = Self::CanPend,
        CanEnd = Self::CanEnd,
    >
    where
        Self: Sized,
    {
        Fuse::new(self)
    }

    /// Does something with each element of a pull, passing the value on.
    ///
    /// When using pulls, you'll often chain several of them together.
    /// While working on such code, you might want to check out what's
    /// happening at various parts in the pipeline. To do that, insert
    /// a call to `inspect()`.
    fn inspect<F>(self, f: F) -> Inspect<Self, F>
    where
        Self: Sized,
        F: FnMut(&Self::Item),
    {
        Inspect::new(self, f)
    }

    /// Takes a closure and creates a pull that calls that closure on each element.
    ///
    /// `map()` transforms one pull into another, by means of its argument: something
    /// that implements `FnMut`. It produces a new pull which calls this closure on
    /// each element of the original pull.
    fn map<B, F>(self, f: F) -> Map<Self, F>
    where
        Self: Sized,
        F: FnMut(Self::Item) -> B,
    {
        Map::new(self, f)
    }

    /// Creates a future that pulls all items and sends them into a [`Sink`].
    fn send_sink<Push>(self, push: Push) -> SendSink<Self, Push>
    where
        Self: Sized,
        Push: Sink<Self::Item>,
    {
        SendSink::new(self, push)
    }

    /// Creates a pull that skips the first `n` elements.
    fn skip(self, n: usize) -> Skip<Self>
    where
        Self: Sized,
    {
        Skip::new(self, n)
    }

    /// Creates a pull that skips elements based on a predicate.
    ///
    /// `skip_while()` takes a closure as an argument. It will call this closure
    /// on each element of the pull, and ignore elements until it returns `false`.
    fn skip_while<P>(self, predicate: P) -> SkipWhile<Self, P>
    where
        Self: Sized,
        P: FnMut(&Self::Item) -> bool,
    {
        SkipWhile::new(self, predicate)
    }

    /// Creates a pull that yields the first `n` elements, or fewer if the
    /// underlying pull ends sooner.
    fn take(self, n: usize) -> Take<Self>
    where
        Self: Sized,
    {
        Take::new(self, n)
    }

    /// Creates a pull that yields elements based on a predicate.
    ///
    /// `take_while()` takes a closure as an argument. It will call this closure
    /// on each element of the pull, and yield elements while it returns `true`.
    fn take_while<P>(self, predicate: P) -> TakeWhile<Self, P>
    where
        Self: Sized,
        P: FnMut(&Self::Item) -> bool,
    {
        TakeWhile::new(self, predicate)
    }

    /// Zips two pulls together, continuing until both are exhausted.
    ///
    /// Unlike a regular zip which ends when either pull ends, `zip_longest`
    /// continues until both pulls have ended, yielding [`EitherOrBoth`]
    /// values to indicate which pulls yielded items.
    ///
    /// Both pulls must be fused ([`FusedPull`]) to ensure correct behavior
    /// after one pull ends.
    fn zip_longest<U>(self, other: U) -> ZipLongest<Self, U>
    where
        Self: Sized + FusedPull,
        U: FusedPull<Meta = Self::Meta>,
    {
        ZipLongest::new(self, other)
    }

    /// Creates a future that resolves with the next item from this pull.
    ///
    /// This is the `Pull` equivalent of the `StreamExt::next()` future.
    fn next(self) -> Next<Self>
    where
        Self: Sized,
    {
        Next::new(self)
    }

    /// Crosses each item from this pull with a singleton value from another pull.
    ///
    /// The singleton value is obtained from the first item of `singleton_pull` and cached.
    /// All subsequent items from this pull are paired with this cached singleton value.
    ///
    /// If `singleton_pull` ends before yielding any items, the entire combinator ends immediately.
    fn cross_singleton<SinglePull>(
        self,
        singleton_pull: SinglePull,
    ) -> CrossSingleton<Self, SinglePull, Option<SinglePull::Item>>
    where
        Self: Sized,
        SinglePull: Pull,
        SinglePull::Item: Clone,
    {
        CrossSingleton::new(self, singleton_pull, None)
    }

    /// [Self::cross_singleton] with external state.
    fn cross_singleton_state<SinglePull>(
        self,
        singleton_pull: SinglePull,
        singleton_state: &mut Option<SinglePull::Item>,
    ) -> CrossSingleton<Self, SinglePull, &mut Option<SinglePull::Item>>
    where
        Self: Sized,
        SinglePull: Pull,
        SinglePull::Item: Clone,
    {
        CrossSingleton::new(self, singleton_pull, singleton_state)
    }

    /// Performs a symmetric hash join with another pull.
    ///
    /// Joins items from this pull with items from `rhs` based on a common key.
    /// Both pulls must yield `(Key, Value)` tuples. The result is a pull of
    /// `(Key, (V1, V2))` tuples for each matching pair.
    ///
    /// The `lhs_state` and `rhs_state` parameters store the join state and must
    /// implement [`HalfJoinState`].
    #[cfg(feature = "std")]
    #[cfg_attr(docsrs, doc(cfg(feature = "std")))]
    fn symmetric_hash_join<Key, V1, Rhs, V2, LhsState, RhsState>(
        self,
        rhs: Rhs,
        lhs_state: LhsState,
        rhs_state: RhsState,
    ) -> SymmetricHashJoin<Self, Rhs, LhsState, RhsState, LhsState, RhsState>
    where
        Self: Sized + FusedPull<Item = (Key, V1), Meta = ()>,
        Key: Eq + std::hash::Hash + Clone,
        V1: Clone,
        V2: Clone,
        Rhs: FusedPull<Item = (Key, V2), Meta = ()>,
        LhsState: HalfJoinState<Key, V1, V2>,
        RhsState: HalfJoinState<Key, V2, V1>,
    {
        SymmetricHashJoin::new(self, rhs, lhs_state, rhs_state)
    }

    /// [Self::symmetric_hash_join] with external state.
    #[cfg(feature = "std")]
    #[cfg_attr(docsrs, doc(cfg(feature = "std")))]
    fn symmetric_hash_join_state<'a, Key, V1, Rhs, V2, LhsState, RhsState>(
        self,
        rhs: Rhs,
        lhs_state: &'a mut LhsState,
        rhs_state: &'a mut RhsState,
    ) -> SymmetricHashJoin<Self, Rhs, &'a mut LhsState, &'a mut RhsState, LhsState, RhsState>
    where
        Self: Sized + FusedPull<Item = (Key, V1), Meta = ()>,
        Key: Eq + std::hash::Hash + Clone,
        V1: Clone,
        V2: Clone,
        Rhs: FusedPull<Item = (Key, V2), Meta = ()>,
        LhsState: HalfJoinState<Key, V1, V2>,
        RhsState: HalfJoinState<Key, V2, V1>,
    {
        SymmetricHashJoin::new(self, rhs, lhs_state, rhs_state)
    }
}

impl<P> Pull for &mut P
where
    P: Pull + Unpin + ?Sized,
{
    type Ctx<'ctx> = P::Ctx<'ctx>;

    type Item = P::Item;
    type Meta = P::Meta;
    type CanPend = P::CanPend;
    type CanEnd = P::CanEnd;

    fn pull(
        mut self: Pin<&mut Self>,
        ctx: &mut Self::Ctx<'_>,
    ) -> Step<Self::Item, Self::Meta, Self::CanPend, Self::CanEnd> {
        Pin::new(&mut **self).pull(ctx)
    }
}

/// A marker trait for pulls that are "fused".
///
/// A fused pull guarantees that once it returns [`Step::Ended`], all subsequent
/// calls to [`Pull::pull`] will also return [`Step::Ended`]. This property allows
/// downstream operators like [`Pull::chain`] to avoid tracking whether
/// the upstream has ended.
///
/// Implementors should ensure this invariant is upheld. The [`Pull::fuse`]
/// adapter can be used to make any pull fused.
pub trait FusedPull: Pull {}

/// Creates a pull from an iterator.
///
/// This is the primary way to create a pull from synchronous data.
/// The resulting pull will never pend and will end when the iterator is exhausted.
pub fn iter<I: IntoIterator>(iter: I) -> Iter<I::IntoIter> {
    Iter::new(iter.into_iter())
}

/// Creates a pull from a `futures::Stream`.
///
/// The resulting pull requires `&mut Context<'_>` to be polled and can both
/// pend and end.
pub const fn stream<S: futures_core::stream::Stream>(stream: S) -> Stream<S> {
    Stream::new(stream)
}

/// Creates a pull from a `futures::Stream` with a custom waker.
///
/// This variant uses a provided waker function instead of requiring a context.
/// When the stream returns `Pending`, this pull treats it as ended (non-blocking).
pub const fn stream_ready<S>(stream: S, waker: Waker) -> StreamReady<S>
where
    S: futures_core::stream::Stream,
{
    StreamReady::new(stream, waker)
}

/// Creates a synchronous pull from a closure.
///
/// The closure is called each time the pull is polled and should return a `Step`.
pub fn from_fn<F, Item, Meta, CanEnd>(func: F) -> FromFn<F, Item, Meta, CanEnd>
where
    F: FnMut() -> Step<Item, Meta, No, CanEnd>,
    Meta: Copy,
    CanEnd: Toggle,
{
    FromFn::new(func)
}

/// Creates an asynchronous or synchronous pull from a closure.
///
/// The closure is called each time the pull is polled and should return a `Step`.
pub fn poll_fn<F, Item, Meta, CanPend, CanEnd>(func: F) -> PollFn<F, Item, Meta, CanPend, CanEnd>
where
    F: FnMut(&mut core::task::Context<'_>) -> Step<Item, Meta, CanPend, CanEnd>,
    Meta: Copy,
    CanPend: Toggle,
    CanEnd: Toggle,
{
    PollFn::new(func)
}

/// Creates an empty pull that immediately ends.
pub fn empty<Item>() -> Empty<Item> {
    Empty::default()
}

/// Creates a pull that yields a single item.
pub const fn once<Item>(item: Item) -> Once<Item> {
    Once::new(item)
}

/// Creates a pull that yields clones of the given item forever.
pub const fn repeat<Item>(item: Item) -> Repeat<Item>
where
    Item: Clone,
{
    Repeat::new(item)
}

/// Creates a pull that is always pending and never yields items or ends.
pub fn pending<Item>() -> Pending<Item> {
    Pending::default()
}

/// A macro to override `Pull::fuse` for pulls that are already fused.
///
/// This macro should be used in the `impl` block for a pull type that is already fused.
/// It provides a default implementation of `fuse` that simply returns `self`.
#[macro_export]
macro_rules! fuse_self {
    () => {
        fn fuse(
            self,
        ) -> impl for<'ctx> FusedPull<
            Ctx<'ctx> = Self::Ctx<'ctx>,
            Item = Self::Item,
            Meta = Self::Meta,
            CanPend = Self::CanPend,
            CanEnd = Self::CanEnd,
        >
        where
            Self: Sized,
        {
            self
        }
    };
}
