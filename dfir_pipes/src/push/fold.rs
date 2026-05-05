//! [`Fold`] push combinator.
use core::pin::Pin;

use pin_project_lite::pin_project;

use crate::push::{Push, PushStep};

pin_project! {
    /// Push combinator that accumulates all items via a fold function, then emits
    /// the accumulated value downstream on flush.
    ///
    /// During `start_send`, items are folded into the accumulator.
    /// During `poll_flush`, the accumulated value is cloned and sent downstream,
    /// then the downstream is flushed.
    #[must_use = "`Push`es do nothing unless items are pushed into them"]
    #[derive(Clone, Debug)]
    pub struct Fold<Acc, CombFn, Next> {
        #[pin]
        next: Next,
        acc: Acc,
        comb_fn: CombFn,
        flushed: bool,
    }
}

impl<Acc, CombFn, Next> Fold<Acc, CombFn, Next> {
    /// Creates a new `Fold` push combinator with the given initial accumulator value.
    pub const fn new(acc: Acc, comb_fn: CombFn, next: Next) -> Self {
        Self {
            next,
            acc,
            comb_fn,
            flushed: false,
        }
    }
}

// TODO(mingwei): support arbitrary metadata.
impl<Acc, CombFn, Item, Next> Push<Item, ()> for Fold<Acc, CombFn, Next>
where
    Acc: Clone,
    CombFn: FnMut(&mut Acc, Item),
    Next: Push<Acc, ()>,
{
    type Ctx<'ctx> = Next::Ctx<'ctx>;

    type CanPend = Next::CanPend;

    fn poll_ready(self: Pin<&mut Self>, _ctx: &mut Self::Ctx<'_>) -> PushStep<Self::CanPend> {
        PushStep::Done
    }

    fn start_send(self: Pin<&mut Self>, item: Item, _meta: ()) {
        let this = self.project();
        (this.comb_fn)(this.acc, item);
        *this.flushed = false;
    }

    fn poll_flush(self: Pin<&mut Self>, ctx: &mut Self::Ctx<'_>) -> PushStep<Self::CanPend> {
        let mut this = self.project();
        if !*this.flushed {
            *this.flushed = true;
            let value = this.acc.clone();
            this.next.as_mut().start_send(value, ());
        }
        this.next.poll_flush(ctx)
    }

    fn size_hint(self: Pin<&mut Self>, _hint: (usize, Option<usize>)) {
        self.project().next.size_hint((1, Some(1)));
    }
}
