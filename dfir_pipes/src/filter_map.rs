use core::pin::Pin;

use pin_project_lite::pin_project;

use crate::{Pull, Step};

pin_project! {
    pub struct FilterMap<Prev, Func> {
        #[pin]
        prev: Prev,
        func: Func,
    }
}

impl<Prev, Func> FilterMap<Prev, Func> {
    pub fn new(prev: Prev, func: Func) -> Self {
        Self { prev, func }
    }
}

impl<Prev, Func, Item> Pull for FilterMap<Prev, Func>
where
    Prev: Pull,
    Func: FnMut(Prev::Item) -> Option<Item>,
{
    type Ctx<'ctx> = Prev::Ctx<'ctx>;

    type Item = Item;
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
                    if let Some(mapped) = (this.func)(item) {
                        Step::Ready(mapped, meta)
                    } else {
                        continue;
                    }
                }
                Step::Pending(can_pend) => Step::Pending(can_pend),
                Step::Ended(can_end) => Step::Ended(can_end),
            };
        }
    }

    fn size_hint(self: Pin<&Self>) -> (usize, Option<usize>) {
        let (_, upper) = self.project_ref().prev.size_hint();
        (0, upper)
    }
}
