use std::pin::Pin;
use std::task::{Context, Poll};

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
    pub fn new<Item>(func: Func) -> Self
    where
        Self: Sink<Item>,
    {
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
