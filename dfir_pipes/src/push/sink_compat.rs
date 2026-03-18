//! [`SinkCompat`] adapter wrapping a [`Push`] into a [`futures_sink::Sink`].
use core::pin::Pin;
use core::task::Poll;

use futures_sink::Sink;
use pin_project_lite::pin_project;

use super::PushStep;
use crate::Context;
use crate::push::Push;

pin_project! {
    /// Adapter that wraps a [`Push`] to implement the [`Sink`] trait.
    #[must_use = "`Sink`s do nothing unless polled"]
    pub struct SinkCompat<Psh> {
        #[pin]
        push: Psh,
    }
}

impl<Psh> SinkCompat<Psh> {
    /// Creates a new [`SinkCompat`] wrapping the given [`Push`].
    pub(crate) const fn new(push: Psh) -> Self {
        Self { push }
    }

    /// Returns the wrapped [`Push`].
    pub fn into_inner(self) -> Psh {
        self.push
    }

    /// Returns a pinned mutable reference to the wrapped [`Push`].
    pub fn as_pin_mut(self: Pin<&mut Self>) -> Pin<&mut Psh> {
        self.project().push
    }

    /// Returns a pinned reference to the wrapped [`Push`].
    pub fn as_pin_ref(self: Pin<&Self>) -> Pin<&Psh> {
        self.project_ref().push
    }
}

impl<Psh> AsMut<Psh> for SinkCompat<Psh> {
    fn as_mut(&mut self) -> &mut Psh {
        &mut self.push
    }
}

impl<Psh> AsRef<Psh> for SinkCompat<Psh> {
    fn as_ref(&self) -> &Psh {
        &self.push
    }
}

impl<Psh, Item> Sink<Item> for SinkCompat<Psh>
where
    Psh: Push<Item, ()>,
{
    type Error = core::convert::Infallible;

    fn poll_ready(
        self: Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        match self.as_pin_mut().poll_ready(Context::from_task(cx)) {
            PushStep::Pending(_) => Poll::Pending,
            PushStep::Done => Poll::Ready(Ok(())),
        }
    }

    fn start_send(self: Pin<&mut Self>, item: Item) -> Result<(), Self::Error> {
        self.as_pin_mut().start_send(item, ());
        Ok(())
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        match self.as_pin_mut().poll_flush(Context::from_task(cx)) {
            PushStep::Pending(_) => Poll::Pending,
            PushStep::Done => Poll::Ready(Ok(())),
        }
    }

    fn poll_close(
        self: Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        self.poll_flush(cx)
    }
}
