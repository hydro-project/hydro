use core::pin::Pin;

use pin_project_lite::pin_project;

use crate::{FusedPull, Pull, Step};

pin_project! {
    /// Pull combinator that maps each item to an iterator and flattens the results.
    #[must_use = "`Pull`s do nothing unless polled"]
    #[derive(Clone, Debug)]
    pub struct FlatMap<Prev, Func, Iter, Meta> {
        #[pin]
        prev: Prev,
        func: Func,
        current: Option<(Iter, Meta)>,
    }
}

impl<Prev, Func, Iter, Meta> FlatMap<Prev, Func, Iter, Meta>
where
    Self: Pull,
{
    pub(crate) const fn new(prev: Prev, func: Func) -> Self {
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
            let (iter, meta) = if let Some(current) = this.current.as_mut() {
                current
            } else {
                match this.prev.as_mut().pull(ctx) {
                    Step::Ready(item, meta) => {
                        this.current.insert(((this.func)(item).into_iter(), meta))
                    }
                    Step::Pending(can_pend) => {
                        return Step::Pending(can_pend);
                    }
                    Step::Ended(can_end) => {
                        return Step::Ended(can_end);
                    }
                }
            };
            if let Some(item) = iter.next() {
                return Step::Ready(item, *meta);
            }
            *this.current = None;
        }
    }

    fn size_hint(self: Pin<&Self>) -> (usize, Option<usize>) {
        let this = self.project_ref();
        let current_len = this
            .current
            .as_ref()
            .map(|(iter, _)| iter.size_hint().0)
            .unwrap_or_default();
        // We can't know the upper bound since each mapped iterator could have any size
        (current_len, None)
    }
}

impl<Prev, Func, IntoIter> FusedPull for FlatMap<Prev, Func, IntoIter::IntoIter, Prev::Meta>
where
    Prev: FusedPull,
    Func: FnMut(Prev::Item) -> IntoIter,
    IntoIter: IntoIterator,
{
}
