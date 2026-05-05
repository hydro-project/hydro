//! [`Reduce`] push combinator.
use core::pin::Pin;

use pin_project_lite::pin_project;

use crate::push::{Push, PushStep};

pin_project! {
    /// Push combinator that reduces all items into a single value, then emits
    /// it downstream on finalize. If no items were received, nothing is emitted.
    ///
    /// During `start_send`, items are reduced into the accumulator.
    /// During `poll_finalize`, the accumulated value (if any) is sent downstream.
    #[must_use = "`Push`es do nothing unless items are pushed into them"]
    #[derive(Clone, Debug)]
    pub struct Reduce<Acc, ReduceFn, Next> {
        #[pin]
        next: Next,
        acc: Option<Acc>,
        reduce_fn: ReduceFn,
        flushed: bool,
    }
}

impl<Acc, ReduceFn, Next> Reduce<Acc, ReduceFn, Next> {
    /// Creates a new `Reduce` push combinator.
    pub const fn new(reduce_fn: ReduceFn, next: Next) -> Self {
        Self {
            next,
            acc: None,
            reduce_fn,
            flushed: false,
        }
    }
}

// TODO(mingwei): support arbitrary metadata.
impl<Acc, ReduceFn, Next> Push<Acc, ()> for Reduce<Acc, ReduceFn, Next>
where
    Acc: Clone,
    ReduceFn: FnMut(&mut Acc, Acc),
    Next: Push<Acc, ()>,
{
    type Ctx<'ctx> = Next::Ctx<'ctx>;

    type CanPend = Next::CanPend;

    fn poll_ready(self: Pin<&mut Self>, _ctx: &mut Self::Ctx<'_>) -> PushStep<Self::CanPend> {
        PushStep::Done
    }

    fn start_send(self: Pin<&mut Self>, item: Acc, _meta: ()) {
        let this = self.project();
        match this.acc {
            Some(acc) => (this.reduce_fn)(acc, item),
            None => *this.acc = Some(item),
        }
        *this.flushed = false;
    }

    fn poll_finalize(self: Pin<&mut Self>, ctx: &mut Self::Ctx<'_>) -> PushStep<Self::CanPend> {
        let mut this = self.project();
        if !*this.flushed {
            *this.flushed = true;
            if let Some(value) = this.acc.as_ref() {
                this.next.as_mut().start_send(value.clone(), ());
            }
        }
        this.next.poll_finalize(ctx)
    }

    fn size_hint(self: Pin<&mut Self>, _hint: (usize, Option<usize>)) {
        self.project().next.size_hint((0, Some(1)));
    }
}
