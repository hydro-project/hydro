use core::pin::Pin;
use core::task::{Context, Poll};

use pin_project_lite::pin_project;

use crate::Sink;

pin_project! {
    /// Same as [`core::iterator::Inspect`] but as a [`Sink`].
    ///
    /// Synchronously inspects items before sending them to the following sink.
    #[must_use = "sinks do nothing unless polled"]
    pub struct Inspect<Si, Func> {
        #[pin]
        sink: Si,
        func: Func,
    }
}

impl<Si, Func> Inspect<Si, Func> {
    /// Creates with inspecting `func` and next `sink`.
    pub fn new(func: Func, sink: Si) -> Self {
        Self { sink, func }
    }

    /// Creates with inspecting `func` and next `sink`, ensuring this implements `Sink<Item>`.
    pub fn new_sink<Item>(func: Func, sink: Si) -> Self
    where
        Self: Sink<Item>,
    {
        Self::new(func, sink)
    }
}

impl<Si, Func, Item> Sink<Item> for Inspect<Si, Func>
where
    Si: Sink<Item>,
    Func: FnMut(&Item),
{
    type Error = Si::Error;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.project().sink.poll_ready(cx)
    }
    fn start_send(self: Pin<&mut Self>, item: Item) -> Result<(), Self::Error> {
        let this = self.project();
        (this.func)(&item);
        this.sink.start_send(item)
    }
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.project().sink.poll_flush(cx)
    }
    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.project().sink.poll_close(cx)
    }
}
