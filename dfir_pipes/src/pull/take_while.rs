use core::pin::Pin;

use pin_project_lite::pin_project;

use crate::Yes;
use crate::pull::{Pull, PullStep};

pin_project! {
    /// Pull combinator that yields items while a predicate returns `true`.
    #[must_use = "`Pull`s do nothing unless polled"]
    #[derive(Clone, Debug)]
    pub struct TakeWhile<Prev, Func> {
        #[pin]
        prev: Prev,
        func: Func,
    }
}

impl<Prev, Func> TakeWhile<Prev, Func>
where
    Self: Pull,
{
    pub(crate) const fn new(prev: Prev, func: Func) -> Self {
        Self { prev, func }
    }
}

impl<Prev, Func> Pull for TakeWhile<Prev, Func>
where
    Prev: Pull,
    Func: FnMut(&Prev::Item) -> bool,
{
    type Ctx<'ctx> = Prev::Ctx<'ctx>;

    type Item = Prev::Item;
    type Meta = Prev::Meta;
    type CanPend = Prev::CanPend;
    type CanEnd = Yes;

    fn pull(
        self: Pin<&mut Self>,
        ctx: &mut Self::Ctx<'_>,
    ) -> PullStep<Self::Item, Self::Meta, Self::CanPend, Self::CanEnd> {
        let this = self.project();

        match this.prev.pull(ctx) {
            PullStep::Ready(item, meta) => {
                if (this.func)(&item) {
                    PullStep::Ready(item, meta)
                } else {
                    PullStep::Ended(Yes)
                }
            }
            PullStep::Pending(can_pend) => PullStep::Pending(can_pend),
            PullStep::Ended(_) => PullStep::Ended(Yes),
        }
    }

    fn size_hint(self: Pin<&Self>) -> (usize, Option<usize>) {
        let (_, upper) = self.project_ref().prev.size_hint();
        (0, upper)
    }
}
