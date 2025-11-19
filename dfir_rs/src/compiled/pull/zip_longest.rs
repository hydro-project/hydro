use std::pin::Pin;
use std::task::{Context, Poll, ready};

use futures::stream::{FusedStream, Stream};
use itertools::EitherOrBoth;
use pin_project_lite::pin_project;

pin_project! {
    /// Special stream for the `zip_longest` operator.
    #[must_use = "streams do nothing unless polled"]
    pub struct ZipLongest<St1, St2> {
        #[pin]
        stream1: St1,
        #[pin]
        stream2: St2,
    }
}

impl<St1, St2> ZipLongest<St1, St2>
where
    St1: FusedStream,
    St2: FusedStream,
{
    /// Create a new `ZipLongest` stream from two source streams.
    pub fn new(stream1: St1, stream2: St2) -> Self {
        Self { stream1, stream2 }
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

        let item_left = ready!(this.stream1.as_mut().poll_next(cx));
        let item_right = ready!(this.stream2.as_mut().poll_next(cx));
        Poll::Ready(match (item_left, item_right) {
            (None, None) => None,
            (Some(left), None) => Some(EitherOrBoth::Left(left)),
            (None, Some(right)) => Some(EitherOrBoth::Right(right)),
            (Some(left), Some(right)) => Some(EitherOrBoth::Both(left, right)),
        })
    }
}
