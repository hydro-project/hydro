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
    pub const fn new<Item0, Item1, Meta: Copy>(push_0: Push0, push_1: Push1) -> Self
    where
        Push0: Push<Item0, Meta>,
        Push1: Push<Item1, Meta>,
    {
        Self { push_0, push_1 }
    }
}

impl<P0, P1, Item0, Item1, Meta: Copy> Push<(Item0, Item1), Meta> for Unzip<P0, P1>
where
    P0: Push<Item0, Meta>,
    P1: Push<Item1, Meta>,
{
    type Ctx<'ctx> = <<P0 as Push<Item0, Meta>>::Ctx<'ctx> as Context<'ctx>>::Merged<
        <P1 as Push<Item1, Meta>>::Ctx<'ctx>,
    >;

    type CanPend =
        <<P0 as Push<Item0, Meta>>::CanPend as Toggle>::Or<<P1 as Push<Item1, Meta>>::CanPend>;

    fn poll_ready(self: Pin<&mut Self>, ctx: &mut Self::Ctx<'_>) -> PushStep<Self::CanPend> {
        let this = self.project();
        ready_both!(
            this.push_0
                .poll_ready(<<P0 as Push<Item0, Meta>>::Ctx<'_> as Context<'_>>::unmerge_self(ctx)),
            this.push_1.poll_ready(
                <<P0 as Push<Item0, Meta>>::Ctx<'_> as Context<'_>>::unmerge_other(ctx)
            ),
        );
        PushStep::Done
    }

    fn start_send(self: Pin<&mut Self>, item: (Item0, Item1), meta: Meta) {
        let this = self.project();
        let (item0, item1) = item;
        this.push_0.start_send(item0, meta);
        this.push_1.start_send(item1, meta);
    }

    fn poll_flush(self: Pin<&mut Self>, ctx: &mut Self::Ctx<'_>) -> PushStep<Self::CanPend> {
        let this = self.project();
        ready_both!(
            this.push_0
                .poll_flush(<<P0 as Push<Item0, Meta>>::Ctx<'_> as Context<'_>>::unmerge_self(ctx)),
            this.push_1.poll_flush(
                <<P0 as Push<Item0, Meta>>::Ctx<'_> as Context<'_>>::unmerge_other(ctx)
            ),
        );
        PushStep::Done
    }
}
