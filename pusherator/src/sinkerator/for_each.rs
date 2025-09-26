use std::pin::Pin;
use std::task::{Context, Poll};

use pin_project_lite::pin_project;

use super::Sinkerator;

pin_project! {
    /// An [`Sinkerator`] which synchronously consumes items using `Func`.
    pub struct ForEach<Func> {
        func: Func,
    }
}

impl<Func> ForEach<Func> {
    /// Creates a new [`ForEach`], which will synchronously consume items using `func`.
    pub fn new<Item>(func: Func) -> Self
    where
        Self: Sinkerator<Item>,
    {
        Self { func }
    }
}

impl<Func, Item> Sinkerator<Item> for ForEach<Func>
where
    Func: FnMut(Item),
{
    type Error = futures::never::Never;

    fn poll_send(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        item: Option<Item>,
    ) -> Poll<Result<(), Self::Error>> {
        if let Some(item) = item {
            (self.project().func)(item);
        }
        Poll::Ready(Ok(()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}
