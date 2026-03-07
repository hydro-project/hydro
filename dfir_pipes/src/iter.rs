use core::iter::FusedIterator;
use core::pin::Pin;

use pin_project_lite::pin_project;

use crate::{FusedPull, No, Pull, Step, Yes};

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
    ) -> Step<Self::Item, Self::Meta, Self::CanPend, Self::CanEnd> {
        let this = self.project();
        match this.iter.next() {
            Some(item) => Step::Ready(item, ()),
            None => Step::Ended(Yes),
        }
    }

    fn size_hint(self: Pin<&Self>) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<I> FusedPull for Iter<I> where I: FusedIterator {}
