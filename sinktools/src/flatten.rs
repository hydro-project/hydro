use core::pin::Pin;
use core::task::{Context, Poll, ready};

use pin_project_lite::pin_project;

use crate::Sink;

pin_project! {
    /// Same as [`std::iterator::Flatten`] but as a [`Sink`].
    ///
    /// Synchronously flattens items and sends the outputs to the following sink.
    #[must_use = "sinks do nothing unless polled"]
    pub struct Flatten<Si, Iter, Out> {
        #[pin]
        sink: Si,
        // Current iterator and the next item.
        iter_next: Option<(Iter, Out)>,
        // _marker: PhantomData<fn(Item)>,
    }
}

impl<Si, Iter, Out> Flatten<Si, Iter, Out> {
    /// Create with following `sink`.
    pub fn new(sink: Si) -> Self {
        Self {
            sink,
            iter_next: None,
        }
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

            // Send the output the next item.
            let (mut iter, next) = this.iter_next.take().unwrap();
            this.sink.as_mut().start_send(next)?;

            // Replace the iterator and next item (if any).
            *this.iter_next = iter.next().map(|next| (iter, next));
        }

        Poll::Ready(Ok(()))
    }
}

impl<Si, Item> Sink<Item> for Flatten<Si, Item::IntoIter, Item::Item>
where
    Si: Sink<Item::Item>,
    Item: IntoIterator,
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
        let mut iter = item.into_iter();
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

#[cfg(test)]
mod tests {
    use futures_util::stream::StreamExt;
    use tokio::sync::mpsc::channel;
    use tokio_stream::wrappers::ReceiverStream;
    use tokio_util::sync::PollSender;

    use super::*;
    use crate::sink::SinkExt;

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
