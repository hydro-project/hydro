//! [`ForEach`] terminal push combinator.
use core::pin::Pin;

use pin_project_lite::pin_project;

use crate::No;
use crate::push::{Push, PushStep};

pin_project! {
    /// Terminal push combinator that consumes each item with a closure.
    ///
    /// This is the push equivalent of the pull-side `ForEach` future.
    /// It has no downstream push; items are consumed by `func`.
    #[must_use = "`Push`es do nothing unless items are pushed into them"]
    #[derive(Clone, Debug)]
    pub struct ForEach<Func> {
        func: Func,
    }
}

impl<Func> ForEach<Func> {
    /// Creates with consuming `func`.
    pub(crate) const fn new<Item>(func: Func) -> Self
    where
        Func: FnMut(Item),
    {
        Self { func }
    }
}

impl<Func, Item, Meta> Push<Item, Meta> for ForEach<Func>
where
    Func: FnMut(Item),
    Meta: Copy,
{
    type Ctx<'ctx> = ();

    type CanPend = No;

    fn poll_ready(self: Pin<&mut Self>, _ctx: &mut Self::Ctx<'_>) -> PushStep<Self::CanPend> {
        PushStep::Done
    }

    fn start_send(self: Pin<&mut Self>, item: Item, _meta: Meta) {
        (self.project().func)(item);
    }

    fn poll_flush(self: Pin<&mut Self>, _ctx: &mut Self::Ctx<'_>) -> PushStep<Self::CanPend> {
        PushStep::Done
    }

    fn size_hint(self: Pin<&mut Self>, _hint: (usize, Option<usize>)) {
        // unused
    }
}
