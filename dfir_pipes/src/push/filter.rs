//! [`Filter`] push combinator.
use core::pin::Pin;

use pin_project_lite::pin_project;

use crate::push::{Push, PushStep};

pin_project! {
    /// Push combinator that only pushes items matching a predicate.
    #[must_use = "`Push`es do nothing unless items are pushed into them"]
    #[derive(Clone, Debug)]
    pub struct Filter<Next, Func> {
        #[pin]
        next: Next,
        func: Func,
    }
}

impl<Next, Func> Filter<Next, Func> {
    /// Creates with filtering `func` and next `push`.
    pub(crate) const fn new<Item>(func: Func, next: Next) -> Self
    where
        Func: FnMut(&Item) -> bool,
    {
        Self { next, func }
    }
}

impl<Next, Func, Item, Meta> Push<Item, Meta> for Filter<Next, Func>
where
    Next: Push<Item, Meta>,
    Func: FnMut(&Item) -> bool,
    Meta: Copy,
{
    type Ctx<'ctx> = Next::Ctx<'ctx>;

    type CanPend = Next::CanPend;

    fn poll_ready(self: Pin<&mut Self>, ctx: &mut Self::Ctx<'_>) -> PushStep<Self::CanPend> {
        self.project().next.poll_ready(ctx)
    }

    fn start_send(self: Pin<&mut Self>, item: Item, meta: Meta) {
        let this = self.project();
        if (this.func)(&item) {
            this.next.start_send(item, meta)
        }
    }

    fn poll_flush(self: Pin<&mut Self>, ctx: &mut Self::Ctx<'_>) -> PushStep<Self::CanPend> {
        self.project().next.poll_flush(ctx)
    }

    fn size_hint(self: Pin<&mut Self>, hint: (usize, Option<usize>)) {
        self.project().next.size_hint((0, hint.1));
    }
}
