use core::marker::PhantomData;

use crate::{FusedPull, No, Pull, Step, Yes, fuse_self};

/// A pull that yields no items and immediately ends.
#[must_use = "`Pull`s do nothing unless polled"]
#[derive(Clone, Debug)]
pub struct Empty<Item> {
    _phantom: PhantomData<Item>,
}

impl<Item> Default for Empty<Item> {
    fn default() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<Item> Pull for Empty<Item> {
    type Ctx<'ctx> = ();

    type Item = Item;
    type Meta = ();
    type CanPend = No;
    type CanEnd = Yes;

    fn pull(
        self: core::pin::Pin<&mut Self>,
        _ctx: &mut Self::Ctx<'_>,
    ) -> Step<Self::Item, Self::Meta, Self::CanPend, Self::CanEnd> {
        Step::Ended(Yes)
    }

    fn size_hint(self: core::pin::Pin<&Self>) -> (usize, Option<usize>) {
        (0, Some(0))
    }

    fuse_self!();
}

impl<Item> FusedPull for Empty<Item> {}
