//! [`FlattenStream`] push combinator.
use core::pin::Pin;
use core::task::{Context, Poll};

use futures_core::Stream;
use pin_project_lite::pin_project;

use crate::Yes;
use crate::push::{Push, PushStep, ready};

pin_project! {
    struct FlattenStreamBuffer<St, Meta> where St: Stream {
        #[pin]
        stream: St,
        item: Option<St::Item>,
        meta: Meta,
    }
}

pin_project! {
    /// Push combinator that flattens stream items by polling each stream and pushing elements downstream.
    ///
    /// When the inner stream yields `Poll::Pending`, this operator yields as well.
    #[must_use = "`Push`es do nothing unless items are pushed into them"]
    pub struct FlattenStream<Next, St, Meta>
    where
        St: Stream,
    {
        #[pin]
        next: Next,
        #[pin]
        buffer: Option<FlattenStreamBuffer<St, Meta>>,
    }
}

impl<Next, St, Meta> FlattenStream<Next, St, Meta>
where
    Next: Push<St::Item, Meta>,
    St: Stream,
    Meta: Copy,
{
    /// Creates with next `push`.
    pub(crate) const fn new(next: Next) -> Self {
        Self { next, buffer: None }
    }
}

impl<Next, St, Meta> Push<St, Meta> for FlattenStream<Next, St, Meta>
where
    Next: Push<St::Item, Meta>,
    St: Stream,
    Meta: Copy,
{
    type Ctx<'ctx> = Context<'ctx>;

    type CanPend = Yes;

    fn size_hint(self: Pin<&mut Self>, _hint: (usize, Option<usize>)) {
        let this = self.project();
        let lower = this
            .buffer
            .as_pin_mut()
            .map(|b| b.project().stream.size_hint().0)
            .unwrap_or_default();
        this.next.size_hint((lower, None));
    }

    fn poll_ready(self: Pin<&mut Self>, ctx: &mut Self::Ctx<'_>) -> PushStep<Self::CanPend> {
        let mut this = self.project();

        while let Some(buf) = this.buffer.as_mut().as_pin_mut().map(|buf| buf.project()) {
            if buf.item.is_some() {
                ready!(
                    this.next
                        .as_mut()
                        .poll_ready(crate::Context::from_task(ctx))
                );
                let item = buf.item.take().unwrap();
                this.next.as_mut().start_send(item, *buf.meta);
            }
            debug_assert!(buf.item.is_none());

            match Stream::poll_next(buf.stream, ctx) {
                Poll::Ready(Some(next_item)) => *buf.item = Some(next_item),
                Poll::Ready(None) => this.buffer.as_mut().set(None),
                Poll::Pending => return PushStep::Pending(Yes),
            }
        }
        PushStep::Done
    }

    fn start_send(self: Pin<&mut Self>, stream: St, meta: Meta) {
        let mut this = self.project();
        assert!(
            this.buffer.is_none(),
            "FlattenStream: poll_ready must be called before start_send"
        );
        this.buffer.set(Some(FlattenStreamBuffer {
            stream,
            item: None,
            meta,
        }));
    }

    fn poll_flush(mut self: Pin<&mut Self>, ctx: &mut Self::Ctx<'_>) -> PushStep<Self::CanPend> {
        ready!(self.as_mut().poll_ready(ctx));
        self.project()
            .next
            .poll_flush(crate::Context::from_task(ctx))
            .convert_into()
    }
}

#[cfg(test)]
mod tests {
    use core::pin::Pin;
    use core::task::{Context, Waker};

    extern crate alloc;
    use alloc::vec;

    use futures_util::stream;

    use crate::push::Push;
    use crate::push::test_utils::TestPush;

    #[test]
    fn flatten_stream_readies_downstream_before_each_send() {
        let waker = Waker::noop();
        let mut cx = Context::from_waker(waker);

        let mut tp = TestPush::no_pend();
        let mut fs =
            crate::push::flatten_stream::<stream::Iter<vec::IntoIter<i32>>, (), _>(&mut tp);
        let mut fs = Pin::new(&mut fs);

        let result = Push::<stream::Iter<vec::IntoIter<i32>>, ()>::poll_ready(fs.as_mut(), &mut cx);
        assert!(result.is_done());

        Push::<stream::Iter<vec::IntoIter<i32>>, ()>::start_send(
            fs.as_mut(),
            stream::iter(vec![1, 2]),
            (),
        );

        let result = Push::<stream::Iter<vec::IntoIter<i32>>, ()>::poll_ready(fs.as_mut(), &mut cx);
        assert!(result.is_done());

        Push::<stream::Iter<vec::IntoIter<i32>>, ()>::start_send(
            fs.as_mut(),
            stream::iter(vec![3]),
            (),
        );

        let result = Push::<stream::Iter<vec::IntoIter<i32>>, ()>::poll_flush(fs.as_mut(), &mut cx);
        assert!(result.is_done());

        assert_eq!(tp.items(), vec![1, 2, 3]);
    }

    #[test]
    fn flatten_stream_pending_propagates() {
        let waker = Waker::noop();
        let mut cx = Context::from_waker(waker);

        let mut tp: TestPush<i32, crate::No, true> = TestPush::new_fused([], []);
        let mut fs = crate::push::flatten_stream::<stream::Pending<i32>, (), _>(&mut tp);
        let mut fs = Pin::new(&mut fs);

        // Ready initially (no stream buffered).
        let result = Push::<stream::Pending<i32>, ()>::poll_ready(fs.as_mut(), &mut cx);
        assert!(result.is_done());

        // Send a stream that is always pending.
        Push::<stream::Pending<i32>, ()>::start_send(fs.as_mut(), stream::pending(), ());

        // poll_ready should return Pending since the stream pends.
        let result = Push::<stream::Pending<i32>, ()>::poll_ready(fs.as_mut(), &mut cx);
        assert!(result.is_pending());
    }
}
