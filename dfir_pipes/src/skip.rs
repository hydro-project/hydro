use core::pin::Pin;

use pin_project_lite::pin_project;

use crate::{Pull, Step};

pin_project! {
    pub struct Skip<Prev> {
        #[pin]
        prev: Prev,
        remaining: usize,
    }
}

impl<Prev> Skip<Prev> {
    pub fn new(prev: Prev, n: usize) -> Self {
        Self { prev, remaining: n }
    }
}

impl<Prev> Pull for Skip<Prev>
where
    Prev: Pull,
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
                    if *this.remaining > 0 {
                        *this.remaining -= 1;
                        continue;
                    }
                    Step::Ready(item, meta)
                }
                Step::Pending(can_pend) => Step::Pending(can_pend),
                Step::Ended(can_end) => Step::Ended(can_end),
            };
        }
    }

    fn size_hint(self: Pin<&Self>) -> (usize, Option<usize>) {
        let this = self.project_ref();
        let (lower, upper) = this.prev.size_hint();
        let remaining = *this.remaining;
        (
            lower.saturating_sub(remaining),
            upper.map(|u| u.saturating_sub(remaining)),
        )
    }
}
