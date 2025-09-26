use std::pin::Pin;
use std::task::{Context, Poll};

use pin_project_lite::pin_project;

use super::Sinkerator;

pin_project! {
    /// An [`Sinkerator`] which maps items using `Func` before sending them to the sink.
    pub struct Map<Si, Func> {
        #[pin]
        si: Si,
        func: Func,
    }
}

impl<Si, Func> Map<Si, Func> {
    /// Creates a new [`Map`], which will map items using `func` before sending them to `si`.
    pub fn new<Item>(func: Func, si: Si) -> Self
    where
        Self: Sinkerator<Item>,
    {
        Self { si, func }
    }
}

impl<Si, Func, Item, Out> Sinkerator<Item> for Map<Si, Func>
where
    Si: Sinkerator<Out>,
    Func: FnMut(Item) -> Out,
{
    type Error = Si::Error;

    fn poll_send(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        item: Option<Item>,
    ) -> Poll<Result<(), Self::Error>> {
        let this = self.project();
        this.si.poll_send(cx, item.map(this.func))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.project().si.poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.project().si.poll_close(cx)
    }
}
