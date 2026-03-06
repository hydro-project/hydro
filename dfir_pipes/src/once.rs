use crate::{FusedPull, No, Pull, Step, Yes, fuse_self};

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
    ) -> Step<Self::Item, Self::Meta, Self::CanPend, Self::CanEnd> {
        match self.get_mut().item.take() {
            Some(item) => Step::Ready(item, ()),
            None => Step::Ended(Yes),
        }
    }

    fn size_hint(self: core::pin::Pin<&Self>) -> (usize, Option<usize>) {
        let n = if self.item.is_some() { 1 } else { 0 };
        (n, Some(n))
    }

    fuse_self!();
}

impl<Item> FusedPull for Once<Item> {}
