//! [`Flatten`] push combinator.
use core::pin::Pin;

use pin_project_lite::pin_project;

use crate::push::{Push, PushStep, ready};

pin_project! {
    /// Push combinator that flattens iterable items by pushing each element downstream.
    #[must_use = "`Push`es do nothing unless items are pushed into them"]
    pub struct Flatten<Next, IntoIter, Meta>
    where
        IntoIter: IntoIterator,
    {
        #[pin]
        next: Next,
        // Current iterator and the next item.
        buffer: Option<(IntoIter::IntoIter, IntoIter::Item, Meta)>,
    }
}

impl<Next, IntoIter, Meta> Flatten<Next, IntoIter, Meta>
where
    IntoIter: IntoIterator,
{
    /// Creates with next `push`.
    pub(crate) const fn new(next: Next) -> Self
    where
        Meta: Copy,
        Next: Push<IntoIter::Item, Meta>,
    {
        Self { next, buffer: None }
    }
}

impl<Next, IntoIter, Meta> Push<IntoIter, Meta> for Flatten<Next, IntoIter, Meta>
where
    Next: Push<IntoIter::Item, Meta>,
    IntoIter: IntoIterator,
    Meta: Copy,
{
    type Ctx<'ctx> = Next::Ctx<'ctx>;

    type CanPend = Next::CanPend;

    fn poll_ready(self: Pin<&mut Self>, ctx: &mut Self::Ctx<'_>) -> PushStep<Self::CanPend> {
        let mut this = self.project();

        while let Some((iter, item, meta)) = this.buffer.as_mut() {
            // Ensure following sink is ready.
            ready!(this.next.as_mut().poll_ready(ctx));
            let meta = *meta;

            // Swap in the next item.
            let item = if let Some(next_item) = iter.next() {
                core::mem::replace(item, next_item)
            } else {
                let (_, item, _) = this.buffer.take().unwrap();
                item
            };

            // Send the prev item.
            this.next.as_mut().start_send(item, meta);
        }
        this.next.poll_ready(ctx)
    }

    fn start_send(self: Pin<&mut Self>, item: IntoIter, meta: Meta) {
        let this = self.project();
        assert!(
            this.buffer.is_none(),
            "Flatten: poll_ready must be called before start_send"
        );
        let mut iter = item.into_iter();
        *this.buffer = iter.next().map(|next_item| (iter, next_item, meta));
    }

    fn poll_flush(mut self: Pin<&mut Self>, ctx: &mut Self::Ctx<'_>) -> PushStep<Self::CanPend> {
        ready!(self.as_mut().poll_ready(ctx));
        self.project().next.poll_flush(ctx)
    }

    fn size_hint(self: Pin<&mut Self>, _hint: (usize, Option<usize>)) {
        self.project().next.size_hint((0, None));
    }
}

#[cfg(test)]
mod tests {
    use core::pin::Pin;

    extern crate alloc;
    use alloc::vec;
    use alloc::vec::Vec;

    use crate::push::Push;
    use crate::push::test_utils::TestPush;

    #[test]
    fn flatten_readies_downstream_before_each_send() {
        let mut tp = TestPush::no_pend();
        let mut fl = crate::push::flatten::<Vec<i32>, (), _>(&mut tp);
        let mut fl = Pin::new(&mut fl);
        fl.as_mut().poll_ready(&mut ());
        fl.as_mut().start_send(vec![1, 2], ());
        fl.as_mut().poll_ready(&mut ());
        fl.as_mut().start_send(vec![3], ());
        fl.as_mut().poll_flush(&mut ());
    }
}
