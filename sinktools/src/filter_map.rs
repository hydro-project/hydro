//! [`FilterMap`] and related items.
use core::pin::Pin;

use pin_project_lite::pin_project;

use crate::{Sink, SinkBuild, forward_sink};

pin_project! {
    /// Same as [`core::iterator::FilterMap`] but as a [`Sink`].
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
    /// Creates with mapping `func` and next `sink`.
    pub fn new<Item>(func: Func, sink: Si) -> Self
    where
        Self: Sink<Item>,
    {
        Self { sink, func }
    }
}

impl<Si, Func, Item, Out> Sink<Item> for FilterMap<Si, Func>
where
    Si: Sink<Out>,
    Func: FnMut(Item) -> Option<Out>,
{
    type Error = Si::Error;

    fn start_send(self: Pin<&mut Self>, item: Item) -> Result<(), Self::Error> {
        let this = self.project();
        if let Some(item) = (this.func)(item) {
            this.sink.start_send(item)
        } else {
            Ok(())
        }
    }

    forward_sink!(poll_ready, poll_flush, poll_close);
}

/// [`SinkBuild`] for [`FilterMap`].
pub struct FilterMapBuilder<Prev, Func> {
    pub(crate) prev: Prev,
    pub(crate) func: Func,
}
impl<Prev, ItemOut, Func> SinkBuild for FilterMapBuilder<Prev, Func>
where
    Prev: SinkBuild,
    Func: FnMut(Prev::Item) -> Option<ItemOut>,
{
    type Item = ItemOut;

    type Output<Next: Sink<ItemOut>> = Prev::Output<FilterMap<Next, Func>>;

    fn send_to<Next>(self, next: Next) -> Self::Output<Next>
    where
        Next: Sink<ItemOut>,
    {
        self.prev.send_to(FilterMap::new(self.func, next))
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
    async fn test_filter_map() {
        let (out_send, out_recv) = channel(1);
        let out_send = PollSender::new(out_send);
        let mut out_recv = ReceiverStream::new(out_recv);

        let mut sink = FilterMap::new(core::convert::identity, out_send);

        let a = tokio::task::spawn(async move {
            sink.send(Some(0)).await.unwrap();
            sink.send(Some(1)).await.unwrap();
            sink.send(None).await.unwrap();
            sink.send(Some(2)).await.unwrap();
            sink.send(None).await.unwrap();
            sink.send(None).await.unwrap();
            sink.send(Some(3)).await.unwrap();
            sink.send(None).await.unwrap();
            sink.send(None).await.unwrap();
            sink.send(None).await.unwrap();
            sink.send(Some(4)).await.unwrap();
            sink.send(Some(5)).await.unwrap();
            sink.send(None).await.unwrap();
        });
        println!("{}", line!());
        assert_eq!(
            &[0, 1, 2, 3, 4, 5],
            &*out_recv.by_ref().collect::<Vec<_>>().await
        );
        println!("{}", line!());
        a.await.unwrap();
    }
}
