//! [`Flatten`] push combinator.
use core::pin::Pin;

use crate::push::{Push, PushStep, ready};

/// Push combinator that flattens iterable items by pushing each element downstream.
#[must_use = "`Push`es do nothing unless items are pushed into them"]
pub struct Flatten<Next, IntoIter: IntoIterator> {
    next: Next,
    buffer: Option<(IntoIter::IntoIter, IntoIter::Item)>,
}

impl<Next: Unpin, IntoIter: IntoIterator> Unpin for Flatten<Next, IntoIter> {}

impl<Next, IntoIter: IntoIterator> Flatten<Next, IntoIter> {
    /// Creates with next `push`.
    pub const fn new<Meta: Copy>(next: Next) -> Self
    where
        Next: Push<IntoIter::Item, Meta>,
    {
        Self { next, buffer: None }
    }
}

impl<Next, IntoIter, Meta: Copy + Default> Push<IntoIter, Meta> for Flatten<Next, IntoIter>
where
    Next: Push<IntoIter::Item, Meta>,
    IntoIter: IntoIterator,
{
    type Ctx<'ctx> = <Next as Push<IntoIter::Item, Meta>>::Ctx<'ctx>;

    type CanPend = <Next as Push<IntoIter::Item, Meta>>::CanPend;

    fn poll_ready(self: Pin<&mut Self>, ctx: &mut Self::Ctx<'_>) -> PushStep<Self::CanPend> {
        let this = unsafe { self.get_unchecked_mut() };

        while let Some((ref mut iter, ref mut next_item_slot)) = this.buffer {
            ready!(unsafe { Pin::new_unchecked(&mut this.next) }.poll_ready(ctx));
            if let Some(following) = iter.next() {
                let item = core::mem::replace(next_item_slot, following);
                unsafe { Pin::new_unchecked(&mut this.next) }.start_send(item, Default::default());
            } else {
                let (_, item) = this.buffer.take().unwrap();
                unsafe { Pin::new_unchecked(&mut this.next) }.start_send(item, Default::default());
            }
        }
        unsafe { Pin::new_unchecked(&mut this.next) }.poll_ready(ctx)
    }

    fn start_send(self: Pin<&mut Self>, item: IntoIter, _meta: Meta) {
        let this = unsafe { self.get_unchecked_mut() };
        assert!(
            this.buffer.is_none(),
            "Flatten: poll_ready must be called before start_send"
        );
        let mut iter = item.into_iter();
        this.buffer = iter.next().map(|next_item| (iter, next_item));
    }

    fn poll_flush(mut self: Pin<&mut Self>, ctx: &mut Self::Ctx<'_>) -> PushStep<Self::CanPend> {
        ready!(self.as_mut().poll_ready(ctx));
        let this = unsafe { self.get_unchecked_mut() };
        unsafe { Pin::new_unchecked(&mut this.next) }.poll_flush(ctx)
    }
}
