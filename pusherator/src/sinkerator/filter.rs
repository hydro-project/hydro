use std::pin::Pin;
use std::task::{Context, Poll};

use pin_project_lite::pin_project;

use super::Sinkerator;

pin_project! {
    /// An [`Sinkerator`] which will filter items using `Func` to determine if to send them to `Si`.
    pub struct Filter<Si, Func> {
        #[pin]
        si: Si,
        func: Func,
    }
}

impl<Si, Func> Filter<Si, Func> {
    /// Creates a new [`Filter`], which will filter items using `func` to determine if to send them to `si`.
    pub fn new<Item>(func: Func, si: Si) -> Self
    where
        Self: Sinkerator<Item>,
    {
        Self { si, func }
    }
}

impl<Si, Func, Item> Sinkerator<Item> for Filter<Si, Func>
where
    Si: Sinkerator<Item>,
    Func: FnMut(&Item) -> bool,
{
    type Error = Si::Error;

    fn poll_send(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        item: Option<Item>,
    ) -> Poll<Result<(), Self::Error>> {
        let this = self.project();
        if let Some(item) = item {
            if (this.func)(&item) {
                this.si.poll_send(cx, Some(item))
            } else {
                Poll::Ready(Ok(()))
            }
        } else {
            this.si.poll_send(cx, None)
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.project().si.poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.project().si.poll_close(cx)
    }
}
