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
    pub(crate) fn new(prev: Prev) -> Self {
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
            if let Some((iter, meta)) = this.current.as_mut() {
                if let Some(item) = iter.next() {
                    return Step::Ready(item, *meta);
                }
                *this.current = None;
            }

            return match this.prev.as_mut().pull(ctx) {
                Step::Ready(iterable, meta) => {
                    *this.current = Some((iterable.into_iter(), meta));
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
