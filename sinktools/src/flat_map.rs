use core::pin::Pin;
use core::task::{Context, Poll, ready};

use pin_project_lite::pin_project;

use crate::Sink;

pin_project! {
    /// Same as [`core::iterator::FlatMap`] but as a [`Sink`].
    ///
    /// Synchronously maps and flattens items, and sends the outputs to the following sink.
    #[must_use = "sinks do nothing unless polled"]
    pub struct FlatMap<Si, Func, Iter, Out> {
        #[pin]
        sink: Si,
        func: Func,
        // Current iterator and the next item.
        iter_next: Option<(Iter, Out)>,
    }
}

impl<Si, Func, Iter, Out> FlatMap<Si, Func, Iter, Out> {
    /// Create with flat-mapping function `func` and next `sink`.
    pub fn new(func: Func, sink: Si) -> Self {
        Self {
            sink,
            func,
            iter_next: None,
        }
    }

    /// Create with flat-mapping function `func` and next `sink`, ensuring this implements `Sink<Item>`.
    pub fn new_sink<Item>(func: Func, sink: Si) -> Self
    where
        Self: Sink<Item>,
    {
        Self::new(func, sink)
    }

    fn poll_ready_impl(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Si::Error>>
    where
        Si: Sink<Out>,
        Iter: Iterator<Item = Out>,
    {
        let mut this = self.project();

        while this.iter_next.is_some() {
            // Ensure following sink is ready.
            ready!(this.sink.as_mut().poll_ready(cx))?;

            // Send the item.
            let (mut iter, next) = this.iter_next.take().unwrap();
            this.sink.as_mut().start_send(next)?;

            // Replace the iterator and next item (if any).
            *this.iter_next = iter.next().map(|next| (iter, next));
        }

        Poll::Ready(Ok(()))
    }
}

impl<Si, Func, Item, IntoIter> Sink<Item> for FlatMap<Si, Func, IntoIter::IntoIter, IntoIter::Item>
where
    Si: Sink<IntoIter::Item>,
    Func: FnMut(Item) -> IntoIter,
    IntoIter: IntoIterator,
{
    type Error = Si::Error;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.poll_ready_impl(cx)
    }

    fn start_send(self: Pin<&mut Self>, item: Item) -> Result<(), Self::Error> {
        let this = self.project();

        assert!(
            this.iter_next.is_none(),
            "Sink not ready: `poll_ready` must be called and return `Ready` before `start_send` is called."
        );
        let mut iter = (this.func)(item).into_iter();
        *this.iter_next = iter.next().map(|next| (iter, next));
        Ok(())
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        ready!(self.as_mut().poll_ready_impl(cx)?);
        self.project().sink.poll_flush(cx)
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        ready!(self.as_mut().poll_ready_impl(cx)?);
        self.project().sink.poll_close(cx)
    }
}
