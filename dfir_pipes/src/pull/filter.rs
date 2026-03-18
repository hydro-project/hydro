use core::pin::Pin;

use pin_project_lite::pin_project;

use crate::pull::{FusedPull, Pull, PullStep};

pin_project! {
    /// Pull combinator that yields only items matching a predicate.
    #[must_use = "`Pull`s do nothing unless polled"]
    #[derive(Clone, Debug)]
    pub struct Filter<Prev, Func> {
        #[pin]
        prev: Prev,
        func: Func,
    }
}

impl<Prev, Func> Filter<Prev, Func>
where
    Self: Pull,
{
    pub(crate) const fn new(prev: Prev, func: Func) -> Self {
        Self { prev, func }
    }
}

impl<Prev, Func> Pull for Filter<Prev, Func>
where
    Prev: Pull,
    Func: FnMut(&Prev::Item) -> bool,
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
        let mut this = self.project();
        loop {
            return match this.prev.as_mut().pull(ctx) {
                PullStep::Ready(item, meta) => {
                    if (this.func)(&item) {
                        PullStep::Ready(item, meta)
                    } else {
                        continue;
                    }
                }
                PullStep::Pending(can_pend) => PullStep::Pending(can_pend),
                PullStep::Ended(can_end) => PullStep::Ended(can_end),
            };
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (_, upper) = self.prev.size_hint();
        (0, upper)
    }
}

impl<Prev, Func> FusedPull for Filter<Prev, Func>
where
    Prev: FusedPull,
    Func: FnMut(&Prev::Item) -> bool,
{
}
