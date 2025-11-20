use std::collections::VecDeque;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::stream::{FusedStream, Stream};
use pin_project_lite::pin_project;

pin_project! {
    /// Special stream for the `zip` operator.
    #[must_use = "streams do nothing unless polled"]
    pub struct ZipPersist<'a, St1, St2>
    where
        St1: FusedStream,
        St2: FusedStream,
    {
        #[pin]
        stream1: St1,
        #[pin]
        stream2: St2,

        vec1: &'a mut VecDeque<St1::Item>,
        vec2: &'a mut VecDeque<St2::Item>,
    }
}

impl<'a, St1, St2> ZipPersist<'a, St1, St2>
where
    St1: FusedStream,
    St2: FusedStream,
{
    /// Create a new `ZipPersist` stream from two source streams.
    pub fn new(
        stream1: St1,
        stream2: St2,
        vec1: &'a mut VecDeque<St1::Item>,
        vec2: &'a mut VecDeque<St2::Item>,
    ) -> Self {
        Self {
            stream1,
            stream2,
            vec1,
            vec2,
        }
    }
}

impl<St1, St2> Stream for ZipPersist<'_, St1, St2>
where
    St1: FusedStream,
    St2: FusedStream,
{
    type Item = (St1::Item, St2::Item);

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        if !this.vec1.is_empty() && !this.vec2.is_empty() {
            let item1 = this.vec1.pop_front().unwrap();
            let item2 = this.vec2.pop_front().unwrap();
            return Poll::Ready(Some((item1, item2)));
        }

        loop {
            match (
                this.stream1.as_mut().poll_next(cx),
                this.stream2.as_mut().poll_next(cx),
            ) {
                (Poll::Ready(None), Poll::Ready(None)) => {
                    return Poll::Ready(None);
                }
                (Poll::Pending, Poll::Pending)
                | (Poll::Ready(None), Poll::Pending)
                | (Poll::Pending, Poll::Ready(None)) => {
                    return Poll::Pending;
                }
                (Poll::Ready(Some(item1)), Poll::Ready(Some(item2))) => {
                    return Poll::Ready(Some((item1, item2)));
                }
                (Poll::Ready(Some(item1)), _pending_or_none) => {
                    if let Some(item2) = this.vec2.pop_front() {
                        return Poll::Ready(Some((item1, item2)));
                    } else {
                        this.vec1.push_back(item1);
                    }
                }
                (_pending_or_none, Poll::Ready(Some(item2))) => {
                    if let Some(item1) = this.vec1.pop_front() {
                        return Poll::Ready(Some((item1, item2)));
                    } else {
                        this.vec2.push_back(item2);
                    }
                }
            }
        }
    }
}
