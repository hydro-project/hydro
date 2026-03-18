//! [`FilterMap`] push combinator.
use core::pin::Pin;

use pin_project_lite::pin_project;

use crate::push::{Push, PushStep};

pin_project! {
    /// Push combinator that both filters and maps items.
    #[must_use = "`Push`es do nothing unless items are pushed into them"]
    #[derive(Clone, Debug)]
    pub struct FilterMap<Next, Func> {
        #[pin]
        next: Next,
        func: Func,
    }
}

impl<Next, Func> FilterMap<Next, Func> {
    /// Creates with filter-mapping `func` and next `push`.
    pub(crate) const fn new<In, Out>(func: Func, next: Next) -> Self
    where
        Func: FnMut(In) -> Option<Out>,
    {
        Self { next, func }
    }
}

impl<Next, Func, In, Out, Meta> Push<In, Meta> for FilterMap<Next, Func>
where
    Next: Push<Out, Meta>,
    Func: FnMut(In) -> Option<Out>,
    Meta: Copy,
{
    type Ctx<'ctx> = Next::Ctx<'ctx>;

    type CanPend = Next::CanPend;

    fn poll_ready(self: Pin<&mut Self>, ctx: &mut Self::Ctx<'_>) -> PushStep<Self::CanPend> {
        self.project().next.poll_ready(ctx)
    }

    fn start_send(self: Pin<&mut Self>, item: In, meta: Meta) {
        let this = self.project();
        if let Some(item) = (this.func)(item) {
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
