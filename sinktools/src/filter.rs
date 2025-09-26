use core::pin::Pin;

use pin_project_lite::pin_project;

use crate::{Sink, forward_sink};

pin_project! {
    /// Same as [`core::iterator::Filter`] but as a [`Sink`].
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
    /// Creates with filtering `func` and next `sink`.
    pub fn new(func: Func, sink: Si) -> Self {
        Self { sink, func }
    }

    /// Creates with filtering `func` and next `sink`, ensuring this implements `Sink<Item>`.
    pub fn new_sink<Item>(func: Func, sink: Si) -> Self
    where
        Self: Sink<Item>,
    {
        Self::new(func, sink)
    }
}

impl<Si, Func, Item> Sink<Item> for Filter<Si, Func>
where
    Si: Sink<Item>,
    Func: FnMut(&Item) -> bool,
{
    type Error = Si::Error;

    fn start_send(self: Pin<&mut Self>, item: Item) -> Result<(), Self::Error> {
        let this = self.project();
        if (this.func)(&item) {
            this.sink.start_send(item)
        } else {
            Ok(())
        }
    }

    forward_sink!(poll_ready, poll_flush, poll_close);
}
