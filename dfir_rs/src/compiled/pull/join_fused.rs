/// State semantics for (each half of) the `join_fused` operator.
use std::collections::HashMap;
use std::collections::hash_map;
use std::hash::{BuildHasher, Hash};
use std::pin::Pin;
use std::task::{Context, Poll, ready};

use futures::stream::{FusedStream, Stream};
use pin_project_lite::pin_project;

use crate::util::accumulator::Accumulator;

pin_project! {
    #[project = JoinFusedStateProj]
    #[project_replace = JoinFusedStateProjOwn]
    enum JoinFusedState<'a, Lhs, Rhs, Key, LhsVal, RhsVal, LhsHasher, RhsHasher, LhsAccum, RhsAccum> {
        Build {
            #[pin]
            lhs: Lhs,
            #[pin]
            rhs: Rhs,

            lhs_accum: LhsAccum,
            rhs_accum: RhsAccum,

            lhs_state: &'a mut HashMap<Key, LhsVal, LhsHasher>,
            rhs_state: &'a mut HashMap<Key, RhsVal, RhsHasher>,
        },
        ScanLeft {
            lhs_scan: hash_map::Iter<'a, Key, LhsVal>,
            rhs_state: &'a mut HashMap<Key, RhsVal, RhsHasher>,
        },
        ScanRight {
            lhs_state: &'a mut HashMap<Key, LhsVal, LhsHasher>,
            rhs_scan: hash_map::Iter<'a, Key, RhsVal>,
        },
        Empty,
    }
}

pin_project! {
    pub struct JoinFused<'a, Lhs, Rhs, Key, LhsVal, RhsVal, LhsHasher, RhsHasher, LhsAccum, RhsAccum> {
        #[pin]
        state: JoinFusedState<'a, Lhs, Rhs, Key, LhsVal, RhsVal, LhsHasher, RhsHasher, LhsAccum, RhsAccum>,
    }
}

impl<'a, Lhs, Rhs, Key, LhsVal, RhsVal, LhsHasher, RhsHasher, LhsAccum, RhsAccum, LhsItem, RhsItem>
    JoinFused<'a, Lhs, Rhs, Key, LhsVal, RhsVal, LhsHasher, RhsHasher, LhsAccum, RhsAccum>
where
    Lhs: FusedStream<Item = (Key, LhsItem)>,
    Rhs: Stream<Item = (Key, RhsItem)>,
    LhsAccum: Accumulator<LhsVal, LhsItem>,
    RhsAccum: Accumulator<RhsVal, RhsItem>,
    Key: Clone + Eq + Hash,
    LhsHasher: BuildHasher,
    RhsHasher: BuildHasher,
    LhsVal: Clone,
    RhsVal: Clone,
{
    pub fn new(
        lhs: Lhs,
        rhs: Rhs,
        lhs_accum: LhsAccum,
        rhs_accum: RhsAccum,
        lhs_state: &'a mut HashMap<Key, LhsVal, LhsHasher>,
        rhs_state: &'a mut HashMap<Key, RhsVal, RhsHasher>,
    ) -> Self {
        Self {
            state: JoinFusedState::Build {
                lhs,
                rhs,
                lhs_accum,
                rhs_accum,
                lhs_state,
                rhs_state,
            },
        }
    }
}

impl<'a, Lhs, Rhs, Key, LhsVal, RhsVal, LhsHasher, RhsHasher, LhsAccum, RhsAccum, LhsItem, RhsItem>
    Stream
    for JoinFused<'a, Lhs, Rhs, Key, LhsVal, RhsVal, LhsHasher, RhsHasher, LhsAccum, RhsAccum>
where
    Lhs: FusedStream<Item = (Key, LhsItem)>,
    Rhs: Stream<Item = (Key, RhsItem)>,
    LhsAccum: Accumulator<LhsVal, LhsItem>,
    RhsAccum: Accumulator<RhsVal, RhsItem>,
    Key: Clone + Eq + Hash,
    LhsHasher: BuildHasher,
    RhsHasher: BuildHasher,
    LhsVal: Clone,
    RhsVal: Clone,
{
    type Item = (Key, (LhsVal, RhsVal));

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.as_mut().project();

        match this.state.as_mut().project() {
            JoinFusedStateProj::Build {
                mut lhs,
                mut rhs,
                lhs_state,
                rhs_state,
                lhs_accum,
                rhs_accum,
            } => {
                // Build left
                while let Some((key, lhs_item)) = ready!(lhs.as_mut().poll_next(cx)) {
                    let () = lhs_accum.accumulate(lhs_state.entry(key), lhs_item);
                }
                // Build right
                while let Some((key, rhs_item)) = ready!(rhs.as_mut().poll_next(cx)) {
                    let () = rhs_accum.accumulate(rhs_state.entry(key), rhs_item);
                }

                let JoinFusedStateProjOwn::Build {
                    lhs_state,
                    rhs_state,
                    ..
                } = this.state.as_mut().project_replace(JoinFusedState::Empty)
                else {
                    unreachable!();
                };

                this.state
                    .as_mut()
                    .set(if lhs_state.len() <= rhs_state.len() {
                        JoinFusedState::ScanLeft {
                            lhs_scan: lhs_state.iter(),
                            rhs_state,
                        }
                    } else {
                        JoinFusedState::ScanRight {
                            lhs_state,
                            rhs_scan: rhs_state.iter(),
                        }
                    });
                self.poll_next(cx)
            }
            JoinFusedStateProj::ScanLeft {
                lhs_scan,
                rhs_state,
            } => {
                while let Some((key, lhs_item)) = lhs_scan.next() {
                    if let Some(rhs_item) = rhs_state.get(key) {
                        return Poll::Ready(Some((
                            key.clone(),
                            (lhs_item.clone(), rhs_item.clone()),
                        )));
                    }
                }
                this.state.set(JoinFusedState::Empty);
                Poll::Ready(None)
            }
            JoinFusedStateProj::ScanRight {
                lhs_state,
                rhs_scan,
            } => {
                while let Some((key, rhs_item)) = rhs_scan.next() {
                    if let Some(lhs_item) = lhs_state.get(key) {
                        return Poll::Ready(Some((
                            key.clone(),
                            (lhs_item.clone(), rhs_item.clone()),
                        )));
                    }
                }
                this.state.set(JoinFusedState::Empty);
                Poll::Ready(None)
            }
            JoinFusedStateProj::Empty => Poll::Ready(None),
        }
    }
}

pin_project! {
    pub struct JoinFusedLhs<'a, Lhs, Rhs, Key, LhsVal, RhsVal, LhsHasher, LhsAccum> {
        #[pin]
        lhs: Lhs,
        #[pin]
        rhs: Rhs,

        lhs_accum: LhsAccum,

        lhs_state: &'a mut HashMap<Key, LhsVal, LhsHasher>,
        rhs_state: &'a mut Vec<(Key, RhsVal)>,
        rhs_replay_idx: usize,
    }
}

impl<'a, Lhs, Rhs, Key, LhsVal, RhsVal, LhsHasher, LhsAccum, LhsItem>
    JoinFusedLhs<'a, Lhs, Rhs, Key, LhsVal, RhsVal, LhsHasher, LhsAccum>
where
    Lhs: FusedStream<Item = (Key, LhsItem)>,
    Rhs: Stream<Item = (Key, RhsVal)>,
    LhsAccum: Accumulator<LhsVal, LhsItem>,
    Key: Eq + Hash,
    LhsHasher: BuildHasher,
    LhsVal: Clone,
    RhsVal: Clone,
{
    pub fn new(
        lhs: Lhs,
        rhs: Rhs,
        lhs_accum: LhsAccum,
        lhs_state: &'a mut HashMap<Key, LhsVal, LhsHasher>,
        rhs_state: &'a mut Vec<(Key, RhsVal)>,
        rhs_replay_idx: usize,
    ) -> Self {
        debug_assert!(rhs_replay_idx <= rhs_state.len());
        Self {
            lhs,
            rhs,
            lhs_accum,
            lhs_state,
            rhs_state,
            rhs_replay_idx,
        }
    }
}

impl<'a, Lhs, Rhs, Key, LhsVal, RhsVal, LhsHasher, LhsAccum, LhsItem> Stream
    for JoinFusedLhs<'a, Lhs, Rhs, Key, LhsVal, RhsVal, LhsHasher, LhsAccum>
where
    Lhs: FusedStream<Item = (Key, LhsItem)>,
    Rhs: Stream<Item = (Key, RhsVal)>,
    LhsAccum: Accumulator<LhsVal, LhsItem>,
    Key: Clone + Eq + Hash,
    LhsHasher: BuildHasher,
    LhsVal: Clone,
    RhsVal: Clone,
{
    type Item = (Key, (LhsVal, RhsVal));

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.as_mut().project();

        // Stage 1: Accumulate all LHS items.
        while let Some((key, lhs_item)) = ready!(this.lhs.as_mut().poll_next(cx)) {
            let () = this
                .lhs_accum
                .accumulate(this.lhs_state.entry(key), lhs_item);
        }

        // Stage 2: Replay.
        while let Some((key, rhs_item)) = this.rhs_state.get(*this.rhs_replay_idx) {
            *this.rhs_replay_idx += 1;
            if let Some(lhs_item) = this.lhs_state.get(key) {
                return Poll::Ready(Some((key.clone(), (lhs_item.clone(), rhs_item.clone()))));
            }
        }

        // Stage 3: Stream.
        while let Some((key, rhs_item)) = ready!(this.rhs.as_mut().poll_next(cx)) {
            *this.rhs_replay_idx += 1;
            if let Some(lhs_item) = this.lhs_state.get(&key) {
                this.rhs_state.push((key.clone(), rhs_item.clone()));
                return Poll::Ready(Some((key, (lhs_item.clone(), rhs_item))));
            } else {
                this.rhs_state.push((key, rhs_item));
            }
        }

        // Done
        Poll::Ready(None)
    }
}
