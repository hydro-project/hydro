use core::iter::FusedIterator;
use core::pin::Pin;

use pin_project_lite::pin_project;

use crate::pull::{FusedPull, Pull, PullStep};
use crate::{No, Yes};

pin_project! {
    /// A pull that wraps an iterator.
    #[must_use = "`Pull`s do nothing unless polled"]
    #[derive(Clone, Debug, Default)]
    pub struct Iter<I> {
        iter: I,
    }
}

impl<I> Iter<I> {
    pub(crate) const fn new(iter: I) -> Self {
        Self { iter }
    }
}

impl<I> Pull for Iter<I>
where
    I: Iterator,
{
    type Ctx<'ctx> = ();

    type Item = I::Item;
    type Meta = ();
    type CanPend = No;
    type CanEnd = Yes;

    fn pull(
        self: Pin<&mut Self>,
        _ctx: &mut Self::Ctx<'_>,
    ) -> PullStep<Self::Item, Self::Meta, Self::CanPend, Self::CanEnd> {
        let this = self.project();
        match this.iter.next() {
            Some(item) => PullStep::Ready(item, ()),
            None => PullStep::Ended(Yes),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<I> FusedPull for Iter<I> where I: FusedIterator {}
