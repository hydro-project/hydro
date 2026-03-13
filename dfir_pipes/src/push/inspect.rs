//! [`Inspect`] push combinator.
use core::pin::Pin;

use pin_project_lite::pin_project;

use crate::push::{Push, PushStep};

pin_project! {
    /// Push combinator that calls a closure on each item for side effects before pushing downstream.
    #[must_use = "`Push`es do nothing unless items are pushed into them"]
    #[derive(Clone, Debug)]
    pub struct Inspect<Next, Func> {
        #[pin]
        next: Next,
        func: Func,
    }
}

impl<Next, Func> Inspect<Next, Func> {
    /// Creates with inspecting `func` and next `push`.
    pub(crate) const fn new<Item>(func: Func, next: Next) -> Self
    where
        Func: FnMut(&Item),
    {
        Self { next, func }
    }
}

impl<Next, Func, Item, Meta> Push<Item, Meta> for Inspect<Next, Func>
where
    Next: Push<Item, Meta>,
    Func: FnMut(&Item),
    Meta: Copy,
{
    type Ctx<'ctx> = Next::Ctx<'ctx>;

    type CanPend = Next::CanPend;

    fn poll_ready(self: Pin<&mut Self>, ctx: &mut Self::Ctx<'_>) -> PushStep<Self::CanPend> {
        self.project().next.poll_ready(ctx)
    }

    fn start_send(self: Pin<&mut Self>, item: Item, meta: Meta) {
        let this = self.project();
        (this.func)(&item);
        this.next.start_send(item, meta)
    }

    fn poll_flush(self: Pin<&mut Self>, ctx: &mut Self::Ctx<'_>) -> PushStep<Self::CanPend> {
        self.project().next.poll_flush(ctx)
    }
}
