use core::pin::Pin;
use core::task::Context;

use pin_project_lite::pin_project;

use crate::{Pull, Step, Yes};

pin_project! {
    pub struct Stream<St> {
        #[pin]
        stream: St,
    }
}

impl<St> Stream<St> {
    pub(crate) fn new(stream: St) -> Self {
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
