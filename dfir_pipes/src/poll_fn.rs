//! [`PollFn`] - a `Pull` created from a closure.

use core::pin::Pin;

use crate::{Pull, Step, Toggle, Yes};

/// A `Pull` implementation created from a closure.
pub struct PollFn<F, Item, Meta, CanEnd> {
    func: F,
    _marker: core::marker::PhantomData<fn() -> (Item, Meta, CanEnd)>,
}

impl<F, Item, Meta, CanEnd> PollFn<F, Item, Meta, CanEnd> {
    /// Create a new `PollFn` from the given closure.
    pub fn new(func: F) -> Self {
        Self {
            func,
            _marker: core::marker::PhantomData,
        }
    }
}

impl<F, Item, Meta, CanEnd> Unpin for PollFn<F, Item, Meta, CanEnd> {}

impl<F, Item, Meta, CanEnd> Pull for PollFn<F, Item, Meta, CanEnd>
where
    F: FnMut(&mut core::task::Context<'_>) -> Step<Item, Meta, Yes, CanEnd>,
    Meta: Copy,
    CanEnd: Toggle,
{
    type Ctx<'ctx> = core::task::Context<'ctx>;

    type Item = Item;
    type Meta = Meta;
    type CanPend = Yes;
    type CanEnd = CanEnd;

    fn pull(
        self: Pin<&mut Self>,
        ctx: &mut Self::Ctx<'_>,
    ) -> Step<Self::Item, Self::Meta, Self::CanPend, Self::CanEnd> {
        // SAFETY: PollFn is Unpin, so we can get a mutable reference
        let this = self.get_mut();
        (this.func)(ctx)
    }
}
