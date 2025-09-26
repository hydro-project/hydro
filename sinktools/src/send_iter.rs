//! [`SendIter`] and related items.
use core::pin::Pin;
use core::task::{Context, Poll, ready};

use pin_project_lite::pin_project;

use crate::{Sink, SinkBuild};

pin_project! {
    /// [`Future`] for pulling from an [`Iterator`] and pushing to a [`Sink`].
    #[must_use = "futures do nothing unless polled"]
    pub struct SendIter<Pull, Push> {
        pull: Pull,
        #[pin]
        push: Push,
    }
}
impl<Pull, Push> SendIter<Pull, Push>
where
    Self: Future,
{
    /// Create a new [`SendIter`] from the given `pull` and `push` sides.
    pub fn new(pull: Pull, push: Push) -> Self {
        Self { pull, push }
    }
}
impl<Pull, Push> Future for SendIter<Pull, Push>
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

/// [`SinkBuild`] for [`SendIter`]s.
pub struct SendIterBuild<Iter> {
    pub(crate) iter: Iter,
}
impl<Iter> SinkBuild for SendIterBuild<Iter>
where
    Iter: Iterator,
{
    type Item = Iter::Item;

    type Output<Next: Sink<Self::Item>> = SendIter<Iter, Next>;
    fn send_to<Next>(self, next: Next) -> Self::Output<Next>
    where
        Next: Sink<Self::Item>,
    {
        SendIter::new(self.iter, next)
    }
}
