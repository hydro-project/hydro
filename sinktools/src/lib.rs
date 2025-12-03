#![doc = include_str!("../README.md")]
#![cfg_attr(not(any(test, feature = "std")), no_std)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]

use core::marker::PhantomData;

use futures_util::Stream;
pub use futures_util::sink;
pub use futures_util::sink::Sink;
#[cfg(feature = "variadics")]
#[cfg_attr(docsrs, doc(cfg(feature = "variadics")))]
pub use variadics;

pub mod filter;
pub mod filter_map;
pub mod flat_map;
pub mod flatten;
pub mod for_each;
pub mod inspect;
pub mod lazy;
pub mod lazy_sink_source;
pub mod map;
pub mod send_iter;
pub mod send_stream;
pub mod try_for_each;
pub mod unzip;

use filter::Filter;
use filter_map::FilterMap;
use flat_map::FlatMap;
use flatten::Flatten;
use for_each::ForEach;
use inspect::Inspect;
use map::Map;
use send_iter::SendIter;
use send_stream::SendStream;
use try_for_each::TryForEach;
use unzip::Unzip;

#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
pub mod demux_map;
#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
pub use demux_map::demux_map;

#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
pub mod demux_map_lazy;
#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
pub use demux_map_lazy::demux_map_lazy;

#[cfg(feature = "variadics")]
#[cfg_attr(docsrs, doc(cfg(feature = "variadics")))]
pub mod demux_var;
#[cfg(feature = "variadics")]
#[cfg_attr(docsrs, doc(cfg(feature = "variadics")))]
pub use demux_var::{SinkVariadic, demux_var};

/// A helper trait for building [`Sink`]s in forward order, unlike with `Sinktools`.
///
/// To start a sink adaptor chain, use [`SinkBuilder`].
pub trait SinkBuild {
    /// The output item type.
    type Item;

    /// The output [`Sink`] type, if it is prepended to `Next`.
    type Output<Next: Sink<Self::Item>>;
    /// Complete this sink adaptor chain by connecting it to `next` as the output.
    ///
    /// This method may be used directly if you're trying to connect to an existing sink. Otherwise, use methods like
    /// `Self::for_each` directly to complete a chain.
    fn send_to<Next>(self, next: Next) -> Self::Output<Next>
    where
        Next: Sink<Self::Item>;

    /// Clone each item and send to both `sink0` and `sink1`, completing this sink adaptor chain.
    fn fanout<Si0, Si1>(self, sink0: Si0, sink1: Si1) -> Self::Output<sink::Fanout<Si0, Si1>>
    where
        Self: Sized,
        Self::Item: Clone,
        Si0: Sink<Self::Item>,
        Si1: Sink<Self::Item, Error = Si0::Error>,
    {
        self.send_to(sink::SinkExt::fanout(sink0, sink1))
    }

    /// Appends a function which consumes each element, completing this sink adaptor chain.
    fn for_each<Func>(self, func: Func) -> Self::Output<ForEach<Func>>
    where
        Self: Sized,
        Func: FnMut(Self::Item),
    {
        self.send_to(ForEach::new(func))
    }

    /// Appends a function which consumes each element and returns a result, completing this sink
    /// adaptor chain.
    fn try_for_each<Func, Error>(self, func: Func) -> Self::Output<TryForEach<Func>>
    where
        Self: Sized,
        Func: FnMut(Self::Item) -> Result<(), Error>,
    {
        self.send_to(TryForEach::new(func))
    }

    /// Appends a function which is called on each element and pases along each output.
    fn map<Func, Out>(self, func: Func) -> map::MapBuilder<Self, Func>
    where
        Self: Sized,
        Func: FnMut(Self::Item) -> Out,
    {
        map::MapBuilder { prev: self, func }
    }

    /// Appends a predicate function which filters items.
    fn filter<Func>(self, func: Func) -> filter::FilterBuilder<Self, Func>
    where
        Self: Sized,
        Func: FnMut(&Self::Item) -> bool,
    {
        filter::FilterBuilder { prev: self, func }
    }

    /// Appends a function which both filters and maps items.
    fn filter_map<Func, Out>(self, func: Func) -> filter_map::FilterMapBuilder<Self, Func>
    where
        Self: Sized,
        Func: FnMut(Self::Item) -> Option<Out>,
    {
        filter_map::FilterMapBuilder { prev: self, func }
    }

    /// Appends a function which maps each item to an iterator and flattens the results.
    fn flat_map<Func, IntoIter>(self, func: Func) -> flat_map::FlatMapBuilder<Self, Func>
    where
        Self: Sized,
        Func: FnMut(Self::Item) -> IntoIter,
        IntoIter: IntoIterator,
    {
        flat_map::FlatMapBuilder { prev: self, func }
    }

    /// Flattens items that are iterators.
    fn flatten<IntoIter>(self) -> flatten::FlattenBuilder<Self>
    where
        Self: Sized,
        Self::Item: IntoIterator,
    {
        flatten::FlattenBuilder { prev: self }
    }

    /// Appends a function which inspects each item without modifying it.
    fn inspect<Func>(self, func: Func) -> inspect::InspectBuilder<Self, Func>
    where
        Self: Sized,
        Func: FnMut(&Self::Item),
    {
        inspect::InspectBuilder { prev: self, func }
    }

    /// Splits items into two sinks based on tuple structure.
    fn unzip<Si0, Si1, Item0, Item1>(self, sink0: Si0, sink1: Si1) -> Self::Output<Unzip<Si0, Si1>>
    where
        Self: Sized + SinkBuild<Item = (Item0, Item1)>,
        Si0: Sink<Item0>,
        Si1: Sink<Item1>,
        Si0::Error: From<Si1::Error>,
    {
        self.send_to(Unzip::new(sink0, sink1))
    }

    /// Sends each item into one sink depending on the key, where the sinks are in a [`HashMap`](std::collections::HashMap).
    ///
    /// This requires sinks `Si` to be `Unpin`. If your sinks are not `Unpin`, first wrap them in `Box::pin` to make them `Unpin`.
    #[cfg(feature = "std")]
    #[cfg_attr(docsrs, doc(cfg(feature = "std")))]
    fn demux_map<Key, ItemVal, Si>(
        self,
        sinks: impl Into<std::collections::HashMap<Key, Si>>,
    ) -> Self::Output<demux_map::DemuxMap<Key, Si>>
    where
        Self: Sized + SinkBuild<Item = (Key, ItemVal)>,
        Key: Eq + core::hash::Hash + core::fmt::Debug + Unpin,
        Si: Sink<ItemVal> + Unpin,
    {
        self.send_to(demux_map(sinks))
    }

    /// Sends each item into one sink depending on the key, lazily creating sinks on first use.
    ///
    /// This requires sinks `Si` to be `Unpin`. If your sinks are not `Unpin`, first wrap them in `Box::pin` to make them `Unpin`.
    #[cfg(feature = "std")]
    #[cfg_attr(docsrs, doc(cfg(feature = "std")))]
    fn demux_map_lazy<Key, ItemVal, Si, Func>(
        self,
        func: Func,
    ) -> Self::Output<demux_map_lazy::LazyDemuxSink<Key, Si, Func>>
    where
        Self: Sized + SinkBuild<Item = (Key, ItemVal)>,
        Key: Eq + core::hash::Hash + core::fmt::Debug + Unpin,
        Si: Sink<ItemVal> + Unpin,
        Func: FnMut(&Key) -> Si + Unpin,
    {
        self.send_to(demux_map_lazy(func))
    }

    /// Sends each item into one sink depending on the index, where the sinks are a variadic.
    #[cfg(feature = "variadics")]
    #[cfg_attr(docsrs, doc(cfg(feature = "variadics")))]
    fn demux_var<Sinks, ItemVal, Error>(
        self,
        sinks: Sinks,
    ) -> Self::Output<demux_var::DemuxVar<Sinks, Error>>
    where
        Self: Sized + SinkBuild<Item = (usize, ItemVal)>,
        Sinks: SinkVariadic<ItemVal, Error>,
    {
        self.send_to(demux_var(sinks))
    }
}

/// Start a [`SinkBuild`] adaptor chain, with `Item` as the input item type.
pub struct SinkBuilder<Item>(PhantomData<fn() -> Item>);
impl<Item> Default for SinkBuilder<Item> {
    fn default() -> Self {
        Self(PhantomData)
    }
}
impl<Item> SinkBuilder<Item> {
    /// Create a new sink builder.
    pub fn new() -> Self {
        Self::default()
    }
}
impl<Item> SinkBuild for SinkBuilder<Item> {
    type Item = Item;

    type Output<Next: Sink<Self::Item>> = Next;
    fn send_to<Next>(self, next: Next) -> Self::Output<Next>
    where
        Next: Sink<Self::Item>,
    {
        next
    }
}

/// Blanket trait for sending items from `Self` into a [`SinkBuild`].
pub trait ToSinkBuild {
    /// Starts a [`SinkBuild`] adaptor chain to send all items from `self` as an [`Iterator`].
    fn iter_to_sink_build(self) -> send_iter::SendIterBuild<Self>
    where
        Self: Sized + Iterator,
    {
        send_iter::SendIterBuild { iter: self }
    }

    /// Starts a [`SinkBuild`] adaptor chain to send all items from `self` as a [`Stream`].
    fn stream_to_sink_build(self) -> send_stream::SendStreamBuild<Self>
    where
        Self: Sized + Stream,
    {
        send_stream::SendStreamBuild { stream: self }
    }
}
impl<T> ToSinkBuild for T {}

/// Forwards sink methods to `self.project().sink`.
macro_rules! forward_sink {
    (
        $( $method:ident ),+
    ) => {
        $(
            fn $method(self: ::core::pin::Pin<&mut Self>, cx: &mut ::core::task::Context<'_>) -> ::core::task::Poll<::core::result::Result<(), Self::Error>> {
                self.project().sink.$method(cx)
            }
        )+
    }
}
use forward_sink;

/// Evaluates both `Poll<()>` expressions and returns `Poll::Pending` if either is pending.
macro_rules! ready_both {
    ($a:expr, $b:expr $(,)?) => {
        if !matches!(
            ($a, $b),
            (::core::task::Poll::Ready(()), ::core::task::Poll::Ready(())),
        ) {
            return ::core::task::Poll::Pending;
        }
    };
}
use ready_both;

/// Creates a [`Map`] sink that applies a function to each item.
pub fn map<Func, In, Out, Si>(func: Func, sink: Si) -> Map<Si, Func>
where
    Func: FnMut(In) -> Out,
    Si: Sink<Out>,
{
    Map::new(func, sink)
}

/// Creates a [`Filter`] sink that filters items based on a predicate.
pub fn filter<Func, Item, Si>(func: Func, sink: Si) -> Filter<Si, Func>
where
    Func: FnMut(&Item) -> bool,
    Si: Sink<Item>,
{
    Filter::new(func, sink)
}

/// Creates a [`FilterMap`] sink that filters and maps items in one step.
pub fn filter_map<Func, In, Out, Si>(func: Func, sink: Si) -> FilterMap<Si, Func>
where
    Func: FnMut(In) -> Option<Out>,
    Si: Sink<Out>,
{
    FilterMap::new(func, sink)
}

/// Creates a [`FlatMap`] sink that maps each item to an iterator and flattens the results.
pub fn flat_map<Func, In, IntoIter, Si>(func: Func, sink: Si) -> FlatMap<Si, Func, IntoIter>
where
    Func: FnMut(In) -> IntoIter,
    IntoIter: IntoIterator,
    Si: Sink<IntoIter::Item>,
{
    FlatMap::new(func, sink)
}

/// Creates a [`Flatten`] sink that flattens items that are iterators.
///
/// Note: Due to type inference limitations, you may need to specify the item type:
/// `flatten::<Vec<i32>, _>(sink)` where `Vec<i32>` is the input item type.
pub fn flatten<IntoIter, Si>(sink: Si) -> Flatten<Si, IntoIter>
where
    IntoIter: IntoIterator,
    Si: Sink<IntoIter::Item>,
{
    Flatten::new(sink)
}

/// Creates an [`Inspect`] sink that inspects each item without modifying it.
pub fn inspect<Func, Item, Si>(func: Func, sink: Si) -> Inspect<Si, Func>
where
    Func: FnMut(&Item),
    Si: Sink<Item>,
{
    Inspect::new(func, sink)
}

/// Creates an [`Unzip`] sink that splits tuple items into two separate sinks.
pub fn unzip<Si0, Si1, Item0, Item1>(sink0: Si0, sink1: Si1) -> Unzip<Si0, Si1>
where
    Si0: Sink<Item0>,
    Si1: Sink<Item1>,
    Si0::Error: From<Si1::Error>,
{
    Unzip::new(sink0, sink1)
}

/// Creates a [`ForEach`] sink that consumes each item with a function.
pub fn for_each<Func, Item>(func: Func) -> ForEach<Func>
where
    Func: FnMut(Item),
{
    ForEach::new(func)
}

/// Creates a [`TryForEach`] sink that consumes each item with a fallible function.
pub fn try_for_each<Func, Item, Error>(func: Func) -> TryForEach<Func>
where
    Func: FnMut(Item) -> Result<(), Error>,
{
    TryForEach::new(func)
}

/// Creates a [`SendIter`] future that sends all items from an iterator to a sink.
pub fn send_iter<I, Si>(iter: I, sink: Si) -> SendIter<I::IntoIter, Si>
where
    I: IntoIterator,
    Si: Sink<I::Item>,
{
    SendIter::new(iter.into_iter(), sink)
}

/// Creates a [`SendStream`] future that sends all items from a stream to a sink.
pub fn send_stream<St, Si>(stream: St, sink: Si) -> SendStream<St, Si>
where
    St: Stream,
    Si: Sink<St::Item>,
{
    SendStream::new(stream, sink)
}
