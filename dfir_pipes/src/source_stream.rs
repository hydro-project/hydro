//! [`SourceStream`] - a `Pull` that wraps a `Stream`.

use core::pin::Pin;
use core::task::{Poll, Waker};

use futures_core::stream::Stream;
use pin_project_lite::pin_project;

use crate::{No, Pull, Step, Yes};

pin_project! {
    /// A `Pull` implementation that wraps a `Stream` and a `Waker`.
    ///
    /// This is used by the `source_stream` operator to convert a `Stream` into a `Pull`.
    pub struct SourceStream<S> {
        #[pin]
        stream: S,
        waker: Waker,
    }
}

impl<S> SourceStream<S> {
    /// Create a new `SourceStream` from the given stream and waker function.
    pub fn new(stream: S, waker: Waker) -> Self {
        Self { stream, waker }
    }
}

/// SourceStream uses its own waker, so it ignores the context parameter.
/// It implements `Pull` with `Ctx = ()`.
impl<S> Pull for SourceStream<S>
where
    S: Stream,
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
        let this = self.project();
        let mut cx = core::task::Context::from_waker(&this.waker);
        match this.stream.poll_next(&mut cx) {
            Poll::Ready(Some(item)) => Step::Ready(item, ()),
            Poll::Ready(None) => Step::Ended(Yes),
            Poll::Pending => Step::Ended(Yes),
        }
    }

    fn size_hint(self: Pin<&Self>) -> (usize, Option<usize>) {
        self.stream.size_hint()
    }
}
