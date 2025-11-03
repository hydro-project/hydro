//! [`SendStream`] and related items.
use core::pin::Pin;
use core::task::{Context, Poll, ready};

use futures_util::Stream;
use pin_project_lite::pin_project;

use crate::{Sink, SinkBuild};

pin_project! {
    /// [`Future`] for pulling from an [`Iterator`] and pushing to a [`Sink`].
    #[must_use = "futures do nothing unless polled"]
    pub struct SendStream<Pull, Push> {
        #[pin]
        pull: Pull,
        #[pin]
        push: Push,
    }
}
impl<Pull, Push> SendStream<Pull, Push>
where
    Self: Future,
{
    /// Create a new [`SendStream`] from the given `pull` and `push` sides.
    pub fn new(pull: Pull, push: Push) -> Self {
        Self { pull, push }
    }
}
impl<Pull, Push> Future for SendStream<Pull, Push>
where
    Pull: Stream,
    Push: Sink<Pull::Item>,
{
    type Output = Result<(), Push::Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();

        loop {
            ready!(this.push.as_mut().poll_ready(cx)?);
            if let Some(item) = ready!(this.pull.as_mut().poll_next(cx)) {
                let () = this.push.as_mut().start_send(item)?;
            } else {
                break;
            }
        }
        this.push.as_mut().poll_flush(cx)
    }
}

/// [`SinkBuild`] for [`SendStream`].
pub struct SendStreamBuild<St> {
    pub(crate) stream: St,
}
impl<St> SinkBuild for SendStreamBuild<St>
where
    St: Stream,
{
    type Item = St::Item;

    type Output<Next: Sink<Self::Item>> = SendStream<St, Next>;
    fn send_to<Next>(self, next: Next) -> Self::Output<Next>
    where
        Next: Sink<Self::Item>,
    {
        SendStream::new(self.stream, next)
    }
}
