//! [`Fold`] push combinator.
use core::borrow::BorrowMut;
use core::marker::PhantomData;
use core::pin::Pin;

use pin_project_lite::pin_project;

use crate::push::{Push, PushStep, ready};

pin_project! {
    /// Push combinator that accumulates all items via a fold function, then emits
    /// the accumulated value downstream on finalize.
    ///
    /// During `start_send`, items are folded into the accumulator.
    /// During `poll_finalize`, the accumulated value is taken and sent downstream,
    /// then the downstream is finalized.
    ///
    /// `AccRef` is typically `&'a mut AccRef` — a mutable reference to externally-owned state.
    #[must_use = "`Push`es do nothing unless items are pushed into them"]
    pub struct Fold<Next, AccRef, CombFn, Acc> {
        #[pin]
        next: Next,
        acc_ref: AccRef,
        comb_fn: CombFn,
        _phantom: PhantomData<Acc>,
    }
}

impl<Next, AccRef, CombFn, Acc> Fold<Next, AccRef, CombFn, Acc> {
    /// Creates a new `Fold` push combinator with the given initial accumulator value.
    pub const fn new(acc_ref: AccRef, comb_fn: CombFn, next: Next) -> Self {
        Self {
            next,
            acc_ref,
            comb_fn,
            _phantom: PhantomData,
        }
    }
}

// TODO(mingwei): support arbitrary metadata.
impl<Next, AccRef, CombFn, Acc, Item, Meta> Push<Item, Meta> for Fold<Next, AccRef, CombFn, Acc>
where
    Next: Push<AccRef, Meta>,
    AccRef: BorrowMut<Acc>,
    CombFn: FnMut(&mut Acc, Item),
    Meta: Copy,
{
    type Ctx<'ctx> = Next::Ctx<'ctx>;

    type CanPend = Next::CanPend;

    fn poll_ready(self: Pin<&mut Self>, _ctx: &mut Self::Ctx<'_>) -> PushStep<Self::CanPend> {
        PushStep::Done
    }

    fn start_send(self: Pin<&mut Self>, item: Item, meta: Meta) {
        let this = self.project();
        (this.comb_fn)(this.acc_ref.borrow_mut(), item);
    }

    fn poll_finalize(self: Pin<&mut Self>, ctx: &mut Self::Ctx<'_>) -> PushStep<Self::CanPend> {
        let mut this = self.project();
        if this.acc_ref.is_some() {
            ready!(this.next.as_mut().poll_ready(ctx));
            let value = this.acc.take().unwrap();
            this.next.as_mut().start_send(value, ());
        }
        this.next.poll_finalize(ctx)
    }

    fn size_hint(self: Pin<&mut Self>, _hint: (usize, Option<usize>)) {
        self.project().next.size_hint((1, Some(1)));
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec;
    use core::pin::Pin;

    use crate::Yes;
    use crate::push::test_utils::{PushCall, TestPush};
    use crate::push::{Push, PushStep};

    #[test]
    fn fold_emits_on_finalize() {
        let mut tp = TestPush::no_pend();
        let mut f = crate::push::fold(0i32, |acc, x| *acc += x, &mut tp);
        let mut f = Pin::new(&mut f);
        f.as_mut().poll_ready(&mut ());
        f.as_mut().start_send(1, ());
        f.as_mut().poll_ready(&mut ());
        f.as_mut().start_send(2, ());
        f.as_mut().poll_ready(&mut ());
        f.as_mut().start_send(3, ());
        f.as_mut().poll_finalize(&mut ());
        assert_eq!(tp.items(), vec![6]);
    }

    #[test]
    fn fold_emits_initial_when_no_items() {
        let mut tp = TestPush::no_pend();
        let mut f = crate::push::fold(0i32, |acc, x: i32| *acc += x, &mut tp);
        let mut f = Pin::new(&mut f);
        f.as_mut().poll_finalize(&mut ());
        assert_eq!(tp.items(), vec![0]);
    }

    #[test]
    fn fold_poll_ready_before_send_on_finalize() {
        let mut tp: TestPush<i32, Yes, true> = TestPush::new_fused([PushStep::pending()], []);
        let mut f = crate::push::fold(0i32, |acc, x| *acc += x, &mut tp);
        let mut f = Pin::new(&mut f);
        f.as_mut().poll_ready(&mut ());
        f.as_mut().start_send(5, ());
        // First call: downstream poll_ready returns Pending.
        let step = f.as_mut().poll_finalize(&mut ());
        assert!(step.is_pending());
        // Second call: poll_ready returns Done (fused), send proceeds.
        let step = f.as_mut().poll_finalize(&mut ());
        assert!(step.is_done());
        assert_eq!(tp.items(), vec![5]);
        assert_eq!(tp.history[0], PushCall::PollReady);
        assert_eq!(tp.history[1], PushCall::PollReady);
        assert_eq!(tp.history[2], PushCall::SendItem(5));
    }
}
