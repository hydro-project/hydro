//! [`TryForEach`] consuming sink.
use core::pin::Pin;
use core::task::{Context, Poll};

use pin_project_lite::pin_project;

use crate::Sink;

pin_project! {
    /// Same as [`crate::ForEach`] but the closure returns `Result<(), Error>` instead of `()`.
    ///
    /// This is useful when you want to handle errors in the closure.
    ///
    /// Synchronously consumes items and always returns `Poll::Ready(Ok(())`.
    #[must_use = "sinks do nothing unless polled"]
    pub struct TryForEach<Func> {
        func: Func,
    }
}
impl<Func> TryForEach<Func> {
    /// Create with consuming `func`.
    pub fn new<Item>(func: Func) -> Self
    where
        Self: Sink<Item>,
    {
        Self { func }
    }
}
impl<Func, Item, Error> Sink<Item> for TryForEach<Func>
where
    Func: FnMut(Item) -> Result<(), Error>,
{
    type Error = Error;

    fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(self: Pin<&mut Self>, item: Item) -> Result<(), Self::Error> {
        (self.project().func)(item)
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}
