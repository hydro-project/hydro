use core::pin::Pin;
use core::task::{Context, Poll, ready};

use futures_sink::Sink;
use pin_project_lite::pin_project;

use crate::pull::{Pull, PullStep};

pin_project! {
    /// [`Future`] for pulling from a [`Pull`] and pushing to a [`Sink`].
    #[must_use = "futures do nothing unless polled"]
    #[derive(Clone, Debug)]
    pub struct SendSink<Pul, Psh> {
        #[pin]
        pull: Pul,
        #[pin]
        push: Psh,
        pull_ended: bool,
    }
}

impl<Pul, Psh> SendSink<Pul, Psh>
where
    Self: Future,
{
    /// Create a new [`SendSink`] from the given `pull` and `push` sides.
    pub(crate) const fn new(pull: Pul, push: Psh) -> Self {
        Self {
            pull,
            push,
            pull_ended: false,
        }
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
        if !*this.pull_ended {
            loop {
                ready!(this.push.as_mut().poll_ready(cx)?);
                match this
                    .pull
                    .as_mut()
                    .pull(<Pul::Ctx<'_> as crate::Context<'_>>::from_task(cx))
                {
                    PullStep::Ready(item, meta) => {
                        let _ = meta; // TODO(mingwei):
                        let () = this.push.as_mut().start_send(item)?;
                    }
                    PullStep::Pending(_) => return Poll::Pending,
                    PullStep::Ended(_) => {
                        *this.pull_ended = true;
                        break;
                    }
                }
            }
        }
        this.push.as_mut().poll_flush(cx)
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec;
    use core::task::{Poll, Waker};

    use super::SendSink;
    use crate::Yes;
    use crate::pull::test_utils::TestPull;
    use crate::push::test_utils::TestPush;
    use crate::push::{PushStep, sink_compat};

    /// SendSink must not re-poll the pull after it returned Ended,
    /// even if poll_flush returns Pending.
    #[test]
    fn send_sink_no_repoll_after_ended_on_flush_pending() {
        let pull = TestPull::items_fused(0..2);
        let mut push: TestPush<_, _, true> = TestPush::new_fused([], [PushStep::Pending(Yes)]);
        let sink = sink_compat(&mut push); // Re-use `TestPush` by converting into a sink.

        let mut send = core::pin::pin!(SendSink::new(pull, sink));

        let waker = Waker::noop();
        let mut cx = core::task::Context::from_waker(waker);

        let result = send.as_mut().poll(&mut cx);
        assert!(result.is_pending());

        let result = send.as_mut().poll(&mut cx);
        assert!(result == Poll::Ready(Ok(())));

        assert_eq!(push.items(), vec![0, 1]);
    }
}
