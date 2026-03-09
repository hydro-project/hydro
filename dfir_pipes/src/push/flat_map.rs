//! [`FlatMap`] push combinator.
use core::marker::PhantomData;
use core::pin::Pin;

use crate::push::{Push, PushStep};

/// Push combinator that maps each item to an iterator and pushes each element.
#[must_use = "`Push`es do nothing unless items are pushed into them"]
pub struct FlatMap<Next, Func, IntoIter: IntoIterator, In> {
    next: Next,
    func: Func,
    // Buffered: (iterator, next_item_to_send). None when empty.
    buffer: Option<(IntoIter::IntoIter, IntoIter::Item)>,
    _phantom: PhantomData<fn(In)>,
}

impl<Next: Unpin, Func, IntoIter: IntoIterator, In> Unpin for FlatMap<Next, Func, IntoIter, In> {}

impl<Next, Func, IntoIter: IntoIterator, In> FlatMap<Next, Func, IntoIter, In> {
    /// Creates with flat-mapping `func` and next `push`.
    pub fn new<Meta: Copy>(func: Func, next: Next) -> Self
    where
        Next: Push<IntoIter::Item, Meta>,
        Func: FnMut(In) -> IntoIter,
    {
        Self {
            next,
            func,
            buffer: None,
            _phantom: PhantomData,
        }
    }
}

impl<Next, Func, IntoIter, In, Meta: Copy + Default> Push<In, Meta>
    for FlatMap<Next, Func, IntoIter, In>
where
    Next: Push<IntoIter::Item, Meta>,
    Func: FnMut(In) -> IntoIter,
    IntoIter: IntoIterator,
{
    type Ctx<'ctx> = <Next as Push<IntoIter::Item, Meta>>::Ctx<'ctx>;

    type CanPend = <Next as Push<IntoIter::Item, Meta>>::CanPend;

    fn poll_ready(self: Pin<&mut Self>, ctx: &mut Self::Ctx<'_>) -> PushStep<Self::CanPend> {
        let this = unsafe { self.get_unchecked_mut() };

        while let Some((ref mut iter, ref mut next_item_slot)) = this.buffer {
            match unsafe { Pin::new_unchecked(&mut this.next) }.poll_ready(ctx) {
                PushStep::Done => {}
                step @ PushStep::Pending(_) => return step,
            }
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

    fn start_send(self: Pin<&mut Self>, item: In, _meta: Meta) {
        let this = unsafe { self.get_unchecked_mut() };
        assert!(
            this.buffer.is_none(),
            "FlatMap: poll_ready must be called before start_send"
        );
        let mut iter = (this.func)(item).into_iter();
        this.buffer = iter.next().map(|next_item| (iter, next_item));
    }

    fn poll_flush(mut self: Pin<&mut Self>, ctx: &mut Self::Ctx<'_>) -> PushStep<Self::CanPend> {
        match self.as_mut().poll_ready(ctx) {
            PushStep::Done => {}
            step @ PushStep::Pending(_) => return step,
        }
        let this = unsafe { self.get_unchecked_mut() };
        unsafe { Pin::new_unchecked(&mut this.next) }.poll_flush(ctx)
    }
}
