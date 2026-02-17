use core::pin::Pin;

use pin_project_lite::pin_project;

use crate::{Pull, Step};

pin_project! {
    pub struct SkipWhile<Prev, Func> {
        #[pin]
        prev: Prev,
        func: Func,
        skipping: bool,
    }
}

impl<Prev, Func> SkipWhile<Prev, Func> {
    pub fn new(prev: Prev, func: Func) -> Self {
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
    ) -> Step<Self::Item, Self::Meta, Self::CanPend, Self::CanEnd> {
        let mut this = self.project();

        loop {
            return match this.prev.as_mut().pull(ctx) {
                Step::Ready(item, meta) => {
                    if *this.skipping && (this.func)(&item) {
                        continue;
                    }
                    *this.skipping = false;
                    Step::Ready(item, meta)
                }
                Step::Pending(can_pend) => Step::Pending(can_pend),
                Step::Ended(can_end) => Step::Ended(can_end),
            };
        }
    }

    fn size_hint(self: Pin<&Self>) -> (usize, Option<usize>) {
        let this = self.project_ref();
        let (_, upper) = this.prev.size_hint();
        if *this.skipping {
            // Still skipping, so lower bound is 0
            (0, upper)
        } else {
            // Done skipping, pass through
            this.prev.size_hint()
        }
    }
}
