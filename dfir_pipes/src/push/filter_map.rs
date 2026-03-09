//! [`FilterMap`] push combinator.
use core::marker::PhantomData;
use core::pin::Pin;

use pin_project_lite::pin_project;

use crate::push::{Push, PushStep};

pin_project! {
    /// Push combinator that both filters and maps items.
    #[must_use = "`Push`es do nothing unless items are pushed into them"]
    #[derive(Clone, Debug)]
    pub struct FilterMap<Next, Func, In> {
        #[pin]
        next: Next,
        func: Func,
        _phantom: PhantomData<fn(In)>,
    }
}

impl<Next, Func, In> FilterMap<Next, Func, In> {
    /// Creates with filter-mapping `func` and next `push`.
    pub fn new<Out, Meta: Copy>(func: Func, next: Next) -> Self
    where
        Func: FnMut(In) -> Option<Out>,
        Next: Push<Out, Meta>,
    {
        Self {
            next,
            func,
            _phantom: PhantomData,
        }
    }
}

impl<Next, Func, In, Out, Meta: Copy> Push<In, Meta> for FilterMap<Next, Func, In>
where
    Next: Push<Out, Meta>,
    Func: FnMut(In) -> Option<Out>,
{
    type Ctx<'ctx> = <Next as Push<Out, Meta>>::Ctx<'ctx>;

    type CanPend = <Next as Push<Out, Meta>>::CanPend;

    fn poll_ready(self: Pin<&mut Self>, ctx: &mut Self::Ctx<'_>) -> PushStep<Self::CanPend> {
        self.project().next.poll_ready(ctx)
    }

    fn start_send(self: Pin<&mut Self>, item: In, meta: Meta) {
        let this = self.project();
        if let Some(item) = (this.func)(item) {
            this.next.start_send(item, meta)
        }
    }

    fn poll_flush(self: Pin<&mut Self>, ctx: &mut Self::Ctx<'_>) -> PushStep<Self::CanPend> {
        self.project().next.poll_flush(ctx)
    }
}
