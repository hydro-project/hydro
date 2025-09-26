use std::pin::Pin;
use std::task::{Context, Poll, ready};

use pin_project_lite::pin_project;

use super::Sinkerator;

pin_project! {
    /// [`Future`] for pulling from an [`Iterator`] and pushing to a [`Sinkerator`].
    pub struct Pivot<Pull, Push> {
        pull: Pull,
        #[pin]
        push: Push,
        ready: bool,
    }
}

impl<Pull, Push> Pivot<Pull, Push> {
    /// Create a new [`Pivot`] from the given `pull` and `push` sides.
    pub fn new(pull: Pull, push: Push) -> Self
    where
        Self: Future,
    {
        Self {
            pull,
            push,
            ready: false,
        }
    }
}

impl<Pull, Push> Future for Pivot<Pull, Push>
where
    Pull: Iterator,
    Push: Sinkerator<Pull::Item>,
{
    type Output = Result<(), Push::Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();

        if !*this.ready {
            ready!(this.push.as_mut().poll_send(cx, None)?);
            *this.ready = true;
        }

        for item in this.pull {
            if let Poll::Ready(result) = this.push.as_mut().poll_send(cx, Some(item)) {
                let () = result?;
            } else {
                *this.ready = false;
                return Poll::Pending;
            }
        }
        this.push.as_mut().poll_flush(cx)
    }
}
