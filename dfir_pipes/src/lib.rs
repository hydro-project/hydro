#![no_std]
#![cfg_attr(nightly, feature(extend_one))]

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

pub use ::itertools;

mod chain;
mod collect;
mod empty;
mod enumerate;
mod filter;
mod filter_map;
mod flat_map;
mod flatten;
mod for_each;
mod fuse;
mod inspect;
mod iter;
mod map;
mod merge;
mod once;
mod poll_fn;
mod pull_fn;
mod scan;
mod send_sink;
mod skip;
mod skip_while;
mod source_stream;
mod stream;
mod take;
mod take_while;
#[cfg(test)]
mod test_utils;
mod zip_longest;

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

use futures_core::stream::Stream;
use futures_sink::Sink;
pub use itertools::EitherOrBoth;
use sealed::sealed;

/// A sealed trait for type-level booleans used to track pull capabilities.
///
/// `Toggle` is used to statically encode whether a pull can pend (`CanPend`) or end (`CanEnd`).
/// This enables compile-time guarantees about pull behavior and allows the type system to
/// optimize away impossible code paths.
#[sealed]
pub trait Toggle: Sized {
    /// Attempts to convert this type, returning `Err(())` if converting to `No`.
    fn try_convert_from(other: impl Toggle) -> Result<Self, ()>;

    /// Attemps to convert this type, panicking if converting to `No`.
    fn convert_from(other: impl Toggle) -> Self {
        Self::try_convert_from(other).unwrap()
    }

    /// The result of OR-ing two toggles. `Yes.or(T) = Yes`, `No.or(T) = T`.
    type Or<T: Toggle>: Toggle;
    /// The result of AND-ing two toggles. `Yes.and(T) = T`, `No.and(T) = No`.
    type And<T: Toggle>: Toggle;
}

/// Type-level `true` for [`Toggle`].
///
/// Indicates that a capability is present (e.g., the pull can pend or can end).
pub struct Yes;
#[sealed]
impl Toggle for Yes {
    fn try_convert_from(_other: impl Toggle) -> Result<Self, ()> {
        Ok(Yes)
    }

    type Or<T: Toggle> = Yes;
    type And<T: Toggle> = T;
}
#[sealed]
impl Toggle for No {
    fn try_convert_from(_other: impl Toggle) -> Result<Self, ()> {
        Err(())
    }

    type Or<T: Toggle> = T;
    type And<T: Toggle> = No;
}

fn mut_unit<'a>() -> &'a mut () {
    // SAFETY: `UNIT` is a zero-sized type (ZST), so its pointer cannot dangle.
    // https://doc.rust-lang.org/reference/behavior-considered-undefined.html#r-undefined.dangling.zero-size
    unsafe { core::ptr::NonNull::dangling().as_mut() }
}

#[sealed]
pub trait Context<'ctx>: Sized {
    type Merged<Other: Context<'ctx>>: Context<'ctx>;

    fn from_task<'s>(task_ctx: &'s mut core::task::Context<'ctx>) -> &'s mut Self;

    fn unmerge_self<'s, Other: Context<'ctx>>(merged: &'s mut Self::Merged<Other>) -> &'s mut Self;
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
    pub fn try_convert_into<NewPend: Toggle, NewEnd: Toggle>(
        self,
    ) -> Result<Step<Item, Meta, NewPend, NewEnd>, ()> {
        Ok(match self {
            Self::Ready(item, meta) => Step::Ready(item, meta),
            Self::Pending(can_pend) => Step::Pending(Toggle::try_convert_from(can_pend)?),
            Self::Ended(can_end) => Step::Ended(Toggle::try_convert_from(can_end)?),
        })
    }

    pub fn convert_into<NewPend: Toggle, NewEnd: Toggle>(
        self,
    ) -> Step<Item, Meta, NewPend, NewEnd> {
        match self {
            Self::Ready(item, meta) => Step::Ready(item, meta),
            Self::Pending(can_pend) => Step::Pending(Toggle::convert_from(can_pend)),
            Self::Ended(can_end) => Step::Ended(Toggle::convert_from(can_end)),
        }
    }

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
/// (like `SourceStream`) will use `Ctx = &mut Context<'_>`.
///
/// Setting `Ctx = ()` allows most pull pipelines to be used without any context.
pub trait Pull {
    type Ctx<'ctx>: Context<'ctx>;

    type Item;
    type Meta: Copy;
    type CanPend: Toggle;
    type CanEnd: Toggle;

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

    fn by_ref(&mut self) -> &mut Self {
        self
    }

    /// Takes two pulls and creates a new pull over both in sequence.
    ///
    /// `chain()` will return a new pull which will first iterate over
    /// values from the first pull and then over values from the second pull.
    ///
    /// The first pull must be finite (`CanEnd = Yes`) and fused ([`FusedPull`]).
    fn chain<U>(self, other: U) -> chain::Chain<Self, U>
    where
        Self: Sized,
        U: Pull<Item = Self::Item, Meta = Self::Meta>,
    {
        chain::Chain::new(self, other)
    }

    /// Asynchronously merges two pulls, interleaving their items.
    ///
    /// Unlike [`chain`](Self::chain), `merge` does not require either pull to be
    /// finite. Items are pulled from both sources in a round-robin fashion, and
    /// the merged pull only ends when both upstream pulls have ended.
    fn merge<U>(self, other: U) -> merge::Merge<Self, U>
    where
        Self: Sized,
        U: Pull<Item = Self::Item, Meta = Self::Meta>,
    {
        merge::Merge::new(self, other)
    }

    /// Creates a pull which gives the current iteration count as well as the next value.
    ///
    /// The pull returned yields pairs `(i, val)`, where `i` is the current index
    /// of iteration and `val` is the value returned by the pull.
    fn enumerate(self) -> enumerate::Enumerate<Self>
    where
        Self: Sized,
    {
        enumerate::Enumerate::new(self)
    }

    /// Creates a pull which uses a closure to determine if an element should be yielded.
    ///
    /// Given an element the closure must return `true` or `false`. The returned pull
    /// will yield only the elements for which the closure returns `true`.
    fn filter<P>(self, predicate: P) -> filter::Filter<Self, P>
    where
        Self: Sized,
        P: FnMut(&Self::Item) -> bool,
    {
        filter::Filter::new(self, predicate)
    }

    /// Creates a pull that both filters and maps.
    ///
    /// The returned pull yields only the values for which the supplied closure
    /// returns `Some(value)`.
    fn filter_map<B, F>(self, f: F) -> filter_map::FilterMap<Self, F>
    where
        Self: Sized,
        F: FnMut(Self::Item) -> Option<B>,
    {
        filter_map::FilterMap::new(self, f)
    }

    /// Creates a pull that works like map, but flattens nested structure.
    ///
    /// The `flat_map()` method is useful when you have a pull of items, and you
    /// want to apply a function that returns an iterator for each item, then
    /// flatten all those iterators into a single pull.
    fn flat_map<U, F>(self, f: F) -> flat_map::FlatMap<Self, F, U::IntoIter, Self::Meta>
    where
        Self: Sized,
        F: FnMut(Self::Item) -> U,
        U: IntoIterator,
    {
        flat_map::FlatMap::new(self, f)
    }

    /// Creates a pull that flattens nested structure.
    ///
    /// This is useful when you have a pull of iterables, and you want to
    /// flatten them into a single pull.
    fn flatten(self) -> flatten::Flatten<Self, <Self::Item as IntoIterator>::IntoIter, Self::Meta>
    where
        Self: Sized,
        Self::Item: IntoIterator,
    {
        flatten::Flatten::new(self)
    }

    /// Creates a future which runs the given function on each element of a pull.
    fn for_each<F>(self, f: F) -> for_each::ForEach<Self, F>
    where
        Self: Sized,
        F: FnMut(Self::Item),
    {
        for_each::ForEach::new(self, f)
    }

    /// Creates a future which collects all elements of a pull into a collection.
    ///
    /// The collection type `C` must implement `Default` and `Extend<Item>`.
    fn collect<C>(self) -> collect::Collect<Self, C>
    where
        Self: Sized,
        C: Default + Extend<Self::Item>,
    {
        collect::Collect::new(self)
    }

    /// Creates a pull that ends after the first `None`.
    ///
    /// After a pull returns `Ended` for the first time, the behavior of calling
    /// `pull` again is implementation-defined. `fuse()` adapts any pull,
    /// ensuring that after `Ended` is given once, it will always return `Ended`
    /// forever.
    fn fuse(self) -> fuse::Fuse<Self>
    where
        Self: Sized,
    {
        fuse::Fuse::new(self)
    }

    /// Does something with each element of a pull, passing the value on.
    ///
    /// When using pulls, you'll often chain several of them together.
    /// While working on such code, you might want to check out what's
    /// happening at various parts in the pipeline. To do that, insert
    /// a call to `inspect()`.
    fn inspect<F>(self, f: F) -> inspect::Inspect<Self, F>
    where
        Self: Sized,
        F: FnMut(&Self::Item),
    {
        inspect::Inspect::new(self, f)
    }

    /// Takes a closure and creates a pull that calls that closure on each element.
    ///
    /// `map()` transforms one pull into another, by means of its argument: something
    /// that implements `FnMut`. It produces a new pull which calls this closure on
    /// each element of the original pull.
    fn map<B, F>(self, f: F) -> map::Map<Self, F>
    where
        Self: Sized,
        F: FnMut(Self::Item) -> B,
    {
        map::Map::new(self, f)
    }

    /// A pull adapter similar to [`fold`] that holds internal state and
    /// produces a new pull.
    ///
    /// `scan()` takes two arguments: an initial value which seeds the internal
    /// state, and a closure with two arguments, the first being a mutable
    /// reference to the internal state and the second a pull element.
    /// The closure can assign to the internal state to share state between
    /// iterations.
    ///
    /// On iteration, the closure will be applied to each element of the pull
    /// and the return value from the closure, an `Option`, is returned by the
    /// `next` method. Thus the closure can return `Some(value)` to yield `value`,
    /// or `None` to end the iteration.
    ///
    /// [`fold`]: Iterator::fold
    fn scan<St, B, F>(self, initial_state: St, f: F) -> scan::Scan<Self, St, F>
    where
        Self: Sized,
        F: FnMut(&mut St, Self::Item) -> Option<B>,
    {
        scan::Scan::new(self, initial_state, f)
    }

    fn send_sink<Push>(self, push: Push) -> send_sink::SendSink<Self, Push>
    where
        Self: Sized,
        Push: Sink<Self::Item>,
    {
        send_sink::SendSink::new(self, push)
    }

    /// Creates a pull that skips the first `n` elements.
    fn skip(self, n: usize) -> skip::Skip<Self>
    where
        Self: Sized,
    {
        skip::Skip::new(self, n)
    }

    /// Creates a pull that skips elements based on a predicate.
    ///
    /// `skip_while()` takes a closure as an argument. It will call this closure
    /// on each element of the pull, and ignore elements until it returns `false`.
    fn skip_while<P>(self, predicate: P) -> skip_while::SkipWhile<Self, P>
    where
        Self: Sized,
        P: FnMut(&Self::Item) -> bool,
    {
        skip_while::SkipWhile::new(self, predicate)
    }

    /// Creates a pull that yields the first `n` elements, or fewer if the
    /// underlying pull ends sooner.
    fn take(self, n: usize) -> take::Take<Self>
    where
        Self: Sized,
    {
        take::Take::new(self, n)
    }

    /// Creates a pull that yields elements based on a predicate.
    ///
    /// `take_while()` takes a closure as an argument. It will call this closure
    /// on each element of the pull, and yield elements while it returns `true`.
    fn take_while<P>(self, predicate: P) -> take_while::TakeWhile<Self, P>
    where
        Self: Sized,
        P: FnMut(&Self::Item) -> bool,
    {
        take_while::TakeWhile::new(self, predicate)
    }

    /// Zips two pulls together, continuing until both are exhausted.
    ///
    /// Unlike a regular zip which ends when either pull ends, `zip_longest`
    /// continues until both pulls have ended, yielding [`EitherOrBoth`](zip_longest::EitherOrBoth)
    /// values to indicate which pulls yielded items.
    ///
    /// Both pulls must be fused ([`FusedPull`]) to ensure correct behavior
    /// after one pull ends.
    fn zip_longest<U>(self, other: U) -> zip_longest::ZipLongest<Self, U>
    where
        Self: Sized + FusedPull,
        U: FusedPull<Meta = Self::Meta>,
    {
        zip_longest::ZipLongest::new(self, other)
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
/// downstream operators like [`Chain`](chain::Chain) to avoid tracking whether
/// the upstream has ended.
///
/// Implementors should ensure this invariant is upheld. The [`Fuse`](fuse::Fuse)
/// adapter can be used to make any pull fused.
pub trait FusedPull: Pull {}

/// Creates a pull from an iterator.
///
/// This is the primary way to create a pull from synchronous data.
/// The resulting pull will never pend and will end when the iterator is exhausted.
pub fn from_iter<I: IntoIterator>(iter: I) -> iter::Iter<I::IntoIter> {
    iter::Iter::new(iter.into_iter())
}

/// Creates a pull from a `futures::Stream`.
///
/// The resulting pull requires `&mut Context<'_>` to be polled and can both
/// pend and end.
pub fn from_stream<S: Stream>(stream: S) -> stream::Stream<S> {
    stream::Stream::new(stream)
}

/// Creates a pull from a `futures::Stream` with a custom waker.
///
/// This variant uses a provided waker function instead of requiring a context.
/// When the stream returns `Pending`, this pull treats it as ended (non-blocking).
pub fn from_stream_with_waker<S>(stream: S, waker: Waker) -> source_stream::SourceStream<S>
where
    S: Stream,
{
    source_stream::SourceStream::new(stream, waker)
}

/// Creates a pull from a closure.
///
/// The closure is called each time the pull is polled and should return a `Step`.
pub fn from_fn<F, Item, Meta, CanEnd>(func: F) -> pull_fn::PullFn<F, Item, Meta, CanEnd>
where
    F: FnMut() -> Step<Item, Meta, No, CanEnd>,
    CanEnd: Toggle,
{
    pull_fn::PullFn::new(func)
}

/// Creates a pull from a closure.
///
/// The closure is called each time the pull is polled and should return a `Step`.
pub fn from_poll_fn<F, Item, Meta, CanEnd>(func: F) -> poll_fn::PollFn<F, Item, Meta, CanEnd>
where
    F: FnMut(&mut core::task::Context<'_>) -> Step<Item, Meta, Yes, CanEnd>,
    CanEnd: Toggle,
{
    poll_fn::PollFn::new(func)
}

pub fn empty<Item>() -> empty::Empty<Item> {
    empty::Empty::new()
}

pub fn once<Item>(item: Item) -> once::Once<Item> {
    once::Once::new(item)
}
