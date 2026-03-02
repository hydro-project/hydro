use std::pin::Pin;
use std::task::{Context, Poll};

use dfir_pipes::Pull;
use pin_project_lite::pin_project;

pin_project! {
    /// A future which resolves with the next item in the pull.
    pub struct IntoNextPull<Prev> {
        #[pin]
        pub(crate) prev: Prev,
    }
}

impl<Prev> IntoNextPull<Prev>
where
    Prev: Pull,
{
    /// Create a new IntoNextPull future.
    pub fn new(prev: Prev) -> Self {
        Self { prev }
    }
}

impl<Prev> Future for IntoNextPull<Prev>
where
    Prev: Pull,
{
    type Output = Option<(Prev::Item, Prev::Meta)>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let ctx = <Prev::Ctx<'_> as dfir_pipes::Context<'_>>::from_task(cx);
        this.prev.pull(ctx).into_poll()
    }
}
