//! [`PullFn`] - a `Pull` created from a closure.

use core::pin::Pin;

use crate::{Pull, Step, Toggle};

/// A `Pull` implementation created from a closure.
pub struct PullFn<F, Item, Meta, CanPend, CanEnd> {
    func: F,
    _marker: core::marker::PhantomData<fn() -> (Item, Meta, CanPend, CanEnd)>,
}

impl<F, Item, Meta, CanPend, CanEnd> PullFn<F, Item, Meta, CanPend, CanEnd> {
    /// Create a new `PullFn` from the given closure.
    pub fn new(func: F) -> Self {
        Self {
            func,
            _marker: core::marker::PhantomData,
        }
    }
}

impl<F, Item, Meta, CanPend, CanEnd> Unpin for PullFn<F, Item, Meta, CanPend, CanEnd> {}

impl<F, Item, Meta, CanPend, CanEnd> Pull for PullFn<F, Item, Meta, CanPend, CanEnd>
where
    F: FnMut() -> Step<Item, Meta, CanPend, CanEnd>,
    Meta: Copy,
    CanPend: Toggle,
    CanEnd: Toggle,
{
    type Ctx<'ctx> = ();

    type Item = Item;
    type Meta = Meta;
    type CanPend = CanPend;
    type CanEnd = CanEnd;

    fn pull(
        self: Pin<&mut Self>,
        _ctx: &mut Self::Ctx<'_>,
    ) -> Step<Self::Item, Self::Meta, Self::CanPend, Self::CanEnd> {
        // SAFETY: PullFn is Unpin, so we can get a mutable reference
        let this = self.get_mut();
        (this.func)()
    }
}
