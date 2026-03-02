use std::pin::Pin;
use std::task::{Context, Poll, ready};

use futures::stream::{FusedStream, Stream};
use pin_project_lite::pin_project;

use crate::itertools::EitherOrBoth;

pin_project! {
    /// Special stream for the `zip_longest` operator.
    #[must_use = "streams do nothing unless polled"]
    pub struct ZipLongest<St1: Stream, St2: Stream> {
        #[pin]
        stream1: St1,
        #[pin]
        stream2: St2,
        // Buffers an item from `stream1` so it is not lost if `stream2` returns `Poll::Pending`.
        // `None` = no buffered item (stream1 not yet polled, or result already consumed);
        // `Some(item)` = item waiting to be paired with stream2's next value.
        item1: Option<St1::Item>,
    }
}

impl<St1, St2> ZipLongest<St1, St2>
where
    St1: FusedStream,
    St2: FusedStream,
{
    /// Create a new `ZipLongest` stream from two source streams.
    pub fn new(stream1: St1, stream2: St2) -> Self {
        Self {
            stream1,
            stream2,
            item1: None,
        }
    }
}

impl<St1, St2> Stream for ZipLongest<St1, St2>
where
    St1: FusedStream,
    St2: FusedStream,
{
    type Item = EitherOrBoth<St1::Item, St2::Item>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        // Store `item1` so it is not dropped if `stream2` returns `Poll::Pending`.
        if this.item1.is_none() {
            *this.item1 = ready!(this.stream1.as_mut().poll_next(cx));
        }
        let item2 = ready!(this.stream2.as_mut().poll_next(cx));

        Poll::Ready(match (this.item1.take(), item2) {
            (None, None) => None,
            (Some(item1), None) => Some(EitherOrBoth::Left(item1)),
            (None, Some(item2)) => Some(EitherOrBoth::Right(item2)),
            (Some(item1), Some(item2)) => Some(EitherOrBoth::Both(item1, item2)),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::pin::pin;
    use std::task::{Context, Poll, Waker};

    use futures::stream::{FusedStream, Stream};
    use itertools::EitherOrBoth;

    use super::ZipLongest;

    /// A stream that returns `Poll::Pending` for the first `pending_count` polls, then yields
    /// items from `items`, then returns `Poll::Ready(None)`.
    struct PendingThenItems<T> {
        pending_count: usize,
        items: std::vec::IntoIter<T>,
        done: bool,
    }

    impl<T> PendingThenItems<T> {
        fn new(pending_count: usize, items: Vec<T>) -> Self {
            Self {
                pending_count,
                items: items.into_iter(),
                done: false,
            }
        }
    }

    impl<T: Unpin> Stream for PendingThenItems<T> {
        type Item = T;

        fn poll_next(mut self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<T>> {
            if self.pending_count > 0 {
                self.pending_count -= 1;
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }
            let item = self.items.next();
            if item.is_none() {
                self.done = true;
            }
            Poll::Ready(item)
        }
    }

    impl<T: Unpin> FusedStream for PendingThenItems<T> {
        fn is_terminated(&self) -> bool {
            self.done
        }
    }

    /// Regression test: LHS item must not be dropped when RHS returns `Poll::Pending`.
    #[test]
    fn test_lhs_not_dropped_when_rhs_pending() {
        // LHS: immediately yields 1, 2, 3
        // RHS: returns Pending once, then yields 10, 20
        let lhs = PendingThenItems::new(0, vec![1_i32, 2, 3]);
        let rhs = PendingThenItems::new(1, vec![10_i32, 20]);

        let mut zip = pin!(ZipLongest::new(lhs, rhs));
        let mut cx = Context::from_waker(Waker::noop());

        // First poll: LHS ready(1), RHS pending -> should return Pending (with LHS buffered)
        assert_eq!(Poll::Pending, zip.as_mut().poll_next(&mut cx));

        // Second poll: LHS buffered(1), RHS ready(10) -> Both(1, 10)
        assert_eq!(
            Poll::Ready(Some(EitherOrBoth::Both(1, 10))),
            zip.as_mut().poll_next(&mut cx)
        );

        // Remaining: Both(2, 20), Left(3), None
        assert_eq!(
            Poll::Ready(Some(EitherOrBoth::Both(2, 20))),
            zip.as_mut().poll_next(&mut cx)
        );
        assert_eq!(
            Poll::Ready(Some(EitherOrBoth::Left(3))),
            zip.as_mut().poll_next(&mut cx)
        );
        assert_eq!(Poll::Ready(None), zip.as_mut().poll_next(&mut cx));
    }
}
