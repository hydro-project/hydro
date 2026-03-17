use core::pin::Pin;
use core::task::{Context, Poll, ready};

use futures_sink::Sink;
use pin_project_lite::pin_project;

use crate::{Pull, Step};

pin_project! {
    /// [`Future`] for pulling from a [`Pull`] and pushing to a [`Sink`].
    #[must_use = "futures do nothing unless polled"]
    #[derive(Clone, Debug)]
    pub struct SendSink<Pul, Psh> {
        #[pin]
        pull: Pul,
        #[pin]
        push: Psh,
    }
}

impl<Pul, Psh> SendSink<Pul, Psh>
where
    Self: Future,
{
    /// Create a new [`SendSink`] from the given `pull` and `push` sides.
    pub(crate) const fn new(pull: Pul, push: Psh) -> Self {
        Self { pull, push }
    }
}

impl<Pul, Psh, Item> Future for SendSink<Pul, Psh>
where
    Pul: Pull<Item = Item>,
    Psh: Sink<Item>,
    for<'ctx> Pul::Ctx<'ctx>: crate::Context<'ctx>,
{
    type Output = Result<(), Psh::Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();
        loop {
            ready!(this.push.as_mut().poll_ready(cx)?);
            match this
                .pull
                .as_mut()
                .pull(<Pul::Ctx<'_> as crate::Context<'_>>::from_task(cx))
            {
                Step::Ready(item, meta) => {
                    let _ = meta; // TODO(mingwei):
                    let () = this.push.as_mut().start_send(item)?;
                }
                Step::Pending(_) => return Poll::Pending,
                Step::Ended(_) => break,
            }
        }
        this.push.as_mut().poll_flush(cx)
    }
}
