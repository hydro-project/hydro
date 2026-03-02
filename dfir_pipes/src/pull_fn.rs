//! [`PullFn`] - a `Pull` created from a closure.

use core::pin::Pin;

use crate::{Pull, Step, Toggle, No};

/// A `Pull` implementation created from a closure.
pub struct PullFn<F, Item, Meta, CanEnd> {
    func: F,
    _marker: core::marker::PhantomData<fn() -> (Item, Meta, CanEnd)>,
}

impl<F, Item, Meta, CanEnd> PullFn<F, Item, Meta, CanEnd> {
    /// Create a new `PullFn` from the given closure.
    pub fn new(func: F) -> Self {
        Self {
            func,
            _marker: core::marker::PhantomData,
        }
    }
}

impl<F, Item, Meta, CanEnd> Unpin for PullFn<F, Item, Meta, CanEnd> {}

impl<F, Item, Meta, CanEnd> Pull for PullFn<F, Item, Meta, CanEnd>
where
    F: FnMut() -> Step<Item, Meta, No, CanEnd>,
    Meta: Copy,
    CanEnd: Toggle,
{
    type Ctx<'ctx> = ();

    type Item = Item;
    type Meta = Meta;
    type CanPend = No;
    type CanEnd = CanEnd;

    fn pull(
        self: Pin<&mut Self>,
        _ctx: &mut Self::Ctx<'_>,
    ) -> Step<Self::Item, Self::Meta, Self::CanPend, Self::CanEnd> {
        let this = self.get_mut();
        (this.func)()
    }
}
