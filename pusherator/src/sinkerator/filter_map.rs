use std::pin::Pin;
use std::task::{Context, Poll};

use pin_project_lite::pin_project;

use super::Sinkerator;

pin_project! {
    /// An [`Sinkerator`] which maps each item using `Func` and sends to output to the next sink if it was `Some`.
    pub struct FilterMap<Si, Func> {
        #[pin]
        si: Si,
        func: Func,
    }
}

impl<Si, Func> FilterMap<Si, Func> {
    /// Creates a new [`FilterMap`], which maps each item using `func` and sends the output `si` if it was `Some`.
    pub fn new<Item>(func: Func, si: Si) -> Self
    where
        Self: Sinkerator<Item>,
    {
        Self { si, func }
    }
}

impl<Si, Func, Item, Out> Sinkerator<Item> for FilterMap<Si, Func>
where
    Si: Sinkerator<Out>,
    Func: FnMut(Item) -> Option<Out>,
{
    type Error = Si::Error;

    fn poll_send(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        item: Option<Item>,
    ) -> Poll<Result<(), Self::Error>> {
        let mut this = self.project();

        if let Some(item) = item {
            if let Some(out) = (this.func)(item) {
                this.si.as_mut().poll_send(cx, Some(out))
            } else {
                Poll::Ready(Ok(()))
            }
        } else {
            this.si.as_mut().poll_send(cx, None)
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.project().si.poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.project().si.poll_close(cx)
    }
}
