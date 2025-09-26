//! Extra [`Sink`] adaptors and functions.
#![cfg_attr(not(test), no_std)]

pub use futures_util::sink;
pub use futures_util::sink::Sink;
#[cfg(feature = "variadics")]
#[cfg_attr(docsrs, doc(cfg(feature = "variadics")))]
pub use variadics;

mod filter;
mod filter_map;
mod flat_map;
mod flatten;
mod for_each;
mod inspect;
mod map;
mod send_all_iter;
mod unzip;
pub use filter::Filter;
pub use filter_map::FilterMap;
pub use flat_map::FlatMap;
pub use flatten::Flatten;
pub use for_each::ForEach;
pub use inspect::Inspect;
pub use map::Map;
pub use send_all_iter::SendAllIter;
pub use unzip::Unzip;

#[cfg(feature = "variadics")]
#[cfg_attr(docsrs, doc(cfg(feature = "variadics")))]
mod demux_var;
#[cfg(feature = "variadics")]
#[cfg_attr(docsrs, doc(cfg(feature = "variadics")))]
pub use demux_var::{DemuxVar, SinkVariadic};

pub trait Sinktools<Item>: Sink<Item> {
    fn by_ref(&mut self) -> &mut Self {
        self
    }

    fn send_all_iter<I>(self, into_iter: I) -> SendAllIter<I::IntoIter, Self>
    where
        Self: Sized,
        I: IntoIterator<Item = Item>,
    {
        SendAllIter::new(into_iter.into_iter(), self)
    }

    fn un_fanout<SiOther>(self, other: SiOther) -> sink::Fanout<Self, SiOther>
    where
        Self: Sized,
        Item: Clone,
        SiOther: Sink<Item, Error = Self::Error>,
    {
        sink::SinkExt::fanout(self, other)
    }

    fn un_filter_map<In, Func>(self, func: Func) -> FilterMap<Self, Func>
    where
        Self: Sized,
        Func: FnMut(In) -> Option<Item>,
    {
        FilterMap::new(func, self)
    }

    fn un_filter<Func>(self, func: Func) -> Filter<Self, Func>
    where
        Self: Sized,
        Func: FnMut(&Item) -> bool,
    {
        Filter::new(func, self)
    }

    fn un_flat_map<In, Func, IntoIter>(
        self,
        func: Func,
    ) -> FlatMap<Self, Func, IntoIter::IntoIter, IntoIter::Item>
    where
        Self: Sized,
        Func: FnMut(In) -> IntoIter,
        IntoIter: IntoIterator,
    {
        FlatMap::new(func, self)
    }

    fn un_flatten<In>(self) -> Flatten<Self, In::IntoIter, In::Item>
    where
        Self: Sized,
        In: IntoIterator,
    {
        Flatten::new(self)
    }

    fn un_inspect<Func>(self, func: Func) -> Inspect<Self, Func>
    where
        Self: Sized,
        Func: FnMut(&Item),
    {
        Inspect::new(func, self)
    }

    fn un_map<In, Func>(self, func: Func) -> Map<Self, Func>
    where
        Self: Sized,
        Func: FnMut(In) -> Item,
    {
        Map::new(func, self)
    }

    fn un_unzip<SiOther, ItemOther>(self, other: SiOther) -> Unzip<Self, SiOther>
    where
        Self: Sized,
        SiOther: Sink<ItemOther>,
    {
        Unzip::new(self, other)
    }
}

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

macro_rules! ready_both {
    ($a:expr, $b:expr) => {
        if !matches!(
            ($a, $b),
            (::core::task::Poll::Ready(()), ::core::task::Poll::Ready(())),
        ) {
            return ::core::task::Poll::Pending;
        }
    };
}
use ready_both;
