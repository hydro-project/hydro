use core::pin::Pin;

use pin_project_lite::pin_project;

use crate::{Pull, Step, Yes};

pin_project! {
    pub struct Take<Prev> {
        #[pin]
        prev: Prev,
        remaining: usize,
    }
}

impl<Prev> Take<Prev> {
    pub(crate) fn new(prev: Prev, n: usize) -> Self {
        Self { prev, remaining: n }
    }
}

impl<Prev> Pull for Take<Prev>
where
    Prev: Pull,
{
    type Ctx<'ctx> = Prev::Ctx<'ctx>;

    type Item = Prev::Item;
    type Meta = Prev::Meta;
    type CanPend = Prev::CanPend;
    type CanEnd = Yes;

    fn pull(
        self: Pin<&mut Self>,
        ctx: &mut Self::Ctx<'_>,
    ) -> Step<Self::Item, Self::Meta, Self::CanPend, Self::CanEnd> {
        let this = self.project();

        if *this.remaining == 0 {
            return Step::Ended(Yes);
        }

        match this.prev.pull(ctx) {
            Step::Ready(item, meta) => {
                *this.remaining -= 1;
                Step::Ready(item, meta)
            }
            Step::Pending(can_pend) => Step::Pending(can_pend),
            Step::Ended(_) => Step::Ended(Yes),
        }
    }

    fn size_hint(self: Pin<&Self>) -> (usize, Option<usize>) {
        let this = self.project_ref();
        let (lower, upper) = this.prev.size_hint();
        let remaining = *this.remaining;
        (
            lower.min(remaining),
            upper.map(|u| u.min(remaining)).or(Some(remaining)),
        )
    }
}
