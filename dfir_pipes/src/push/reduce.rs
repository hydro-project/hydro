//! [`Reduce`] push combinator.
use core::pin::Pin;

use pin_project_lite::pin_project;

use crate::push::{Push, PushStep, ready};

pin_project! {
    /// Push combinator that reduces all items into a single value, then emits
    /// it downstream on finalize. If no items were received, nothing is emitted.
    ///
    /// During `start_send`, items are reduced into the accumulator.
    /// During `poll_finalize`, the accumulated value (if any) is taken and sent downstream.
    #[must_use = "`Push`es do nothing unless items are pushed into them"]
    #[derive(Clone, Debug)]
    pub struct Reduce<Acc, ReduceFn, Next> {
        #[pin]
        next: Next,
        acc: Option<Acc>,
        reduce_fn: ReduceFn,
    }
}

impl<Acc, ReduceFn, Next> Reduce<Acc, ReduceFn, Next> {
    /// Creates a new `Reduce` push combinator.
    pub const fn new(reduce_fn: ReduceFn, next: Next) -> Self {
        Self {
            next,
            acc: None,
            reduce_fn,
        }
    }
}

// TODO(mingwei): support arbitrary metadata.
impl<Acc, ReduceFn, Next> Push<Acc, ()> for Reduce<Acc, ReduceFn, Next>
where
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
    }

    fn poll_finalize(self: Pin<&mut Self>, ctx: &mut Self::Ctx<'_>) -> PushStep<Self::CanPend> {
        let mut this = self.project();
        if this.acc.is_some() {
            ready!(this.next.as_mut().poll_ready(ctx));
            let value = this.acc.take().unwrap();
            this.next.as_mut().start_send(value, ());
        }
        this.next.poll_finalize(ctx)
    }

    fn size_hint(self: Pin<&mut Self>, _hint: (usize, Option<usize>)) {
        self.project().next.size_hint((0, Some(1)));
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec;
    use alloc::vec::Vec;
    use core::pin::Pin;

    use crate::Yes;
    use crate::push::test_utils::{PushCall, TestPush};
    use crate::push::{Push, PushStep};

    #[test]
    fn reduce_emits_on_finalize() {
        let mut tp = TestPush::no_pend();
        let mut r = crate::push::reduce(|acc: &mut i32, x| *acc += x, &mut tp);
        let mut r = Pin::new(&mut r);
        r.as_mut().poll_ready(&mut ());
        r.as_mut().start_send(1, ());
        r.as_mut().poll_ready(&mut ());
        r.as_mut().start_send(2, ());
        r.as_mut().poll_ready(&mut ());
        r.as_mut().start_send(3, ());
        r.as_mut().poll_finalize(&mut ());
        assert_eq!(tp.items(), vec![6]);
    }

    #[test]
    fn reduce_no_items_no_output() {
        let mut tp = TestPush::no_pend();
        let mut r = crate::push::reduce(|acc: &mut i32, x| *acc += x, &mut tp);
        let mut r = Pin::new(&mut r);
        r.as_mut().poll_finalize(&mut ());
        assert_eq!(tp.items(), Vec::<i32>::new());
    }

    #[test]
    fn reduce_poll_ready_before_send_on_finalize() {
        let mut tp: TestPush<i32, Yes, true> = TestPush::new_fused([PushStep::pending()], []);
        let mut r = crate::push::reduce(|acc: &mut i32, x| *acc += x, &mut tp);
        let mut r = Pin::new(&mut r);
        r.as_mut().poll_ready(&mut ());
        r.as_mut().start_send(1, ());
        // First call: poll_ready returns Pending, so poll_finalize returns Pending.
        let step = r.as_mut().poll_finalize(&mut ());
        assert!(step.is_pending());
        // Second call: poll_ready returns Done (fused), send proceeds.
        let step = r.as_mut().poll_finalize(&mut ());
        assert!(step.is_done());
        assert_eq!(tp.items(), vec![1]);
        // Verify poll_ready was called before send.
        assert_eq!(tp.history[0], PushCall::PollReady);
        assert_eq!(tp.history[1], PushCall::PollReady);
        assert_eq!(tp.history[2], PushCall::SendItem(1));
    }
}
