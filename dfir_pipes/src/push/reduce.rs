//! [`Reduce`] push combinator.
use core::pin::Pin;

use pin_project_lite::pin_project;

use crate::push::{Push, PushStep};

pin_project! {
    /// Push combinator that reduces all items into a single value, then emits
    /// it downstream on flush. If no items were received, nothing is emitted.
    ///
    /// `AccRef` is typically `&'a mut Option<Item>` — a mutable reference to externally-owned state.
    #[must_use = "`Push`es do nothing unless items are pushed into them"]
    pub struct Reduce<AccRef, ReduceFn, Next> {
        #[pin]
        next: Next,
        acc: AccRef,
        reduce_fn: ReduceFn,
        flushed: bool,
    }
}

impl<AccRef, ReduceFn, Next> Reduce<AccRef, ReduceFn, Next> {
    /// Creates a new `Reduce` push combinator.
    pub const fn new(acc: AccRef, reduce_fn: ReduceFn, next: Next) -> Self {
        Self {
            next,
            acc,
            reduce_fn,
            flushed: false,
        }
    }
}

// TODO(mingwei): support arbitrary metadata.
impl<Item, ReduceFn, Next> Push<Item, ()> for Reduce<&mut Option<Item>, ReduceFn, Next>
where
    Item: Clone,
    ReduceFn: FnMut(&mut Item, Item),
    Next: Push<Item, ()>,
{
    type Ctx<'ctx> = Next::Ctx<'ctx>;

    type CanPend = Next::CanPend;

    fn poll_ready(self: Pin<&mut Self>, _ctx: &mut Self::Ctx<'_>) -> PushStep<Self::CanPend> {
        PushStep::Done
    }

    fn start_send(self: Pin<&mut Self>, item: Item, _meta: ()) {
        let this = self.project();
        match this.acc {
            Some(acc) => (this.reduce_fn)(acc, item),
            None => **this.acc = Some(item),
        }
        *this.flushed = false;
    }

    fn poll_flush(self: Pin<&mut Self>, ctx: &mut Self::Ctx<'_>) -> PushStep<Self::CanPend> {
        let mut this = self.project();
        if !*this.flushed {
            *this.flushed = true;
            if let Some(value) = this.acc.as_ref() {
                this.next.as_mut().start_send(value.clone(), ());
            }
        }
        this.next.poll_flush(ctx)
    }

    fn size_hint(self: Pin<&mut Self>, _hint: (usize, Option<usize>)) {
        self.project().next.size_hint((0, Some(1)));
    }
}
