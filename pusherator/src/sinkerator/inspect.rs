use std::pin::Pin;
use std::task::{Context, Poll};

use pin_project_lite::pin_project;

use super::Sinkerator;

pin_project! {
    /// An [`Sinkerator`] which will inspect items using `Func` before sending them to `Si`.
    pub struct Inspect<Si, Func> {
        #[pin]
        si: Si,
        func: Func,
    }
}

impl<Si, Func> Inspect<Si, Func> {
    /// Creates a new [`Inspect`], which will inspect items using `func` before sending them to `si`.
    pub fn new<Item>(func: Func, si: Si) -> Self
    where
        Self: Sinkerator<Item>,
    {
        Self { si, func }
    }
}

impl<Si, Func, Item> Sinkerator<Item> for Inspect<Si, Func>
where
    Si: Sinkerator<Item>,
    Func: FnMut(&Item),
{
    type Error = Si::Error;

    fn poll_send(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        item: Option<Item>,
    ) -> Poll<Result<(), Self::Error>> {
        let this = self.project();
        this.si.poll_send(cx, item.inspect(this.func))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.project().si.poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.project().si.poll_close(cx)
    }
}
