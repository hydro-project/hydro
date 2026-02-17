//! [`SourceStream`] - a `Pull` that wraps a `Stream`.

use core::pin::Pin;
use core::task::{Poll, Waker};

use futures_core::stream::Stream;

use crate::{No, Pull, Step, Yes};

/// A `Pull` implementation that wraps a `Stream` and a `Waker`.
///
/// This is used by the `source_stream` operator to convert a `Stream` into a `Pull`.
pub struct SourceStream<S, W> {
    stream: S,
    waker: W,
}

impl<S, W> SourceStream<S, W> {
    /// Create a new `SourceStream` from the given stream and waker function.
    pub fn new(stream: S, waker: W) -> Self {
        Self { stream, waker }
    }
}

impl<S, W> Unpin for SourceStream<S, W> where S: Unpin {}

/// SourceStream uses its own waker, so it ignores the context parameter.
/// It implements `Pull` with `Ctx = ()`.
impl<S, W> Pull for SourceStream<S, W>
where
    S: Stream + Unpin,
    W: Fn() -> Waker,
{
    type Ctx<'ctx> = ();

    type Item = S::Item;
    type Meta = ();
    type CanPend = No;
    type CanEnd = Yes;

    fn pull(
        self: Pin<&mut Self>,
        _ctx: &mut Self::Ctx<'_>,
    ) -> Step<Self::Item, Self::Meta, Self::CanPend, Self::CanEnd> {
        let this = self.get_mut();
        let waker = (this.waker)();
        let mut cx = core::task::Context::from_waker(&waker);
        match Pin::new(&mut this.stream).poll_next(&mut cx) {
            Poll::Ready(Some(item)) => Step::Ready(item, ()),
            Poll::Ready(None) => Step::Ended(Yes),
            Poll::Pending => Step::Ended(Yes),
        }
    }

    fn size_hint(self: Pin<&Self>) -> (usize, Option<usize>) {
        self.stream.size_hint()
    }
}
