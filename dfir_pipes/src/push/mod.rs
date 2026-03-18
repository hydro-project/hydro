//! Push-based stream combinators for dataflow pipelines.
//!
//! This module provides push-based operators that mirror the pull-based operators
//! in the parent module, but work in the opposite direction: items are pushed into
//! a pipeline rather than pulled from it.
use core::pin::Pin;
use core::task::Waker;

use crate::{Context, Toggle};

mod fanout;
mod filter;
mod filter_map;
mod flat_map;
mod flatten;
mod for_each;
mod inspect;
mod map;
#[cfg(feature = "alloc")]
#[cfg_attr(docsrs, doc(cfg(feature = "alloc")))]
mod persist;
mod resolve_futures;
mod sink;
mod unzip;

#[cfg(test)]
pub(crate) mod test_utils;

#[cfg(feature = "variadics")]
#[cfg_attr(docsrs, doc(cfg(feature = "variadics")))]
pub mod demux_var;

#[cfg(feature = "variadics")]
#[cfg_attr(docsrs, doc(cfg(feature = "variadics")))]
pub use demux_var::{DemuxVar, PushVariadic, demux_var};
pub use fanout::Fanout;
pub use filter::Filter;
pub use filter_map::FilterMap;
pub use flat_map::FlatMap;
pub use flatten::Flatten;
pub use for_each::ForEach;
use futures_core::FusedStream;
pub use inspect::Inspect;
pub use map::Map;
#[cfg(feature = "alloc")]
#[cfg_attr(docsrs, doc(cfg(feature = "alloc")))]
pub use persist::Persist;
pub use resolve_futures::ResolveFutures;
pub use sink::SinkPush;
pub use unzip::Unzip;

/// The result of pushing an item into a [`Push`].
///
/// `PushStep` represents the two possible outcomes when pushing into a pipeline:
/// - `Done`: The item was successfully consumed.
/// - `Pending(can_pend)`: The push could not accept the item yet (async backpressure).
///
/// The `CanPend` type parameter uses [`Toggle`] to statically encode whether pending
/// is possible. When `CanPend = No`, the `Pending` variant cannot be constructed,
/// and the push is guaranteed to always accept items immediately.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum PushStep<CanPend>
where
    CanPend: Toggle,
{
    /// The item was successfully consumed.
    Done,
    /// The push is not ready yet (only possible when `CanPend = Yes`).
    Pending(CanPend),
}

impl<CanPend> PushStep<CanPend>
where
    CanPend: Toggle,
{
    /// Creates a new `PushStep::Pending`, or panics if `CanPend = No`.
    pub fn pending() -> Self {
        PushStep::Pending(Toggle::create())
    }

    /// Returns `true` if the step is [`PushStep::Done`].
    pub const fn is_done(&self) -> bool {
        matches!(self, PushStep::Done)
    }

    /// Returns `true` if the step is [`PushStep::Pending`].
    pub const fn is_pending(&self) -> bool {
        matches!(self, PushStep::Pending(_))
    }

    /// Tries to convert the `CanPend` type parameter, returning `None` if the conversion is invalid.
    pub fn try_convert_into<NewPend>(self) -> Option<PushStep<NewPend>>
    where
        NewPend: Toggle,
    {
        Some(match self {
            PushStep::Done => PushStep::Done,
            PushStep::Pending(_) => PushStep::Pending(Toggle::try_create()?),
        })
    }

    /// Converts the `CanPend` type parameter, panicking if the conversion is invalid.
    pub fn convert_into<NewPend>(self) -> PushStep<NewPend>
    where
        NewPend: Toggle,
    {
        match self {
            PushStep::Done => PushStep::Done,
            PushStep::Pending(_) => PushStep::pending(),
        }
    }
}

/// The `Push` trait represents a push-based pipeline that items can be sent into.
///
/// This is the dual of [`crate::pull::Pull`]: where `Pull` allows you to request items from
/// a source, `Push` allows you to send items into a sink. Push operators form
/// chains where each operator transforms items and passes them downstream.
///
/// The protocol mirrors [`futures_sink::Sink`]:
/// 1. Call [`Push::poll_ready`] to check if the push can accept an item.
/// 2. If ready, call [`Push::start_send`] to send the item.
/// 3. Call [`Push::poll_flush`] to flush buffered items.
pub trait Push<Item, Meta>
where
    Meta: Copy,
{
    /// The context type required to push into this pipeline.
    type Ctx<'ctx>: Context<'ctx>;

    /// Whether this push can return [`PushStep::Pending`].
    type CanPend: Toggle;

    /// Check if this push is ready to accept an item.
    fn poll_ready(self: Pin<&mut Self>, ctx: &mut Self::Ctx<'_>) -> PushStep<Self::CanPend>;

    /// Send an item into this push pipeline.
    ///
    /// Must only be called after [`Push::poll_ready`] returns [`PushStep::Done`].
    fn start_send(self: Pin<&mut Self>, item: Item, meta: Meta);

    /// Flushes any buffered items in this push pipeline.
    fn poll_flush(self: Pin<&mut Self>, ctx: &mut Self::Ctx<'_>) -> PushStep<Self::CanPend>;
}

impl<P, Item, Meta> Push<Item, Meta> for &mut P
where
    P: Push<Item, Meta> + Unpin + ?Sized,
    Meta: Copy,
{
    type Ctx<'ctx> = P::Ctx<'ctx>;

    type CanPend = P::CanPend;

    fn poll_ready(mut self: Pin<&mut Self>, ctx: &mut Self::Ctx<'_>) -> PushStep<Self::CanPend> {
        Pin::new(&mut **self).poll_ready(ctx)
    }

    fn start_send(mut self: Pin<&mut Self>, item: Item, meta: Meta) {
        Pin::new(&mut **self).start_send(item, meta)
    }

    fn poll_flush(mut self: Pin<&mut Self>, ctx: &mut Self::Ctx<'_>) -> PushStep<Self::CanPend> {
        Pin::new(&mut **self).poll_flush(ctx)
    }
}

/// Evaluates a [`PushStep`] expression and returns [`PushStep::Pending`] if it is pending.
/// Analogous to [`core::task::ready!`] but for [`PushStep`].
macro_rules! ready {
    ($e:expr $(,)?) => {
        match $e {
            PushStep::Done => (),
            PushStep::Pending(_) => return PushStep::pending(),
        }
    };
}
use ready;

/// Evaluates both [`PushStep`] expressions and returns [`PushStep::Pending`] if either is pending.
/// Both expressions are always evaluated so that both sides can do work and/or register wakers.
macro_rules! ready_both {
    ($a:expr, $b:expr $(,)?) => {{
        let a = $a;
        let b = $b;
        $crate::push::ready!(a);
        $crate::push::ready!(b);
    }};
}
use ready_both;

/// Creates a [`Fanout`] push that clones each item and sends to both downstream pushes.
pub const fn fanout<P0, P1>(push0: P0, push1: P1) -> Fanout<P0, P1> {
    Fanout::new(push0, push1)
}

/// Creates a [`Filter`] push that filters items based on a predicate.
pub const fn filter<Func, Item, Next>(func: Func, next: Next) -> Filter<Next, Func>
where
    Func: FnMut(&Item) -> bool,
{
    Filter::new(func, next)
}

/// Creates a [`FilterMap`] push that filters and maps items in one step.
pub const fn filter_map<Func, In, Out, Next>(func: Func, next: Next) -> FilterMap<Next, Func>
where
    Func: FnMut(In) -> Option<Out>,
{
    FilterMap::new(func, next)
}

/// Creates a [`FlatMap`] push that maps each item to an iterator and flattens the results.
pub const fn flat_map<Func, In, IntoIter, Meta, Next>(
    func: Func,
    next: Next,
) -> FlatMap<Next, Func, IntoIter, Meta>
where
    Func: FnMut(In) -> IntoIter,
    IntoIter: IntoIterator,
    Meta: Copy,
    Next: Push<IntoIter::Item, Meta>,
{
    FlatMap::new(func, next)
}

/// Creates a [`Flatten`] push that flattens items that are iterators.
pub const fn flatten<IntoIter, Meta, Next>(next: Next) -> Flatten<Next, IntoIter, Meta>
where
    IntoIter: IntoIterator,
    Meta: Copy,
    Next: Push<IntoIter::Item, Meta>,
{
    Flatten::new(next)
}

/// Creates a [`ForEach`] terminal push that consumes each item with a function.
pub const fn for_each<Func, Item>(func: Func) -> ForEach<Func>
where
    Func: FnMut(Item),
{
    ForEach::new(func)
}

/// Creates an [`Inspect`] push that inspects each item without modifying it.
pub const fn inspect<Func, Item, Next>(func: Func, next: Next) -> Inspect<Next, Func>
where
    Func: FnMut(&Item),
{
    Inspect::new(func, next)
}

/// Creates a [`Map`] push that applies a function to each item.
pub const fn map<Func, In, Out, Next>(func: Func, next: Next) -> Map<Next, Func>
where
    Func: FnMut(In) -> Out,
{
    Map::new(func, next)
}

/// Creates a [`Persist`] using an external `Vec` state for buffering items.
#[cfg(feature = "alloc")]
#[cfg_attr(docsrs, doc(cfg(feature = "alloc")))]
pub fn persist_state<Item, Next>(
    buf: &mut alloc::vec::Vec<Item>,
    replay: bool,
    next: Next,
) -> Persist<Next, &mut alloc::vec::Vec<Item>>
where
    Item: Clone,
    Next: Push<Item, ()>,
{
    Persist::new(buf, replay, next)
}

/// Creates a [`ResolveFutures`] push that resolves futures and sends their outputs.
///
/// The futures queue is supplied as external state.
///
/// `Queue` is generally expected to be either `futures_util::stream::FuturesUnordered`
/// or `futures_util::stream::FuturesOrdered`.
pub const fn resolve_futures_state<Queue, Fut, Next>(
    queue: &mut Queue,
    subgraph_waker: Option<Waker>,
    next: Next,
) -> ResolveFutures<Next, &mut Queue, Queue>
where
    Queue: Default + Extend<Fut> + FusedStream<Item = Fut::Output> + Unpin,
    Fut: Future,
    Next: Push<Fut::Output, ()>,
{
    ResolveFutures::new(queue, subgraph_waker, next)
}

/// Creates an [`Unzip`] push that splits tuple items into two separate pushes.
pub const fn unzip<P0, P1>(push0: P0, push1: P1) -> Unzip<P0, P1> {
    Unzip::new(push0, push1)
}

/// Creates a [`SinkPush`] push that wraps a [`futures_sink::Sink`].
pub const fn sink<Si, Item>(si: Si) -> SinkPush<Si>
where
    Si: futures_sink::Sink<Item>,
{
    SinkPush::new(si)
}
