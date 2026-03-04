use core::pin::Pin;

use pin_project_lite::pin_project;

use crate::{Pull, Step};

pin_project! {
    pub struct Inspect<Prev, Func> {
        #[pin]
        prev: Prev,
        func: Func,
    }
}

impl<Prev, Func> Inspect<Prev, Func> {
    pub(crate) fn new(prev: Prev, func: Func) -> Self {
        Self { prev, func }
    }
}

impl<Prev, Func> Pull for Inspect<Prev, Func>
where
    Prev: Pull,
    Func: FnMut(&Prev::Item),
{
    type Ctx<'ctx> = Prev::Ctx<'ctx>;

    type Item = Prev::Item;
    type Meta = Prev::Meta;
    type CanPend = Prev::CanPend;
    type CanEnd = Prev::CanEnd;

    fn pull(
        self: Pin<&mut Self>,
        ctx: &mut Self::Ctx<'_>,
    ) -> Step<Self::Item, Self::Meta, Self::CanPend, Self::CanEnd> {
        let this = self.project();
        match this.prev.pull(ctx) {
            Step::Ready(item, meta) => {
                (this.func)(&item);
                Step::Ready(item, meta)
            }
            Step::Pending(can_pend) => Step::Pending(can_pend),
            Step::Ended(can_end) => Step::Ended(can_end),
        }
    }

    fn size_hint(self: Pin<&Self>) -> (usize, Option<usize>) {
        self.project_ref().prev.size_hint()
    }
}
