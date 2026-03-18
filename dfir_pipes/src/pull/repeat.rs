use crate::No;
use crate::pull::{FusedPull, Pull, PullStep, fuse_self};

/// A pull that yields clones of an item forever.
#[must_use = "`Pull`s do nothing unless polled"]
#[derive(Clone, Debug, Default)]
pub struct Repeat<Item> {
    item: Item,
}

impl<Item> Repeat<Item>
where
    Self: Pull,
{
    pub(crate) const fn new(item: Item) -> Self {
        Self { item }
    }
}

impl<Item> Unpin for Repeat<Item> {}

impl<Item> Pull for Repeat<Item>
where
    Item: Clone,
{
    type Ctx<'ctx> = ();

    type Item = Item;
    type Meta = ();
    type CanPend = No;
    type CanEnd = No;

    fn pull(
        self: core::pin::Pin<&mut Self>,
        _ctx: &mut Self::Ctx<'_>,
    ) -> PullStep<Self::Item, Self::Meta, Self::CanPend, Self::CanEnd> {
        PullStep::Ready(self.item.clone(), ())
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (usize::MAX, None)
    }

    fuse_self!();
}

impl<Item> FusedPull for Repeat<Item> where Item: Clone {}
