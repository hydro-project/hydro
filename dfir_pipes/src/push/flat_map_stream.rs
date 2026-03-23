//! [`FlatMapStream`] push combinator.
use core::pin::Pin;
use core::task::{Context, Poll};

use futures_core::Stream;
use pin_project_lite::pin_project;

use crate::Yes;
use crate::push::{Push, PushStep, ready};

pin_project! {
    struct FlatMapStreamBuffer<St, Meta> where St: Stream {
        #[pin]
        stream: St,
        item: Option<St::Item>,
        meta: Meta,
    }
}

pin_project! {
    /// Push combinator that maps each item to a stream and pushes each element downstream.
    ///
    /// When the inner stream yields `Poll::Pending`, this operator yields as well.
    #[must_use = "`Push`es do nothing unless items are pushed into them"]
    pub struct FlatMapStream<Next, Func, St, Meta>
    where
        St: Stream,
    {
        #[pin]
        next: Next,
        func: Func,
        #[pin]
        buffer: Option<FlatMapStreamBuffer<St, Meta>>,
    }
}

impl<Next, Func, St, Meta> FlatMapStream<Next, Func, St, Meta>
where
    Next: Push<St::Item, Meta>,
    St: Stream,
    Meta: Copy,
{
    /// Creates with flat-mapping `func` and next `push`.
    pub(crate) const fn new<In>(func: Func, next: Next) -> Self
    where
        Func: FnMut(In) -> St,
    {
        Self {
            next,
            func,
            buffer: None,
        }
    }
}

impl<Next, Func, St, In, Meta> Push<In, Meta> for FlatMapStream<Next, Func, St, Meta>
where
    Next: Push<St::Item, Meta>,
    Func: FnMut(In) -> St,
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

    fn start_send(self: Pin<&mut Self>, item: In, meta: Meta) {
        let mut this = self.project();
        assert!(
            this.buffer.is_none(),
            "FlatMapStream: poll_ready must be called before start_send"
        );
        let stream = (this.func)(item);
        this.buffer.set(Some(FlatMapStreamBuffer {
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
    fn flat_map_stream_readies_downstream_before_each_send() {
        let waker = Waker::noop();
        let mut cx = Context::from_waker(waker);

        let mut tp = TestPush::no_pend();
        let mut fms = crate::push::flat_map_stream::<_, _, stream::Iter<vec::IntoIter<i32>>, (), _>(
            |x: i32| stream::iter(vec![x, x + 10]),
            &mut tp,
        );
        let mut fms = Pin::new(&mut fms);

        let result = Push::<i32, ()>::poll_ready(fms.as_mut(), &mut cx);
        assert!(result.is_done());

        Push::<i32, ()>::start_send(fms.as_mut(), 1, ());

        let result = Push::<i32, ()>::poll_ready(fms.as_mut(), &mut cx);
        assert!(result.is_done());

        Push::<i32, ()>::start_send(fms.as_mut(), 2, ());

        let result = Push::<i32, ()>::poll_flush(fms.as_mut(), &mut cx);
        assert!(result.is_done());

        assert_eq!(tp.items(), vec![1, 11, 2, 12]);
    }

    #[test]
    fn flat_map_stream_pending_propagates() {
        let waker = Waker::noop();
        let mut cx = Context::from_waker(waker);

        let mut tp: TestPush<i32, crate::No, true> = TestPush::new_fused([], []);
        let mut fms = crate::push::flat_map_stream::<_, _, stream::Pending<i32>, (), _>(
            |_: i32| stream::pending(),
            &mut tp,
        );
        let mut fms = Pin::new(&mut fms);

        let result = Push::<i32, ()>::poll_ready(fms.as_mut(), &mut cx);
        assert!(result.is_done());

        Push::<i32, ()>::start_send(fms.as_mut(), 42, ());

        let result = Push::<i32, ()>::poll_ready(fms.as_mut(), &mut cx);
        assert!(result.is_pending());
    }
}
