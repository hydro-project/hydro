use core::pin::Pin;
use core::task::Poll;

use pin_project_lite::pin_project;

use crate::Context;
use crate::pull::{Pull, PullStep};

pin_project! {
    /// Future that collects all items from a pull into a collection.
    #[must_use = "futures do nothing unless polled"]
    #[derive(Clone, Debug, Default)]
    pub struct Collect<Prev, C> {
        #[pin]
        prev: Prev,
        collect: C,
    }
}

impl<Prev, C> Collect<Prev, C>
where
    Self: Future,
    C: Default,
{
    pub(crate) fn new(prev: Prev) -> Self {
        Self {
            prev,
            collect: C::default(),
        }
    }
}

impl<Prev, C> Future for Collect<Prev, C>
where
    Prev: Pull,
    for<'ctx> Prev::Ctx<'ctx>: Context<'ctx>,
    C: Default + Extend<Prev::Item>,
{
    type Output = C;

    fn poll(self: Pin<&mut Self>, cx: &mut core::task::Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();
        let ctx = <Prev::Ctx<'_> as Context<'_>>::from_task(cx);

        #[cfg(nightly)]
        this.collect.extend_reserve(this.prev.size_hint().0);

        loop {
            return match this.prev.as_mut().pull(ctx) {
                PullStep::Ready(item, _meta) => {
                    #[cfg(nightly)]
                    this.collect.extend_one(item);
                    #[cfg(not(nightly))]
                    this.collect.extend(core::iter::once(item));

                    continue;
                }
                PullStep::Pending(_) => Poll::Pending,
                PullStep::Ended(_) => Poll::Ready(core::mem::take(this.collect)),
            };
        }
    }
}
