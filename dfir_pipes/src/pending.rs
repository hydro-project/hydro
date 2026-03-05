use core::marker::PhantomData;
use core::pin::Pin;

use crate::{FusedPull, No, Pull, Step, Yes, fuse_self};

/// A pull that is always pending and never yields items or ends.
#[must_use = "`Pull`s do nothing unless polled"]
#[derive(Clone, Debug)]
pub struct Pending<Item> {
    _phantom: PhantomData<Item>,
}

impl<Item> Default for Pending<Item> {
    fn default() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<Item> Unpin for Pending<Item> {}

impl<Item> Pull for Pending<Item> {
    type Ctx<'ctx> = ();

    type Item = Item;
    type Meta = ();
    type CanPend = Yes;
    type CanEnd = No;

    fn pull(
        self: Pin<&mut Self>,
        _ctx: &mut Self::Ctx<'_>,
    ) -> Step<Self::Item, Self::Meta, Self::CanPend, Self::CanEnd> {
        Step::Pending(Yes)
    }

    fn size_hint(self: Pin<&Self>) -> (usize, Option<usize>) {
        (0, Some(0))
    }

    fuse_self!();
}

impl<Item> FusedPull for Pending<Item> {}
