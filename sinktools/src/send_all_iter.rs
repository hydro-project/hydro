use core::pin::Pin;
use core::task::{Context, Poll, ready};

use pin_project_lite::pin_project;

use crate::Sink;

pin_project! {
    /// [`Future`] for pulling from an [`Iterator`] and pushing to a [`Sink`].
    pub struct SendAllIter<Pull, Push> {
        pull: Pull,
        #[pin]
        push: Push,
    }
}
impl<Pull, Push> SendAllIter<Pull, Push> {
    /// Create a new [`SendAllIter`] from the given `pull` and `push` sides.
    pub fn new(pull: Pull, push: Push) -> Self
    where
        Self: Future,
    {
        Self { pull, push }
    }
}
impl<Pull, Push> Future for SendAllIter<Pull, Push>
where
    Pull: Iterator,
    Push: Sink<Pull::Item>,
{
    type Output = Result<(), Push::Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();

        loop {
            ready!(this.push.as_mut().poll_ready(cx)?);
            if let Some(item) = this.pull.next() {
                let () = this.push.as_mut().start_send(item)?;
            } else {
                break;
            }
        }
        this.push.as_mut().poll_flush(cx)
    }
}
