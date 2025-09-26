use core::pin::Pin;

use pin_project_lite::pin_project;

use crate::{Sink, forward_sink};

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
    pub fn new<Item>(func: Func, sink: Si) -> Self
    where
        Self: Sink<Item>,
    {
        Self { sink, func }
    }
}

impl<Si, Func, Item, ItemOut> Sink<Item> for Map<Si, Func>
where
    Si: Sink<ItemOut>,
    Func: FnMut(Item) -> ItemOut,
{
    type Error = Si::Error;

    fn start_send(self: Pin<&mut Self>, item: Item) -> Result<(), Self::Error> {
        let this = self.project();
        let item = (this.func)(item);
        this.sink.start_send(item)
    }

    forward_sink!(poll_ready, poll_flush, poll_close);
}
