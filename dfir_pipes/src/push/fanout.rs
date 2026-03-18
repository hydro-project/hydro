//! [`Fanout`] push combinator.
use core::pin::Pin;

use pin_project_lite::pin_project;

use super::ready_both;
use crate::push::{Push, PushStep};
use crate::{Context, Toggle};

pin_project! {
    /// Push combinator that clones each item and pushes to both downstream pushes.
    ///
    /// This is the push equivalent of `futures::sink::SinkExt::fanout`.
    #[must_use = "`Push`es do nothing unless items are pushed into them"]
    #[derive(Clone, Debug)]
    pub struct Fanout<Push0, Push1> {
        #[pin]
        push_0: Push0,
        #[pin]
        push_1: Push1,
    }
}

impl<Push0, Push1> Fanout<Push0, Push1> {
    /// Creates with downstream pushes `push_0` and `push_1`.
    pub(crate) const fn new(push_0: Push0, push_1: Push1) -> Self {
        Self { push_0, push_1 }
    }
}

impl<P0, P1, Item, Meta> Push<Item, Meta> for Fanout<P0, P1>
where
    P0: Push<Item, Meta>,
    P1: Push<Item, Meta>,
    Item: Clone,
    Meta: Copy,
{
    type Ctx<'ctx> = <P0::Ctx<'ctx> as Context<'ctx>>::Merged<P1::Ctx<'ctx>>;

    type CanPend = <P0::CanPend as Toggle>::Or<P1::CanPend>;

    fn poll_ready(self: Pin<&mut Self>, ctx: &mut Self::Ctx<'_>) -> PushStep<Self::CanPend> {
        let this = self.project();
        ready_both!(
            this.push_0
                .poll_ready(<P0::Ctx<'_> as Context<'_>>::unmerge_self(ctx)),
            this.push_1
                .poll_ready(<P0::Ctx<'_> as Context<'_>>::unmerge_other(ctx)),
        );
        PushStep::Done
    }

    fn start_send(self: Pin<&mut Self>, item: Item, meta: Meta) {
        let this = self.project();
        let item_clone = item.clone();
        this.push_0.start_send(item, meta);
        this.push_1.start_send(item_clone, meta);
    }

    fn poll_flush(self: Pin<&mut Self>, ctx: &mut Self::Ctx<'_>) -> PushStep<Self::CanPend> {
        let this = self.project();
        ready_both!(
            this.push_0
                .poll_flush(<P0::Ctx<'_> as Context<'_>>::unmerge_self(ctx)),
            this.push_1
                .poll_flush(<P0::Ctx<'_> as Context<'_>>::unmerge_other(ctx)),
        );
        PushStep::Done
    }

    fn size_hint(self: Pin<&mut Self>, hint: (usize, Option<usize>)) {
        let this = self.project();
        this.push_0.size_hint(hint);
        this.push_1.size_hint(hint);
    }
}

#[cfg(test)]
mod tests {
    use core::pin::Pin;

    use super::Fanout;
    use crate::push::Push;
    use crate::push::test_utils::TestPush;

    #[test]
    fn fanout_readies_both_before_send() {
        let mut tp_a = TestPush::no_pend();
        let mut tp_b = TestPush::no_pend();
        let mut f = Fanout::new(&mut tp_a, &mut tp_b);
        let mut f = Pin::new(&mut f);
        f.as_mut().poll_ready(&mut ());
        f.as_mut().start_send(1, ());
        f.as_mut().poll_ready(&mut ());
        f.as_mut().start_send(2, ());
        f.as_mut().poll_flush(&mut ());
    }
}
