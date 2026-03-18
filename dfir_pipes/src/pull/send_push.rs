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
        pull_ended: bool,
    }
}

impl<Pul, Psh> SendPush<Pul, Psh>
where
    Self: Future,
{
    /// Create a new [`SendPush`] from the given `pull` and `push` sides.
    pub(crate) const fn new(pull: Pul, push: Psh) -> Self {
        Self {
            pull,
            push,
            pull_ended: false,
        }
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
        if !*this.pull_ended {
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
                    PullStep::Ended(_) => {
                        *this.pull_ended = true;
                        break;
                    }
                }
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

#[cfg(test)]
mod tests {
    use core::task::Waker;

    extern crate alloc;
    use alloc::vec;
    use alloc::vec::Vec;

    use super::SendPush;
    use crate::pull::test_utils::TestPull;
    use crate::push::test_utils::PendingFlushPush;

    /// SendPush must not re-poll the pull after it returned Ended,
    /// even if poll_flush returns Pending.
    #[test]
    fn send_push_no_repoll_after_ended_on_flush_pending() {
        let pull = TestPull::items(0..2);
        let push = PendingFlushPush {
            items: Vec::new(),
            flush_pending_count: 1,
        };
        let mut send = core::pin::pin!(SendPush::new(pull, push));

        let waker = Waker::noop();
        let mut cx = core::task::Context::from_waker(waker);

        let result = send.as_mut().poll(&mut cx);
        assert!(result.is_pending(), "expected Pending from first poll");

        let result = send.as_mut().poll(&mut cx);
        assert!(result.is_ready(), "expected Ready from second poll");

        let items = &send.into_ref().get_ref().push.items;
        assert_eq!(*items, vec![0, 1]);
    }
}
