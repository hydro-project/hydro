use std::pin::Pin;
use std::task::{Context, Poll, ready};

use futures::stream::Stream;
use pin_project_lite::pin_project;

pin_project! {
    /// Special stream for the `persist` operator
    #[must_use = "streams do nothing unless polled"]
    pub struct Persist<'ctx, St, Item> {
        #[pin]
        stream: St,
        vec: &'ctx mut Vec<Item>,
        replay_idx: usize,
    }
}

impl<'ctx, St, Item> Persist<'ctx, St, Item> {
    /// Create with the preceeding sink and given replay index.
    pub fn new(stream: St, vec: &'ctx mut Vec<Item>, replay_idx: usize) -> Self {
        debug_assert!(replay_idx <= vec.len());

        Self {
            stream,
            vec,
            replay_idx,
        }
    }
}

impl<St> Stream for Persist<'_, St, St::Item>
where
    St: Stream,
    St::Item: Clone,
{
    type Item = St::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();

        debug_assert!(*this.replay_idx <= this.vec.len());

        if let Some(item) = this.vec.get(*this.replay_idx) {
            // Replaying
            *this.replay_idx += 1;
            Poll::Ready(Some(item.clone()))
        } else {
            // Polling new
            debug_assert_eq!(this.vec.len(), *this.replay_idx);

            if let Some(item) = ready!(this.stream.poll_next(cx)) {
                // New item
                this.vec.push(item.clone());
                *this.replay_idx += 1;

                Poll::Ready(Some(item))
            } else {
                // Done
                Poll::Ready(None)
            }
        }
    }
}
