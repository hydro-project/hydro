//! [`FlattenStream`] pull combinator.
use core::pin::Pin;
use core::task::Context;

use futures_core::Stream;
use pin_project_lite::pin_project;

use crate::Yes;
use crate::pull::{FusedPull, Pull, PullStep};

pin_project! {
    /// Pull combinator that flattens items that are streams by polling each inner stream.
    ///
    /// When the inner stream yields `Poll::Pending`, this operator yields `Pending` as well.
    #[must_use = "`Pull`s do nothing unless polled"]
    pub struct FlattenStream<Prev, St, Meta> where St: Stream {
        #[pin]
        prev: Prev,
        #[pin]
        current: Option<FlattenStreamCurrent<St, Meta>>,
    }
}

pin_project! {
    struct FlattenStreamCurrent<St, Meta> where St: Stream {
        #[pin]
        stream: St,
        meta: Meta,
    }
}

impl<Prev, St, Meta> FlattenStream<Prev, St, Meta>
where
    Self: Pull,
    St: Stream,
{
    pub(crate) const fn new(prev: Prev) -> Self {
        Self {
            prev,
            current: None,
        }
    }
}

impl<Prev> Pull for FlattenStream<Prev, Prev::Item, Prev::Meta>
where
    Prev: Pull,
    Prev::Item: Stream,
{
    type Ctx<'ctx> = Context<'ctx>;

    type Item = <Prev::Item as Stream>::Item;
    type Meta = Prev::Meta;
    type CanPend = Yes;
    type CanEnd = Prev::CanEnd;

    fn size_hint(&self) -> (usize, Option<usize>) {
        let current_lower = self
            .current
            .as_ref()
            .map(|c| c.stream.size_hint().0)
            .unwrap_or_default();
        (current_lower, None)
    }

    fn pull(
        self: Pin<&mut Self>,
        ctx: &mut Self::Ctx<'_>,
    ) -> PullStep<Self::Item, Self::Meta, Self::CanPend, Self::CanEnd> {
        let mut this = self.project();
        loop {
            if let Some(cur) = this.current.as_mut().as_pin_mut().map(|c| c.project()) {
                match Stream::poll_next(cur.stream, ctx) {
                    core::task::Poll::Ready(Some(item)) => {
                        return PullStep::Ready(item, *cur.meta);
                    }
                    core::task::Poll::Ready(None) => {
                        this.current.as_mut().set(None);
                    }
                    core::task::Poll::Pending => {
                        return PullStep::Pending(Yes);
                    }
                }
            }
            debug_assert!(this.current.is_none());

            match this.prev.as_mut().pull(crate::Context::from_task(ctx)) {
                PullStep::Ready(stream, meta) => {
                    this.current
                        .as_mut()
                        .set(Some(FlattenStreamCurrent { stream, meta }));
                }
                PullStep::Pending(_) => {
                    return PullStep::Pending(Yes);
                }
                PullStep::Ended(can_end) => {
                    return PullStep::Ended(can_end);
                }
            }
        }
    }
}

impl<Prev> FusedPull for FlattenStream<Prev, Prev::Item, Prev::Meta>
where
    Prev: FusedPull,
    Prev::Item: Stream,
{
}

#[cfg(test)]
mod tests {
    use core::pin::Pin;
    use core::task::{Context, Waker};

    extern crate alloc;
    use alloc::vec;

    use futures_util::stream;

    use crate::Yes;
    use crate::pull::{Pull, PullStep};

    #[test]
    fn flatten_stream_basic() {
        let waker = Waker::noop();
        let mut cx = Context::from_waker(waker);

        let mut p = crate::pull::iter(vec![stream::iter(vec![1, 2]), stream::iter(vec![3])])
            .flatten_stream();
        let mut p = Pin::new(&mut p);

        assert_eq!(PullStep::Ready(1, ()), p.as_mut().pull(&mut cx));
        assert_eq!(PullStep::Ready(2, ()), p.as_mut().pull(&mut cx));
        assert_eq!(PullStep::Ready(3, ()), p.as_mut().pull(&mut cx));

        let step: PullStep<i32, (), Yes, Yes> = p.as_mut().pull(&mut cx);
        assert!(step.is_ended());
    }

    #[test]
    fn flatten_stream_pending_propagates() {
        let waker = Waker::noop();
        let mut cx = Context::from_waker(waker);

        let mut p = crate::pull::iter(vec![stream::pending::<i32>()]).flatten_stream();
        let mut p = Pin::new(&mut p);

        for _ in 0..10 {
            let step: PullStep<i32, (), Yes, Yes> = p.as_mut().pull(&mut cx);
            assert!(step.is_pending());
        }
    }
}
