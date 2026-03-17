use core::pin::Pin;

use pin_project_lite::pin_project;

use crate::{FusedPull, Pull, Step};

pin_project! {
    /// Pull combinator that flattens nested iterables.
    #[must_use = "`Pull`s do nothing unless polled"]
    #[derive(Clone, Debug, Default)]
    pub struct Flatten<Prev, Iter, Meta> {
        #[pin]
        prev: Prev,
        current: Option<(Iter, Meta)>,
    }
}

impl<Prev, Iter, Meta> Flatten<Prev, Iter, Meta>
where
    Self: Pull,
{
    pub(crate) const fn new(prev: Prev) -> Self {
        Self {
            prev,
            current: None,
        }
    }
}

impl<Prev> Pull for Flatten<Prev, <Prev::Item as IntoIterator>::IntoIter, Prev::Meta>
where
    Prev: Pull,
    Prev::Item: IntoIterator,
{
    type Ctx<'ctx> = Prev::Ctx<'ctx>;

    type Item = <Prev::Item as IntoIterator>::Item;
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
                    Step::Ready(iterable, meta) => {
                        this.current.insert((iterable.into_iter(), meta))
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
        // We can't know the upper bound since each inner iterator could have any size
        (current_len, None)
    }
}

impl<Prev> FusedPull for Flatten<Prev, <Prev::Item as IntoIterator>::IntoIter, Prev::Meta>
where
    Prev: FusedPull,
    Prev::Item: IntoIterator,
{
}
