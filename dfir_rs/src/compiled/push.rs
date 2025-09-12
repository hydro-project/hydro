//! Push-based operator helpers, i.e. [`futures::sink::Sink`] helpers.

use std::{
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll, ready},
};

use futures::{never::Never, sink::Sink};
use pin_project_lite::pin_project;

pin_project! {
    /// Same as [`std::iterator::ForEach`] but as a [`Sink`].
    ///
    /// Synchronously consumes items using `f` and always returns `Poll::Ready(Ok(())`.
    #[must_use = "sinks do nothing unless polled"]
    pub struct ForEach<Func> {
        func: Func,
    }
}
impl<Func> ForEach<Func> {
    /// Create with consuming `func`.
    pub fn new(func: Func) -> Self {
        Self { func }
    }
}
impl<Func, Item> Sink<Item> for ForEach<Func>
where
    Func: FnMut(Item),
{
    type Error = Never;

    fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
    fn start_send(self: Pin<&mut Self>, item: Item) -> Result<(), Self::Error> {
        (self.project().func)(item);
        Ok(())
    }
    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}

pin_project! {
    /// Same as [`std::iterator::Map`] but as a [`Sink`].
    ///
    /// Synchronously maps items using `f` and sends the output to the following sink.
    #[must_use = "sinks do nothing unless polled"]
    pub struct Map<Si, Func> {
        #[pin]
        sink: Si,
        func: Func,
    }
}

impl<Si, Func> Map<Si, Func> {
    /// Creates with mapping `func`, following `sink`.
    pub fn new(func: Func, sink: Si) -> Self {
        Self { sink, func }
    }
}

impl<Si, Func, Item, Out> Sink<Item> for Map<Si, Func>
where
    Si: Sink<Out>,
    Func: FnMut(Item) -> Out,
{
    type Error = Si::Error;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.project().sink.poll_ready(cx)
    }
    fn start_send(self: Pin<&mut Self>, item: Item) -> Result<(), Self::Error> {
        let this = self.project();
        let item = (this.func)(item);
        this.sink.start_send(item)
    }
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.project().sink.poll_flush(cx)
    }
    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.project().sink.poll_close(cx)
    }
}

pin_project! {
    /// Same as [`std::iterator::Flatten`] but as a [`Sink`].
    ///
    /// Synchronously flattens items and sends the outputs to the following sink.
    #[must_use = "sinks do nothing unless polled"]
    pub struct Flatten<Si, Item, Iter, Out> {
        #[pin]
        sink: Si,
        // INVARIANT: `iter` is some IFF `out` is some.
        iter: Option<Iter>,
        out: Option<Out>,
        _marker: PhantomData<fn(Item)>,
    }
}

impl<Si, Item, Iter, Out> Flatten<Si, Item, Iter, Out> {
    /// Create with following `sink`.
    pub fn new(sink: Si) -> Self {
        let (iter, out, _marker) = Default::default();
        Self {
            sink,
            iter,
            out,
            _marker,
        }
    }
}

impl<Si, Item, Iter, Out> Sink<Item> for Flatten<Si, Item, Iter, Out>
where
    Si: Sink<Out>,
    Item: IntoIterator<IntoIter = Iter, Item = Out>,
    Item::IntoIter: Iterator<Item = Out>,
{
    type Error = Si::Error;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let mut this = self.project();
        debug_assert_eq!(this.iter.is_some(), this.out.is_some(), "INVARIANT");

        while this.out.is_some() {
            // Ensure following sink is ready for `this.out`.
            ready!(this.sink.as_mut().poll_ready(cx))?; // INVARIANT: if `Poll::Pending` returned, invariant stays same
            // Send the output item `this.out`.
            this.sink.as_mut().start_send(this.out.take().unwrap())?;

            // Repopulate `this.out` using `this.iter`
            *this.out = this.iter.as_mut().unwrap().next();
        }
        // INVARIANT: `this.iter` is now empty; set `this.out` to none.
        *this.iter = None;

        Poll::Ready(Ok(()))
    }

    fn start_send(self: Pin<&mut Self>, item: Item) -> Result<(), Self::Error> {
        let this = self.project();
        debug_assert_eq!(this.iter.is_some(), this.out.is_some(), "INVARIANT");

        assert!(this.iter.is_none(), "Sink not ready.");
        let mut iter = item.into_iter();
        *this.out = iter.next();
        if this.out.is_some() {
            // INVARIANT: `this.out` is now some; set `this.iter` to some.
            *this.iter = Some(iter);
        }
        Ok(())
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        ready!(self.as_mut().poll_ready(cx)?);
        self.project().sink.poll_flush(cx)
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        ready!(self.as_mut().poll_ready(cx)?);
        self.project().sink.poll_flush(cx)
    }
}

#[cfg(test)]
mod tests {
    use futures::sink::SinkExt;
    use futures::stream::StreamExt;
    use tokio::sync::mpsc::channel;
    use tokio_stream::wrappers::ReceiverStream;
    use tokio_util::sync::PollSender;

    use super::*;

    #[tokio::test]
    async fn test_flatten() {
        let (out_send, out_recv) = channel(2);
        let out_send = PollSender::new(out_send);
        let mut out_recv = ReceiverStream::new(out_recv);

        let mut sink = Flatten::new(out_send);

        let a = tokio::task::spawn(async move {
            sink.send(vec![0, 1, 2]).await.unwrap();
            println!("{}", line!());
            sink.send(vec![3, 4, 5]).await.unwrap();
            println!("{}", line!());
            sink.send(vec![6, 7, 8]).await.unwrap();
            println!("{}", line!());
            sink.send(vec![9]).await.unwrap();
        });
        println!("{}", line!());
        assert_eq!(
            &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
            &*out_recv.by_ref().collect::<Vec<_>>().await
        );
        println!("{}", line!());
        a.await.unwrap();
    }
}
