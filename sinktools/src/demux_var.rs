//! Variadics for [`Sink`].

use core::marker::PhantomData;
use core::pin::Pin;
use core::task::{Context, Poll};

use pin_project_lite::pin_project;
use sealed::sealed;
use variadics::Variadic;

use crate::{Sink, forward_sink, ready_both};

/// A variadic of [`Sink`]s.
///
/// Used by [`DemuxVar`].
#[sealed]
pub trait SinkVariadic<Item, Error>: Variadic {
    /// [`Sink::poll_ready`] for all sinks.
    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>>;

    /// [`Sink::start_send`] for the sink at index `idx`.
    fn start_send(self: Pin<&mut Self>, idx: usize, item: Item) -> Result<(), Error>;

    /// [`Sink::poll_flush`] for all sinks.
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>>;

    /// [`Sink::poll_close`] for all sinks.
    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>>;
}

#[sealed]
impl<Si, Item, Rest> SinkVariadic<Item, Si::Error> for (Si, Rest)
where
    Si: Sink<Item>,
    Rest: SinkVariadic<Item, Si::Error>,
{
    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Si::Error>> {
        let (sink, rest) = pin_project_pair(self);
        ready_both!(sink.poll_ready(cx)?, rest.poll_ready(cx)?);
        Poll::Ready(Ok(()))
    }

    fn start_send(self: Pin<&mut Self>, idx: usize, item: Item) -> Result<(), Si::Error> {
        let (sink, rest) = pin_project_pair(self);
        if idx == 0 {
            sink.start_send(item)
        } else {
            rest.start_send(idx - 1, item)
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Si::Error>> {
        let (sink, rest) = pin_project_pair(self);
        ready_both!(sink.poll_flush(cx)?, rest.poll_flush(cx)?);
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Si::Error>> {
        let (sink, rest) = pin_project_pair(self);
        ready_both!(sink.poll_close(cx)?, rest.poll_close(cx)?);
        Poll::Ready(Ok(()))
    }
}

#[sealed]
impl<Item, Error> SinkVariadic<Item, Error> for () {
    fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(self: Pin<&mut Self>, idx: usize, _item: Item) -> Result<(), Error> {
        panic!("index out of bounds (len + {idx})");
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Poll::Ready(Ok(()))
    }
}

fn pin_project_pair<A, B>(pair: Pin<&mut (A, B)>) -> (Pin<&mut A>, Pin<&mut B>) {
    // SAFETY: `pair` is pinned, so its owned fields are also pinned.
    unsafe {
        let (a, b) = pair.get_unchecked_mut();
        (Pin::new_unchecked(a), Pin::new_unchecked(b))
    }
}

pin_project! {
    /// Sink which receives items paired with indices, and pushes to the corresponding output sink in a variadic of sinks.
    #[must_use = "sinks do nothing unless polled"]
    pub struct DemuxVar<Sinks, Error> {
        #[pin]
        sink: Sinks,
        // Must constrain `Error` for impl on empty list.
        _marker: PhantomData<fn() -> Error>,
    }
}

impl<Sinks, Error> DemuxVar<Sinks, Error> {
    /// Create with the given next `sinks`.
    pub fn new<Item>(sinks: Sinks) -> Self
    where
        Self: Sink<Item>,
    {
        Self {
            sink: sinks,
            _marker: PhantomData,
        }
    }
}

impl<Sinks, Item, Error> Sink<(usize, Item)> for DemuxVar<Sinks, Error>
where
    Sinks: SinkVariadic<Item, Error>,
{
    type Error = Error;

    fn start_send(self: Pin<&mut Self>, (idx, item): (usize, Item)) -> Result<(), Self::Error> {
        self.project().sink.start_send(idx, item)
    }

    forward_sink!(poll_ready, poll_flush, poll_close);
}

/// Creates a `DemuxVar` variadic that sends each item to one of many outputs, depending on the index.
pub fn demux_var<Sinks, Item, Error>(sinks: Sinks) -> DemuxVar<Sinks, Error>
where
    Sinks: SinkVariadic<Item, Error>,
{
    DemuxVar::new(sinks)
}
