use std::pin::Pin;
use std::task::{Context, Poll, ready};

use pin_project_lite::pin_project;

use super::Sinkerator;

pin_project! {
    /// An [`Sinkerator`] which maps each item using `Func` and flattens the iterable output to send to the sink.
    pub struct FlatMap<Si, Func, Iter> {
        #[pin]
        si: Si,
        func: Func,
        iter: Option<Iter>,
    }
}

impl<Si, Func, Iter> FlatMap<Si, Func, Iter> {
    /// Creates a new [`FlatMap`], which maps each item using `func` and flattens the iterable output to send to the `si`.
    pub fn new<Item>(func: Func, si: Si) -> Self
    where
        Self: Sinkerator<Item>,
    {
        Self {
            si,
            func,
            iter: None,
        }
    }
}

impl<Si, Func, Item, Out> Sinkerator<Item> for FlatMap<Si, Func, Out::IntoIter>
where
    Si: Sinkerator<Out::Item>,
    Func: FnMut(Item) -> Out,
    Out: IntoIterator,
{
    type Error = Si::Error;

    fn poll_send(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        item: Option<Item>,
    ) -> Poll<Result<(), Self::Error>> {
        let mut this = self.project();

        if let Some(item) = item {
            debug_assert!(
                this.iter.is_none(),
                "Sinkerator not ready: `poll_send` must return `Ready` before an item may be sent."
            );
            *this.iter = Some((this.func)(item).into_iter());
        } else {
            ready!(this.si.as_mut().poll_send(cx, None)?);
        }

        if let Some(iter) = this.iter.as_mut() {
            for item in iter {
                ready!(this.si.as_mut().poll_send(cx, Some(item))?);
            }
        }

        Poll::Ready(Ok(()))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let this = self.project();
        debug_assert!(
            this.iter.is_none(),
            "Sinkerator not ready: `poll_send` must return `Ready` before flushing."
        );
        this.si.poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let this = self.project();
        debug_assert!(
            this.iter.is_none(),
            "Sinkerator not ready: `poll_send` must return `Ready` before closing."
        );
        this.si.poll_close(cx)
    }
}
