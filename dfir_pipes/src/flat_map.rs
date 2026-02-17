use core::pin::Pin;

use pin_project_lite::pin_project;

use crate::{Pull, Step};

pin_project! {
    pub struct FlatMap<Prev, Func, Iter, Meta> {
        #[pin]
        prev: Prev,
        func: Func,
        current: Option<(Iter, Meta)>,
    }
}

impl<Prev, Func, Iter, Meta> FlatMap<Prev, Func, Iter, Meta> {
    pub fn new(prev: Prev, func: Func) -> Self {
        Self {
            prev,
            func,
            current: None,
        }
    }
}

impl<Prev, Func, IntoIter> Pull for FlatMap<Prev, Func, IntoIter::IntoIter, Prev::Meta>
where
    Prev: Pull,
    Func: FnMut(Prev::Item) -> IntoIter,
    IntoIter: IntoIterator,
{
    type Ctx<'ctx> = Prev::Ctx<'ctx>;

    type Item = IntoIter::Item;
    type Meta = Prev::Meta;
    type CanPend = Prev::CanPend;
    type CanEnd = Prev::CanEnd;

    fn pull(
        self: Pin<&mut Self>,
        ctx: &mut Self::Ctx<'_>,
    ) -> Step<Self::Item, Self::Meta, Self::CanPend, Self::CanEnd> {
        let mut this = self.project();
        loop {
            if let Some((iter, meta)) = this.current.as_mut() {
                if let Some(item) = iter.next() {
                    return Step::Ready(item, *meta);
                }
                *this.current = None;
            }

            return match this.prev.as_mut().pull(ctx) {
                Step::Ready(item, meta) => {
                    *this.current = Some(((this.func)(item).into_iter(), meta));
                    continue;
                }
                Step::Pending(can_pend) => Step::Pending(can_pend),
                Step::Ended(can_end) => Step::Ended(can_end),
            };
        }
    }

    fn size_hint(self: Pin<&Self>) -> (usize, Option<usize>) {
        let this = self.project_ref();
        let current_len = this
            .current
            .as_ref()
            .map(|(iter, _)| iter.size_hint().0)
            .unwrap_or(0);
        // We can't know the upper bound since each mapped iterator could have any size
        (current_len, None)
    }
}
