//! [`ForEach`] terminal push combinator.
use core::marker::PhantomData;
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
    pub struct ForEach<Func, Item, Meta = ()> {
        func: Func,
        _phantom: PhantomData<fn(Item, Meta)>,
    }
}

impl<Func, Item> ForEach<Func, Item> {
    /// Creates with consuming `func`.
    pub fn new(func: Func) -> Self
    where
        Func: FnMut(Item),
    {
        Self {
            func,
            _phantom: PhantomData,
        }
    }
}

impl<Func, Item, Meta> ForEach<Func, Item, Meta> {
    /// Creates with consuming `func` and a specific metadata type.
    pub fn new_with_meta(func: Func) -> Self
    where
        Func: FnMut(Item),
    {
        Self {
            func,
            _phantom: PhantomData,
        }
    }
}

impl<Func, Item, Meta: Copy> Push<Item, Meta> for ForEach<Func, Item, Meta>
where
    Func: FnMut(Item),
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
}
