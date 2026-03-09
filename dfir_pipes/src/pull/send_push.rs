use core::pin::Pin;
use core::task::Poll;

use pin_project_lite::pin_project;

use crate::Context;
use crate::pull::{Pull, PullStep};
use crate::push::{Push, PushStep};

pin_project! {
    /// [`Future`] for pulling from a [`Pull`] and pushing to a [`Push`].
    #[must_use = "futures do nothing unless polled"]
    #[derive(Clone, Debug)]
    pub struct SendPush<Pul, Psh> {
        #[pin]
        pull: Pul,
        #[pin]
        push: Psh,
    }
}

impl<Pul, Psh> SendPush<Pul, Psh>
where
    Self: Future,
{
    /// Create a new [`SendPush`] from the given `pull` and `push` sides.
    pub(crate) const fn new(pull: Pul, push: Psh) -> Self {
        Self { pull, push }
    }
}

impl<Pul, Psh, Item, Meta> Future for SendPush<Pul, Psh>
where
    Pul: Pull<Item = Item, Meta = Meta>,
    Meta: Copy,
    Psh: Push<Item, Meta>,
    for<'ctx> Pul::Ctx<'ctx>: Context<'ctx>,
    for<'ctx> Psh::Ctx<'ctx>: Context<'ctx>,
{
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut core::task::Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();
        loop {
            // Ensure push is ready before pulling.
            match this
                .push
                .as_mut()
                .poll_ready(<Psh::Ctx<'_> as Context<'_>>::from_task(cx))
            {
                PushStep::Done => {}
                PushStep::Pending(_) => return Poll::Pending,
            }

            match this
                .pull
                .as_mut()
                .pull(<Pul::Ctx<'_> as Context<'_>>::from_task(cx))
            {
                PullStep::Ready(item, meta) => {
                    this.push.as_mut().start_send(item, meta);
                }
                PullStep::Pending(_) => return Poll::Pending,
                PullStep::Ended(_) => break,
            }
        }
        match this
            .push
            .as_mut()
            .poll_flush(<Psh::Ctx<'_> as Context<'_>>::from_task(cx))
        {
            PushStep::Done => Poll::Ready(()),
            PushStep::Pending(_) => Poll::Pending,
        }
    }
}
