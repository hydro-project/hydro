//! [`FilterMapAsync`] push combinator.
use core::pin::Pin;
use core::task::{Context, Poll};

use pin_project_lite::pin_project;

use crate::Yes;
use crate::push::{Push, PushStep, ready};

pin_project! {
    struct FilterMapAsyncBuffer<Fut, Meta> {
        #[pin]
        future: Fut,
        meta: Meta,
    }
}

pin_project! {
    /// Push combinator that applies an async filter-map function to each item.
    ///
    /// The closure returns a `Future<Output = Option<Out>>`. If the future resolves
    /// to `Some(out)`, the value is pushed downstream. If `None`, the item is dropped.
    #[must_use = "`Push`es do nothing unless items are pushed into them"]
    pub struct FilterMapAsync<Next, Func, Fut, Meta> {
        #[pin]
        next: Next,
        func: Func,
        #[pin]
        buffer: Option<FilterMapAsyncBuffer<Fut, Meta>>,
    }
}

impl<Next, Func, Fut, Meta> FilterMapAsync<Next, Func, Fut, Meta> {
    /// Creates with async filter-mapping `func` and next `push`.
    pub(crate) const fn new<In, Out>(func: Func, next: Next) -> Self
    where
        Func: FnMut(In) -> Fut,
        Fut: Future<Output = Option<Out>>,
    {
        Self {
            next,
            func,
            buffer: None,
        }
    }
}

impl<Next, Func, Fut, In, Out, Meta> Push<In, Meta> for FilterMapAsync<Next, Func, Fut, Meta>
where
    Next: Push<Out, Meta>,
    Func: FnMut(In) -> Fut,
    Fut: Future<Output = Option<Out>>,
    Meta: Copy,
{
    type Ctx<'ctx> = Context<'ctx>;

    type CanPend = Yes;

    fn poll_ready(self: Pin<&mut Self>, ctx: &mut Self::Ctx<'_>) -> PushStep<Self::CanPend> {
        let mut this = self.project();

        if let Some(buf) = this.buffer.as_mut().as_pin_mut() {
            let buf = buf.project();
            match buf.future.poll(ctx) {
                Poll::Ready(Some(out)) => {
                    let meta = *buf.meta;
                    this.buffer.as_mut().set(None);
                    ready!(
                        this.next
                            .as_mut()
                            .poll_ready(crate::Context::from_task(ctx))
                    );
                    this.next.as_mut().start_send(out, meta);
                }
                Poll::Ready(None) => {
                    this.buffer.as_mut().set(None);
                }
                Poll::Pending => return PushStep::Pending(Yes),
            }
        }

        PushStep::Done
    }

    fn start_send(self: Pin<&mut Self>, item: In, meta: Meta) {
        let mut this = self.project();
        assert!(
            this.buffer.is_none(),
            "FilterMapAsync: poll_ready must be called before start_send"
        );
        let future = (this.func)(item);
        this.buffer.set(Some(FilterMapAsyncBuffer { future, meta }));
    }

    fn poll_flush(mut self: Pin<&mut Self>, ctx: &mut Self::Ctx<'_>) -> PushStep<Self::CanPend> {
        ready!(self.as_mut().poll_ready(ctx));
        self.project()
            .next
            .poll_flush(crate::Context::from_task(ctx))
            .convert_into()
    }

    fn size_hint(self: Pin<&mut Self>, hint: (usize, Option<usize>)) {
        self.project().next.size_hint((0, hint.1));
    }
}

#[cfg(test)]
mod tests {
    use core::pin::Pin;
    use core::task::{Context, Waker};

    extern crate alloc;
    use alloc::vec;

    use crate::push::Push;
    use crate::push::test_utils::TestPush;

    #[test]
    fn filter_map_async_some_items() {
        let waker = Waker::noop();
        let mut cx = Context::from_waker(waker);

        let mut tp = TestPush::no_pend();
        let mut fma = crate::push::filter_map_async(
            |x: i32| core::future::ready(if x % 2 == 0 { Some(x * 10) } else { None }),
            &mut tp,
        );
        let mut fma = Pin::new(&mut fma);

        let result = Push::<i32, ()>::poll_ready(fma.as_mut(), &mut cx);
        assert!(result.is_done());

        Push::<i32, ()>::start_send(fma.as_mut(), 2, ());

        let result = Push::<i32, ()>::poll_ready(fma.as_mut(), &mut cx);
        assert!(result.is_done());

        Push::<i32, ()>::start_send(fma.as_mut(), 3, ());

        let result = Push::<i32, ()>::poll_ready(fma.as_mut(), &mut cx);
        assert!(result.is_done());

        Push::<i32, ()>::start_send(fma.as_mut(), 4, ());

        let result = Push::<i32, ()>::poll_flush(fma.as_mut(), &mut cx);
        assert!(result.is_done());

        assert_eq!(tp.items(), vec![20, 40]);
    }

    #[test]
    fn filter_map_async_all_none() {
        let waker = Waker::noop();
        let mut cx = Context::from_waker(waker);

        let mut tp = TestPush::no_pend();
        let mut fma =
            crate::push::filter_map_async(|_x: i32| core::future::ready(None::<i32>), &mut tp);
        let mut fma = Pin::new(&mut fma);

        let result = Push::<i32, ()>::poll_ready(fma.as_mut(), &mut cx);
        assert!(result.is_done());

        Push::<i32, ()>::start_send(fma.as_mut(), 1, ());

        let result = Push::<i32, ()>::poll_flush(fma.as_mut(), &mut cx);
        assert!(result.is_done());

        assert!(tp.items().is_empty());
    }

    #[test]
    fn filter_map_async_pending_propagates() {
        let waker = Waker::noop();
        let mut cx = Context::from_waker(waker);

        let mut tp = TestPush::no_pend();
        let mut fma = crate::push::filter_map_async(
            |_x: i32| core::future::pending::<Option<i32>>(),
            &mut tp,
        );
        let mut fma = Pin::new(&mut fma);

        let result = Push::<i32, ()>::poll_ready(fma.as_mut(), &mut cx);
        assert!(result.is_done());

        Push::<i32, ()>::start_send(fma.as_mut(), 42, ());

        let result = Push::<i32, ()>::poll_ready(fma.as_mut(), &mut cx);
        assert!(result.is_pending());
    }
}
