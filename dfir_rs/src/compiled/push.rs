//! Push-based operator helpers, i.e. [`futures::sink::Sink`] helpers.

use std::{
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll, ready},
};

use futures::sink::Sink;
use pin_project_lite::pin_project;

pin_project! {
    /// Same as [`std::iterator::ForEach`] but as a [`Sink`].
    ///
    /// Synchronously consumes items and always returns `Poll::Ready(Ok(())`.
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
    type Error = crate::Never;

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
    /// Synchronously maps items and sends the output to the following sink.
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
        // Current iterator and the next item.
        iter_next: Option<(Iter, Out)>,
        _marker: PhantomData<fn(Item)>,
    }
}

impl<Si, Item, Iter, Out> Flatten<Si, Item, Iter, Out> {
    /// Create with following `sink`.
    pub fn new(sink: Si) -> Self {
        let (iter_next, _marker) = Default::default();
        Self {
            sink,
            iter_next,
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

        while this.iter_next.is_some() {
            // Ensure following sink is ready for `this.out`.
            ready!(this.sink.as_mut().poll_ready(cx))?; // INVARIANT: if `Poll::Pending` returned, invariant stays same

            // Send the output the next item.
            let (mut iter, next) = this.iter_next.take().unwrap();
            this.sink.as_mut().start_send(next)?;

            // Replace the iterator and next item (if any).
            *this.iter_next = iter.next().map(|next| (iter, next));
        }

        Poll::Ready(Ok(()))
    }

    fn start_send(self: Pin<&mut Self>, item: Item) -> Result<(), Self::Error> {
        let this = self.project();

        assert!(this.iter_next.is_none(), "Sink not ready.");
        let mut iter = item.into_iter();
        *this.iter_next = iter.next().map(|next| (iter, next));
        Ok(())
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        ready!(self.as_mut().poll_ready(cx)?);
        self.project().sink.poll_flush(cx)
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        ready!(self.as_mut().poll_ready(cx)?);
        self.project().sink.poll_close(cx)
    }
}

pin_project! {
    /// Same as [`std::iterator::Filter`] but as a [`Sink`].
    ///
    /// Synchronously filters items and sends the outputs to the following sink.
    #[must_use = "sinks do nothing unless polled"]
    pub struct Filter<Si, Func> {
        #[pin]
        sink: Si,
        func: Func,
    }
}

impl<Si, Func> Filter<Si, Func> {
    /// Creates with filtering `func`, following `sink`.
    pub fn new(func: Func, sink: Si) -> Self {
        Self { sink, func }
    }
}

impl<Si, Func, Item> Sink<Item> for Filter<Si, Func>
where
    Si: Sink<Item>,
    Func: FnMut(&Item) -> bool,
{
    type Error = Si::Error;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.project().sink.poll_ready(cx)
    }
    fn start_send(self: Pin<&mut Self>, item: Item) -> Result<(), Self::Error> {
        let this = self.project();
        if (this.func)(&item) {
            this.sink.start_send(item)
        } else {
            Ok(())
        }
    }
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.project().sink.poll_flush(cx)
    }
    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.project().sink.poll_close(cx)
    }
}

pin_project! {
    /// Same as [`std::iterator::FilterMap`] but as a [`Sink`].
    ///
    /// Synchronously filter-maps items and sends the outputs to the following sink.
    #[must_use = "sinks do nothing unless polled"]
    pub struct FilterMap<Si, Func> {
        #[pin]
        sink: Si,
        func: Func,
    }
}

impl<Si, Func> FilterMap<Si, Func> {
    /// Creates with mapping `func`, following `sink`.
    pub fn new(func: Func, sink: Si) -> Self {
        Self { sink, func }
    }
}

impl<Si, Func, Item, Out> Sink<Item> for FilterMap<Si, Func>
where
    Si: Sink<Out>,
    Func: FnMut(Item) -> Option<Out>,
{
    type Error = Si::Error;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.project().sink.poll_ready(cx)
    }
    fn start_send(self: Pin<&mut Self>, item: Item) -> Result<(), Self::Error> {
        let this = self.project();
        if let Some(item) = (this.func)(item) {
            this.sink.start_send(item)
        } else {
            Ok(())
        }
    }
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.project().sink.poll_flush(cx)
    }
    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.project().sink.poll_close(cx)
    }
}

pin_project! {
    /// Special sink for the `persist` operator.
    #[must_use = "sinks do nothing unless polled"]
    pub struct Persist<'ctx, Si, Item> {
        #[pin]
        sink: Si,
        replay: std::slice::Iter<'ctx, Item>,
    }
}

impl<'ctx, Si, Item> Persist<'ctx, Si, Item> {
    /// Create with the given replay and following sink.
    pub fn new(replay: std::slice::Iter<'ctx, Item>, sink: Si) -> Self {
        Self { sink, replay }
    }

    fn empty_replay(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Si::Error>>
    where
        Si: Sink<Item>,
        Item: Clone,
    {
        let mut this = self.project();
        while let Some(item) = this.replay.next() {
            ready!(this.sink.as_mut().poll_ready(cx))?;
            this.sink.as_mut().start_send(item.clone())?;
        }
        Poll::Ready(Ok(()))
    }
}

impl<'ctx, Si, Item> Sink<Item> for Persist<'ctx, Si, Item>
where
    Si: Sink<Item>,
    Item: Clone,
{
    type Error = Si::Error;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        ready!(self.as_mut().empty_replay(cx))?;
        self.project().sink.poll_ready(cx)
    }
    fn start_send(self: Pin<&mut Self>, item: Item) -> Result<(), Self::Error> {
        self.project().sink.start_send(item)
    }
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        ready!(self.as_mut().empty_replay(cx))?; // TODO(mingwei): needed?
        self.project().sink.poll_flush(cx)
    }
    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        ready!(self.as_mut().empty_replay(cx))?; // TODO(mingwei): needed?
        self.project().sink.poll_close(cx)
    }
}

// fn constrain_types<'ctx, Push, Item>(vec: &'ctx mut Vec<Item>, mut output: Push, is_new_tick: bool) -> impl 'ctx + #root::futures::sink::Sink<Item, Error = #root::Never>
// where
//     Push: 'ctx + #root::futures::sink::Sink<Item, Error = #root::Never>,
//     Item: ::std::clone::Clone,
// {
//     if is_new_tick {
//         #work_fn(|| vec.iter().cloned().for_each(|item| {
//             #root::pusherator::Pusherator::give(&mut output, item);
//         }));
//     }
//     #root::pusherator::map::Map::new(|item| {
//         vec.push(item);
//         vec.last().unwrap().clone()
//     }, output)
// }

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
