use crate::pull::{FusedPull, Pull, PullStep, fuse_self};
use crate::{No, Yes};

/// A pull that yields a single item.
#[must_use = "`Pull`s do nothing unless polled"]
#[derive(Clone, Debug, Default)]
pub struct Once<Item> {
    item: Option<Item>,
}

impl<Item> Once<Item>
where
    Self: Pull,
{
    pub(crate) const fn new(item: Item) -> Self {
        Self { item: Some(item) }
    }
}

impl<Item> Unpin for Once<Item> {}

impl<Item> Pull for Once<Item> {
    type Ctx<'ctx> = ();

    type Item = Item;
    type Meta = ();
    type CanPend = No;
    type CanEnd = Yes;

    fn pull(
        self: core::pin::Pin<&mut Self>,
        _ctx: &mut Self::Ctx<'_>,
    ) -> PullStep<Self::Item, Self::Meta, Self::CanPend, Self::CanEnd> {
        match self.get_mut().item.take() {
            Some(item) => PullStep::Ready(item, ()),
            None => PullStep::Ended(Yes),
        }
    }

    fn size_hint(self: core::pin::Pin<&Self>) -> (usize, Option<usize>) {
        let n = if self.item.is_some() { 1 } else { 0 };
        (n, Some(n))
    }

    fuse_self!();
}

impl<Item> FusedPull for Once<Item> {}
