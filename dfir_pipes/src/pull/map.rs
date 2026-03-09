use core::pin::Pin;

use pin_project_lite::pin_project;

use crate::pull::{FusedPull, Pull, PullStep};

pin_project! {
    /// Pull combinator that transforms each item with a closure.
    #[must_use = "`Pull`s do nothing unless polled"]
    #[derive(Clone, Debug)]
    pub struct Map<Prev, Func> {
        #[pin]
        prev: Prev,
        func: Func,
    }
}

impl<Prev, Func> Map<Prev, Func>
where
    Self: Pull,
{
    pub(crate) const fn new(prev: Prev, func: Func) -> Self {
        Self { prev, func }
    }
}

impl<Prev, Func, Item> Pull for Map<Prev, Func>
where
    Prev: Pull,
    Func: FnMut(Prev::Item) -> Item,
{
    type Ctx<'ctx> = Prev::Ctx<'ctx>;

    type Item = Item;
    type Meta = Prev::Meta;
    type CanPend = Prev::CanPend;
    type CanEnd = Prev::CanEnd;

    fn pull(
        self: Pin<&mut Self>,
        ctx: &mut Self::Ctx<'_>,
    ) -> PullStep<Self::Item, Self::Meta, Self::CanPend, Self::CanEnd> {
        let this = self.project();
        match this.prev.pull(ctx) {
            PullStep::Ready(item, meta) => PullStep::Ready((this.func)(item), meta),
            PullStep::Pending(can_pend) => PullStep::Pending(can_pend),
            PullStep::Ended(can_finish) => PullStep::Ended(can_finish),
        }
    }

    fn size_hint(self: Pin<&Self>) -> (usize, Option<usize>) {
        self.project_ref().prev.size_hint()
    }
}

impl<Prev, Func, Item> FusedPull for Map<Prev, Func>
where
    Prev: FusedPull,
    Func: FnMut(Prev::Item) -> Item,
{
}
