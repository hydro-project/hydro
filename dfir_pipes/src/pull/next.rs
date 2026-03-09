use core::pin::Pin;
use core::task::{Context, Poll};

use pin_project_lite::pin_project;

use crate::pull::Pull;

pin_project! {
    /// A future which resolves with the next item from a [`Pull`].
    ///
    /// This is the `Pull` equivalent of the `StreamExt::next()` future.
    /// It polls the underlying pull and returns the result as a future.
    #[must_use = "futures do nothing unless polled"]
    #[derive(Clone, Debug, Default)]
    pub struct Next<Prev> {
        #[pin]
        prev: Prev,
    }
}

impl<Prev> Next<Prev>
where
    Self: Future,
{
    pub(crate) const fn new(prev: Prev) -> Self {
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
