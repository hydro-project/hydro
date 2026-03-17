//! [`FromFn`] - a `Pull` created from a closure.

use core::pin::Pin;

use crate::{No, Pull, Step, Toggle};

/// A `Pull` implementation created from a closure.
#[must_use = "`Pull`s do nothing unless polled"]
#[derive(Clone, Debug)]
pub struct FromFn<F, Item, Meta, CanEnd> {
    func: F,
    #[expect(clippy::type_complexity, reason = "phantom data")]
    _marker: core::marker::PhantomData<fn() -> (Item, Meta, CanEnd)>,
}

impl<F, Item, Meta, CanEnd> FromFn<F, Item, Meta, CanEnd>
where
    Self: Pull,
{
    /// Create a new `FromFn` from the given closure.
    pub(crate) fn new(func: F) -> Self {
        Self {
            func,
            _marker: core::marker::PhantomData,
        }
    }
}

impl<F, Item, Meta, CanEnd> Unpin for FromFn<F, Item, Meta, CanEnd> {}

impl<F, Item, Meta, CanEnd> Pull for FromFn<F, Item, Meta, CanEnd>
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
