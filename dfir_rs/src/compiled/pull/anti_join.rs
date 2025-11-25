use std::pin::Pin;
use std::task::{Context, Poll, ready};

use futures::Stream;
use futures::stream::FusedStream;
use pin_project_lite::pin_project;
use rustc_hash::FxHashSet;

pin_project! {
    /// Special stream for the `anti_join` operator when `[pos]` is persisted.
    #[must_use = "streams do nothing unless polled"]
    pub struct AntiJoinPersist<'a, StPos, StNeg, Key, Val> {
        #[pin]
        stream_pos: StPos,
        #[pin]
        stream_neg: StNeg,

        state_pos: &'a mut Vec<(Key, Val)>,
        state_neg: &'a mut FxHashSet<Key>,
        replay_idx: usize,
    }
}

impl<'a, StPos, StNeg, Key, Val> AntiJoinPersist<'a, StPos, StNeg, Key, Val>
where
    StPos: Stream<Item = (Key, Val)>,
    StNeg: FusedStream<Item = Key>,
    Key: Eq + std::hash::Hash + Clone,
    Val: Eq + std::hash::Hash + Clone,
{
    pub fn new(
        stream_pos: StPos,
        stream_neg: StNeg,
        state_pos: &'a mut Vec<(Key, Val)>,
        state_neg: &'a mut FxHashSet<Key>,
        replay_idx: usize,
    ) -> Self {
        debug_assert!(replay_idx <= state_pos.len());

        Self {
            stream_pos,
            stream_neg,
            state_pos,
            state_neg,
            replay_idx,
        }
    }
}

impl<'a, StPos, StNeg, Key, Val> Stream for AntiJoinPersist<'a, StPos, StNeg, Key, Val>
where
    StPos: Stream<Item = (Key, Val)>,
    StNeg: FusedStream<Item = Key>,
    Key: Eq + std::hash::Hash + Clone,
    Val: Eq + std::hash::Hash + Clone,
{
    type Item = (Key, Val);

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        // Stage 1: Get all negative.
        while let Some(neg_item) = ready!(this.stream_neg.as_mut().poll_next(cx)) {
            this.state_neg.insert(neg_item);
        }

        // Stage 2: Replay.
        while let Some(item) = this.state_pos.get(*this.replay_idx) {
            *this.replay_idx += 1;
            if !this.state_neg.contains(&item.0) {
                return Poll::Ready(Some(item.clone()));
            }
        }

        // Stage 3: stream, filter, and store positive.
        debug_assert_eq!(this.state_pos.len(), *this.replay_idx);

        while let Some(item) = ready!(this.stream_pos.as_mut().poll_next(cx)) {
            *this.replay_idx += 1;
            if !this.state_neg.contains(&item.0) {
                this.state_pos.push(item.clone());
                return Poll::Ready(Some(item));
            } else {
                this.state_pos.push(item);
            }
        }

        // Done
        Poll::Ready(None)
    }
}

pin_project! {
    /// Special stream for the `anti_join` operator when `[pos]` is not persisted.
    #[must_use = "streams do nothing unless polled"]
    pub struct AntiJoin<'a, StPos, StNeg, Key> {
        #[pin]
        stream_pos: StPos,
        #[pin]
        stream_neg: StNeg,

        state_neg: &'a mut FxHashSet<Key>,
    }
}

impl<'a, StPos, StNeg, Key, Val> AntiJoin<'a, StPos, StNeg, Key>
where
    StPos: Stream<Item = (Key, Val)>,
    StNeg: FusedStream<Item = Key>,
    Key: Eq + std::hash::Hash + Clone,
    Val: Eq + std::hash::Hash + Clone,
{
    pub fn new(stream_pos: StPos, stream_neg: StNeg, state_neg: &'a mut FxHashSet<Key>) -> Self {
        Self {
            stream_pos,
            stream_neg,
            state_neg,
        }
    }
}

impl<'a, StPos, StNeg, Key, Val> Stream for AntiJoin<'a, StPos, StNeg, Key>
where
    StPos: Stream<Item = (Key, Val)>,
    StNeg: FusedStream<Item = Key>,
    Key: Eq + std::hash::Hash + Clone,
    Val: Eq + std::hash::Hash + Clone,
{
    type Item = (Key, Val);

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        // First, get all negative.
        while let Some(neg_item) = ready!(this.stream_neg.as_mut().poll_next(cx)) {
            this.state_neg.insert(neg_item);
        }

        // Then stream & filter positive.
        while let Some(item) = ready!(this.stream_pos.as_mut().poll_next(cx)) {
            if !this.state_neg.contains(&item.0) {
                return Poll::Ready(Some(item));
            }
        }

        // Done
        Poll::Ready(None)
    }
}
