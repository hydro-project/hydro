use core::pin::Pin;
use core::task::Poll;

use pin_project_lite::pin_project;

use crate::Context;
use crate::pull::{Pull, PullStep};

pin_project! {
    /// Future that runs a closure on each item from a pull.
    #[must_use = "futures do nothing unless polled"]
    #[derive(Clone, Debug)]
    pub struct ForEach<Prev, Func> {
        #[pin]
        prev: Prev,
        func: Func,
    }
}

impl<Prev, Func> ForEach<Prev, Func>
where
    Self: Future,
{
    pub(crate) const fn new(prev: Prev, func: Func) -> Self {
        Self { prev, func }
    }
}

impl<Prev, Func, Item> Future for ForEach<Prev, Func>
where
    Prev: Pull<Item = Item>,
    Func: FnMut(Item),
    for<'ctx> Prev::Ctx<'ctx>: Context<'ctx>,
{
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut core::task::Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();
        let ctx = <Prev::Ctx<'_> as Context<'_>>::from_task(cx);
        loop {
            return match this.prev.as_mut().pull(ctx) {
                PullStep::Ready(item, _meta) => {
                    let () = (this.func)(item);
                    continue;
                }
                PullStep::Pending(_) => Poll::Pending,
                PullStep::Ended(_) => Poll::Ready(()),
            };
        }
    }
}
