//! Variadics for [`Sink`].

use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll, ready};

use futures::sink::Sink;
use pin_project_lite::pin_project;
use sealed::sealed;
use variadics::Variadic;

/// A variadic of [`Sink`]s.
#[sealed]
pub trait SinkVariadic<Item, Error>: Variadic {
    /// [`Sink::poll_ready`] for the sink at index `idx`.
    fn poll_ready(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        idx: usize,
    ) -> Poll<Result<(), Error>>;

    /// [`Sink::start_send`] for the sink at index `idx`.
    fn start_send(self: Pin<&mut Self>, item: Item, idx: usize) -> Result<(), Error>;

    /// [`Sink::poll_flush`] for all elements.
    fn poll_flush_all(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>>;

    /// [`Sink::poll_close`] for all elements.
    fn poll_close_all(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>>;
}

#[sealed]
impl<Si, Item, Rest> SinkVariadic<Item, Si::Error> for (Si, Rest)
where
    Si: Sink<Item>,
    Rest: SinkVariadic<Item, Si::Error>,
{
    fn poll_ready(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        idx: usize,
    ) -> Poll<Result<(), Si::Error>> {
        let (sink, rest) = pin_project_pair(self);
        if idx == 0 {
            sink.poll_ready(cx)
        } else {
            rest.poll_ready(cx, idx - 1)
        }
    }

    fn start_send(self: Pin<&mut Self>, item: Item, idx: usize) -> Result<(), Si::Error> {
        let (sink, rest) = pin_project_pair(self);
        if idx == 0 {
            sink.start_send(item)
        } else {
            rest.start_send(item, idx - 1)
        }
    }

    fn poll_flush_all(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Si::Error>> {
        let (sink, rest) = pin_project_pair(self);
        // Flush all sinks simultaneously.
        let ready_sink = sink.poll_flush(cx)?;
        let ready_rest = rest.poll_flush_all(cx)?;
        ready!(ready_sink);
        ready!(ready_rest);
        Poll::Ready(Ok(()))
    }

    fn poll_close_all(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Si::Error>> {
        let (sink, rest) = pin_project_pair(self);
        // Close all sinks simultaneously.
        let ready_sink = sink.poll_close(cx)?;
        let ready_rest = rest.poll_close_all(cx)?;
        ready!(ready_sink);
        ready!(ready_rest);
        Poll::Ready(Ok(()))
    }
}

#[sealed]
impl<Item, Error> SinkVariadic<Item, Error> for () {
    fn poll_ready(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _idx: usize,
    ) -> Poll<Result<(), Error>> {
        panic!("index out of bounds");
    }

    fn start_send(self: Pin<&mut Self>, _item: Item, _idx: usize) -> Result<(), Error> {
        panic!("index out of bounds");
    }

    fn poll_flush_all(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close_all(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
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
    pub struct Partition<Sinks, Item, Error> {
        #[pin]
        sinks: Sinks,
        next: Option<(Item, usize)>,
        _marker: PhantomData<fn() -> Error>,
    }
}

impl<Sinks, Item, Error> Partition<Sinks, Item, Error> {
    /// Create with the given `sinks`.
    pub fn new(sinks: Sinks) -> Self
    where
        Self: Sink<(Item, usize)>,
    {
        Self {
            sinks,
            next: None,
            _marker: PhantomData,
        }
    }

    fn poll_ready_impl(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>>
    where
        Sinks: SinkVariadic<Item, Error>,
    {
        let mut this = self.project();
        if let Some((item, idx)) = this.next.take() {
            ready!(this.sinks.as_mut().poll_ready(cx, idx))?;
            this.sinks.start_send(item, idx)?;
        }
        Poll::Ready(Ok(()))
    }
}

impl<Sinks, Item, Error> Sink<(Item, usize)> for Partition<Sinks, Item, Error>
where
    Sinks: SinkVariadic<Item, Error>,
{
    type Error = Error;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.poll_ready_impl(cx)
    }

    fn start_send(self: Pin<&mut Self>, item: (Item, usize)) -> Result<(), Self::Error> {
        let this = self.project();
        debug_assert!(
            this.next.is_none(),
            "Sink not ready: `poll_ready` must be called and return `Ready` before `start_send` is called."
        );
        *this.next = Some(item);
        Ok(())
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        ready!(self.as_mut().poll_ready_impl(cx)?);
        self.project().sinks.poll_flush_all(cx)
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        ready!(self.as_mut().poll_ready_impl(cx)?);
        self.project().sinks.poll_close_all(cx)
    }
}
