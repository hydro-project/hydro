use alloc::vec::Vec;
use core::borrow::BorrowMut;
use core::pin::Pin;

use pin_project_lite::pin_project;

use crate::push::{Push, PushStep, ready};

pin_project! {
    /// Special push operator for the `persist` operator.
    #[must_use = "pushes do nothing unless items are pushed into them"]
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

impl<Psh, Item, Buf> Push<Item, ()> for Persist<Psh, Buf>
where
    Psh: Push<Item, ()>,
    Item: Clone,
    Buf: BorrowMut<Vec<Item>>,
{
    type Ctx<'ctx> = <Psh as Push<Item, ()>>::Ctx<'ctx>;

    type CanPend = <Psh as Push<Item, ()>>::CanPend;

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
}
