//! [`Unzip`] push combinator.
use core::pin::Pin;

use pin_project_lite::pin_project;

use super::ready_both;
use crate::push::{Push, PushStep};
use crate::{Context, Toggle};

pin_project! {
    /// Push combinator that splits `(A, B)` items into two separate downstream pushes.
    #[must_use = "`Push`es do nothing unless items are pushed into them"]
    #[derive(Clone, Debug)]
    pub struct Unzip<Push0, Push1> {
        #[pin]
        push_0: Push0,
        #[pin]
        push_1: Push1,
    }
}

impl<Push0, Push1> Unzip<Push0, Push1> {
    /// Creates with downstream pushes `push_0` and `push_1`.
    pub(crate) const fn new(push_0: Push0, push_1: Push1) -> Self {
        Self { push_0, push_1 }
    }
}

impl<P0, P1, Item0, Item1, Meta> Push<(Item0, Item1), Meta> for Unzip<P0, P1>
where
    P0: Push<Item0, Meta>,
    P1: Push<Item1, Meta>,
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

    fn start_send(self: Pin<&mut Self>, (item0, item1): (Item0, Item1), meta: Meta) {
        let this = self.project();
        this.push_0.start_send(item0, meta);
        this.push_1.start_send(item1, meta);
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

    use super::Unzip;
    use crate::push::Push;
    use crate::push::test_utils::TestPush;

    #[test]
    fn unzip_readies_both_before_send() {
        let mut tp_a = TestPush::no_pend();
        let mut tp_b = TestPush::no_pend();
        let mut u = Unzip::new(&mut tp_a, &mut tp_b);
        let mut u = Pin::new(&mut u);
        u.as_mut().poll_ready(&mut ());
        u.as_mut().start_send((1, 2), ());
        u.as_mut().poll_ready(&mut ());
        u.as_mut().start_send((3, 4), ());
        u.as_mut().poll_flush(&mut ());
    }
}
