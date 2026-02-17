use core::pin::Pin;

use pin_project_lite::pin_project;

use crate::{FusedPull, Pull, Step};

pin_project! {
    /// Pull combinator that pairs each item with its index.
    #[must_use = "`Pull`s do nothing unless polled"]
    #[derive(Clone, Debug, Default)]
    pub struct Enumerate<Prev> {
        #[pin]
        prev: Prev,
        index: usize,
    }
}

impl<Prev> Enumerate<Prev>
where
    Self: Pull,
{
    pub(crate) const fn new(prev: Prev) -> Self {
        Self { prev, index: 0 }
    }
}

impl<Prev> Pull for Enumerate<Prev>
where
    Prev: Pull,
{
    type Ctx<'ctx> = Prev::Ctx<'ctx>;

    type Item = (usize, Prev::Item);
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
                let idx = *this.index;
                *this.index += 1;
                Step::Ready((idx, item), meta)
            }
            Step::Pending(can_pend) => Step::Pending(can_pend),
            Step::Ended(can_end) => Step::Ended(can_end),
        }
    }

    fn size_hint(self: Pin<&Self>) -> (usize, Option<usize>) {
        self.project_ref().prev.size_hint()
    }
}

impl<Prev> FusedPull for Enumerate<Prev> where Prev: FusedPull {}
