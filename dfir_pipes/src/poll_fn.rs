//! [`PollFn`] - a `Pull` created from a closure.

use core::pin::Pin;

use crate::{Pull, Step, Toggle};

/// A `Pull` implementation created from a closure.
#[must_use = "`Pull`s do nothing unless polled"]
#[derive(Clone, Debug)]
pub struct PollFn<F, Item, Meta, CanPend, CanEnd> {
    func: F,
    #[expect(clippy::type_complexity, reason = "phantom data")]
    _marker: core::marker::PhantomData<fn() -> (Item, Meta, CanPend, CanEnd)>,
}

impl<F, Item, Meta, CanPend, CanEnd> PollFn<F, Item, Meta, CanPend, CanEnd>
where
    Self: Pull,
{
    /// Create a new `PollFn` from the given closure.
    pub(crate) fn new(func: F) -> Self {
        Self {
            func,
            _marker: core::marker::PhantomData,
        }
    }
}

impl<F, Item, Meta, CanPend, CanEnd> Unpin for PollFn<F, Item, Meta, CanPend, CanEnd> {}

impl<F, Item, Meta, CanPend, CanEnd> Pull for PollFn<F, Item, Meta, CanPend, CanEnd>
where
    F: FnMut(&mut core::task::Context<'_>) -> Step<Item, Meta, CanPend, CanEnd>,
    Meta: Copy,
    CanPend: Toggle,
    CanEnd: Toggle,
{
    type Ctx<'ctx> = core::task::Context<'ctx>;

    type Item = Item;
    type Meta = Meta;
    type CanPend = CanPend;
    type CanEnd = CanEnd;

    fn pull(
        self: Pin<&mut Self>,
        ctx: &mut Self::Ctx<'_>,
    ) -> Step<Self::Item, Self::Meta, Self::CanPend, Self::CanEnd> {
        let this = self.get_mut();
        (this.func)(ctx)
    }
}
