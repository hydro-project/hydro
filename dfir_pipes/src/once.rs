use pin_project_lite::pin_project;

use crate::{No, Pull, Step, Yes};

pin_project! {
    pub struct Once<Item> {
        item: Option<Item>,
    }
}

impl<Item> Once<Item> {
    pub fn new(item: Item) -> Self {
        Self { item: Some(item) }
    }
}

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
        let this = self.project();
        match this.item.take() {
            Some(item) => Step::Ready(item, ()),
            None => Step::Ended(Yes),
        }
    }

    fn size_hint(self: core::pin::Pin<&Self>) -> (usize, Option<usize>) {
        let n = if self.item.is_some() { 1 } else { 0 };
        (n, Some(n))
    }
}
