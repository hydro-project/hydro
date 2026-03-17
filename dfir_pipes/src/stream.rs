use core::pin::Pin;
use core::task::Context;

use futures_core::FusedStream;
use pin_project_lite::pin_project;

use crate::{FusedPull, Pull, Step, Yes};

pin_project! {
    /// A pull that wraps a [`futures::Stream`](futures_core::stream::Stream).
    #[must_use = "`Pull`s do nothing unless polled"]
    #[derive(Clone, Debug, Default)]
    pub struct Stream<St> {
        #[pin]
        stream: St,
    }
}

impl<St> Stream<St>
where
    Self: Pull,
{
    pub(crate) const fn new(stream: St) -> Self {
        Self { stream }
    }
}

impl<St> Pull for Stream<St>
where
    St: futures_core::stream::Stream,
{
    type Ctx<'ctx> = Context<'ctx>;

    type Item = St::Item;
    type Meta = ();
    type CanPend = Yes;
    type CanEnd = Yes;

    fn pull(
        self: Pin<&mut Self>,
        ctx: &mut Self::Ctx<'_>,
    ) -> Step<Self::Item, Self::Meta, Self::CanPend, Self::CanEnd> {
        let this = self.project();
        match futures_core::stream::Stream::poll_next(this.stream, ctx) {
            core::task::Poll::Ready(Some(item)) => Step::Ready(item, ()),
            core::task::Poll::Ready(None) => Step::Ended(Yes),
            core::task::Poll::Pending => Step::Pending(Yes),
        }
    }

    fn size_hint(self: Pin<&Self>) -> (usize, Option<usize>) {
        self.project_ref().stream.size_hint()
    }
}

impl<St> FusedPull for Stream<St> where St: FusedStream {}
