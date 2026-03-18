use core::pin::Pin;

use pin_project_lite::pin_project;

use crate::pull::{FusedPull, Pull, PullStep};

pin_project! {
    /// Pull combinator that skips items while a predicate returns `true`.
    #[must_use = "`Pull`s do nothing unless polled"]
    #[derive(Clone, Debug)]
    pub struct SkipWhile<Prev, Func> {
        #[pin]
        prev: Prev,
        func: Func,
        skipping: bool,
    }
}

impl<Prev, Func> SkipWhile<Prev, Func>
where
    Self: Pull,
{
    pub(crate) const fn new(prev: Prev, func: Func) -> Self {
        Self {
            prev,
            func,
            skipping: true,
        }
    }
}

impl<Prev, Func> Pull for SkipWhile<Prev, Func>
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
                    if *this.skipping && (this.func)(&item) {
                        continue;
                    }
                    *this.skipping = false;
                    PullStep::Ready(item, meta)
                }
                PullStep::Pending(can_pend) => PullStep::Pending(can_pend),
                PullStep::Ended(can_end) => PullStep::Ended(can_end),
            };
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (_, upper) = self.prev.size_hint();
        if self.skipping {
            // Still skipping, so lower bound is 0
            (0, upper)
        } else {
            // Done skipping, pass through
            self.prev.size_hint()
        }
    }
}

impl<Prev, Func> FusedPull for SkipWhile<Prev, Func>
where
    Prev: FusedPull,
    Func: FnMut(&Prev::Item) -> bool,
{
}
