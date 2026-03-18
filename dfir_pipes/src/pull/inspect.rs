use core::pin::Pin;

use pin_project_lite::pin_project;

use crate::pull::{FusedPull, Pull, PullStep};

pin_project! {
    /// Pull combinator that calls a closure on each item for side effects, passing the item through.
    #[must_use = "`Pull`s do nothing unless polled"]
    #[derive(Clone, Debug)]
    pub struct Inspect<Prev, Func> {
        #[pin]
        prev: Prev,
        func: Func,
    }
}

impl<Prev, Func> Inspect<Prev, Func>
where
    Self: Pull,
{
    pub(crate) const fn new(prev: Prev, func: Func) -> Self {
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
    ) -> PullStep<Self::Item, Self::Meta, Self::CanPend, Self::CanEnd> {
        let this = self.project();
        match this.prev.pull(ctx) {
            PullStep::Ready(item, meta) => {
                (this.func)(&item);
                PullStep::Ready(item, meta)
            }
            PullStep::Pending(can_pend) => PullStep::Pending(can_pend),
            PullStep::Ended(can_end) => PullStep::Ended(can_end),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.prev.size_hint()
    }
}

impl<Prev, Func> FusedPull for Inspect<Prev, Func>
where
    Prev: FusedPull,
    Func: FnMut(&Prev::Item),
{
}
