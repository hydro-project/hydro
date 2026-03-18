use alloc::vec::Vec;
use core::borrow::BorrowMut;
use core::pin::Pin;

use pin_project_lite::pin_project;

use crate::push::{Push, PushStep, ready};

pin_project! {
    /// Special push operator for the `persist` operator.
    #[must_use = "`Push`es do nothing unless items are pushed into them"]
    pub struct Persist<Psh, Buf> {
        #[pin]
        push: Psh,
        buf: Buf,
        replay_idx: usize,
    }
}

impl<Psh, Buf> Persist<Psh, Buf> {
    /// Create with the given replay index and following push.
    pub(crate) fn new<Item>(buf: Buf, replay: bool, push: Psh) -> Self
    where
        Psh: Push<Item, ()>,
        Item: Clone,
        Buf: BorrowMut<Vec<Item>>,
    {
        let replay_idx = if replay { 0 } else { buf.borrow().len() };
        Self {
            push,
            buf,
            replay_idx,
        }
    }

    /// Drains any pending replay items by pushing them downstream.
    fn empty_replay<Item>(self: Pin<&mut Self>, ctx: &mut Psh::Ctx<'_>) -> PushStep<Psh::CanPend>
    where
        Psh: Push<Item, ()>,
        Item: Clone,
        Buf: BorrowMut<Vec<Item>>,
    {
        let mut this = self.project();
        while let Some(item) = this.buf.borrow().get(*this.replay_idx) {
            ready!(this.push.as_mut().poll_ready(ctx));
            this.push.as_mut().start_send(item.clone(), ());
            *this.replay_idx += 1;
        }
        debug_assert_eq!(this.buf.borrow().len(), *this.replay_idx);
        PushStep::Done
    }
}

// TODO(mingwei): support arbitrary metadata.
impl<Psh, Item, Buf> Push<Item, ()> for Persist<Psh, Buf>
where
    Psh: Push<Item, ()>,
    Item: Clone,
    Buf: BorrowMut<Vec<Item>>,
{
    type Ctx<'ctx> = Psh::Ctx<'ctx>;

    type CanPend = Psh::CanPend;

    fn poll_ready(mut self: Pin<&mut Self>, ctx: &mut Self::Ctx<'_>) -> PushStep<Self::CanPend> {
        // Drain any pending replay items first.
        ready!(self.as_mut().empty_replay(ctx));
        // Then ready the downstream push.
        self.project().push.poll_ready(ctx)
    }

    fn start_send(self: Pin<&mut Self>, item: Item, _meta: ()) {
        let this = self.project();
        debug_assert_eq!(this.buf.borrow().len(), *this.replay_idx);

        // Persist the new item.
        this.buf.borrow_mut().push(item.clone());
        *this.replay_idx += 1;

        // Push it downstream (downstream was readied via poll_ready).
        this.push.start_send(item, ());
    }

    fn poll_flush(mut self: Pin<&mut Self>, ctx: &mut Self::Ctx<'_>) -> PushStep<Self::CanPend> {
        // Ensure all replayed items are sent before flushing the underlying sink.
        ready!(self.as_mut().empty_replay(ctx));
        // Then flush the downstream push.
        self.project().push.poll_flush(ctx)
    }

    fn size_hint(self: Pin<&mut Self>, hint: (usize, Option<usize>)) {
        let this = self.project();
        this.buf.borrow_mut().reserve(hint.0);
        this.push.size_hint(hint);
    }
}

#[cfg(test)]
mod tests {
    use core::pin::Pin;

    extern crate alloc;
    use alloc::vec::Vec;

    use crate::push::Push;
    use crate::push::test_utils::TestPush;

    #[test]
    fn persist_readies_downstream_for_replay_and_new() {
        let mut buf = Vec::new();
        // First pass: persist items 1, 2.
        {
            let mut tp = TestPush::no_pend();
            let mut p = crate::push::persist_state(&mut buf, false, &mut tp);
            let mut p = Pin::new(&mut p);
            p.as_mut().poll_ready(&mut ());
            p.as_mut().start_send(1, ());
            p.as_mut().poll_ready(&mut ());
            p.as_mut().start_send(2, ());
            p.as_mut().poll_flush(&mut ());
        }
        // Second pass: replay=true, should replay 1, 2 then accept new item 3.
        {
            let mut tp = TestPush::no_pend();
            let mut p = crate::push::persist_state(&mut buf, true, &mut tp);
            let mut p = Pin::new(&mut p);
            p.as_mut().poll_ready(&mut ());
            p.as_mut().start_send(3, ());
            p.as_mut().poll_flush(&mut ());
        }
    }
}
