//! [`FlatMap`] push combinator.
use core::pin::Pin;

use pin_project_lite::pin_project;

use crate::push::{Push, PushStep, ready};

pin_project! {
    /// Push combinator that maps each item to an iterator and pushes each element.
    #[must_use = "`Push`es do nothing unless items are pushed into them"]
    pub struct FlatMap<Next, Func, IntoIter, Meta>
    where
        IntoIter: IntoIterator,
    {
        #[pin]
        next: Next,
        func: Func,
        buffer: Option<(IntoIter::IntoIter, IntoIter::Item, Meta)>,
    }
}

impl<Next, Func, IntoIter, Meta> FlatMap<Next, Func, IntoIter, Meta>
where
    IntoIter: IntoIterator,
{
    /// Creates with flat-mapping `func` and next `push`.
    pub(crate) const fn new<In>(func: Func, next: Next) -> Self
    where
        Meta: Copy,
        Next: Push<IntoIter::Item, Meta>,
        Func: FnMut(In) -> IntoIter,
    {
        Self {
            next,
            func,
            buffer: None,
        }
    }
}

impl<Next, Func, IntoIter, In, Meta> Push<In, Meta> for FlatMap<Next, Func, IntoIter, Meta>
where
    Next: Push<IntoIter::Item, Meta>,
    Func: FnMut(In) -> IntoIter,
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

    fn start_send(self: Pin<&mut Self>, item: In, meta: Meta) {
        let this = self.project();
        assert!(
            this.buffer.is_none(),
            "FlatMap: poll_ready must be called before start_send"
        );
        let mut iter = (this.func)(item).into_iter();
        *this.buffer = iter.next().map(|next_item| (iter, next_item, meta));
    }

    fn poll_flush(mut self: Pin<&mut Self>, ctx: &mut Self::Ctx<'_>) -> PushStep<Self::CanPend> {
        ready!(self.as_mut().poll_ready(ctx));
        self.project().next.poll_flush(ctx)
    }
}

#[cfg(test)]
mod tests {
    use core::pin::Pin;

    use crate::push::Push;
    use crate::push::test_utils::ReadyGuardPush;

    #[test]
    fn flat_map_readies_downstream_before_each_send() {
        let mut fm = crate::push::flat_map(|x: i32| [x, x + 10], ReadyGuardPush::new());
        let mut fm = Pin::new(&mut fm);
        fm.as_mut().poll_ready(&mut ());
        fm.as_mut().start_send(1, ());
        fm.as_mut().poll_ready(&mut ());
        fm.as_mut().start_send(2, ());
        fm.as_mut().poll_flush(&mut ());
    }
}
