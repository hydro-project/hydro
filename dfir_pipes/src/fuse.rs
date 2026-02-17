use core::pin::Pin;

use pin_project_lite::pin_project;

use crate::{FusedPull, Pull, Step, Yes};

pin_project! {
    pub struct Fuse<Prev> {
        #[pin]
        prev: Prev,
        done: bool,
    }
}

impl<Prev> Fuse<Prev> {
    pub fn new(prev: Prev) -> Self {
        Self { prev, done: false }
    }
}

impl<Prev> Pull for Fuse<Prev>
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

        if *this.done {
            return Step::Ended(Yes);
        }

        match this.prev.pull(ctx) {
            Step::Ready(item, meta) => Step::Ready(item, meta),
            Step::Pending(can_pend) => Step::Pending(can_pend),
            Step::Ended(_) => {
                *this.done = true;
                Step::Ended(Yes)
            }
        }
    }

    fn size_hint(self: Pin<&Self>) -> (usize, Option<usize>) {
        let this = self.project_ref();
        if *this.done {
            (0, Some(0))
        } else {
            this.prev.size_hint()
        }
    }
}

impl<Prev> FusedPull for Fuse<Prev> where Prev: Pull {}
