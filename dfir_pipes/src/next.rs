use core::pin::Pin;
use core::task::{Context, Poll};

use pin_project_lite::pin_project;

use crate::Pull;

pin_project! {
    /// A future which resolves with the next item from a [`Pull`].
    ///
    /// This is the `Pull` equivalent of [`futures::StreamExt::next()`].
    /// It polls the underlying pull once and returns the result as a future.
    #[must_use = "futures do nothing unless polled"]
    pub struct Next<Prev> {
        #[pin]
        prev: Prev,
    }
}

impl<Prev> Next<Prev>
where
    Prev: Pull,
{
    pub(crate) fn new(prev: Prev) -> Self {
        Self { prev }
    }
}

impl<Prev> Future for Next<Prev>
where
    Prev: Pull,
{
    type Output = Option<(Prev::Item, Prev::Meta)>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let ctx = <Prev::Ctx<'_> as crate::Context<'_>>::from_task(cx);
        this.prev.pull(ctx).into_poll()
    }
}
