use core::pin::Pin;

use pin_project_lite::pin_project;

use crate::{Pull, Step, Yes};

pin_project! {
    pub struct TakeWhile<Prev, Func> {
        #[pin]
        prev: Prev,
        func: Func,
        done: bool,
    }
}

impl<Prev, Func> TakeWhile<Prev, Func> {
    pub fn new(prev: Prev, func: Func) -> Self {
        Self {
            prev,
            func,
            done: false,
        }
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
    ) -> Step<Self::Item, Self::Meta, Self::CanPend, Self::CanEnd> {
        let this = self.project();

        if *this.done {
            return Step::Ended(Yes);
        }

        match this.prev.pull(ctx) {
            Step::Ready(item, meta) => {
                if (this.func)(&item) {
                    Step::Ready(item, meta)
                } else {
                    *this.done = true;
                    Step::Ended(Yes)
                }
            }
            Step::Pending(can_pend) => Step::Pending(can_pend),
            Step::Ended(_) => Step::Ended(Yes),
        }
    }

    fn size_hint(self: Pin<&Self>) -> (usize, Option<usize>) {
        let this = self.project_ref();
        if *this.done {
            (0, Some(0))
        } else {
            let (_, upper) = this.prev.size_hint();
            (0, upper)
        }
    }
}
