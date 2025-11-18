use std::cell::RefCell;
use std::pin::Pin;
use std::task::{Context, Poll, ready};

use futures::stream::{Fuse, FusedStream, Stream, StreamExt};
use itertools::Either;
use pin_project_lite::pin_project;
use smallvec::SmallVec;

use super::HalfJoinState;

pin_project! {
    pub struct SymmetricHashJoin<'a, Lhs, Rhs, LhsState, RhsState, Replay> {
        #[pin]
        lhs: Lhs,
        #[pin]
        rhs: Rhs,

        lhs_state: &'a RefCell<LhsState>,
        rhs_state: &'a RefCell<RhsState>,

        replay: Replay,
    }
}

impl<'a, Key, Lhs, V1, Rhs, V2, LhsState, RhsState, Replay> Stream
    for SymmetricHashJoin<'a, Fuse<Lhs>, Fuse<Rhs>, LhsState, RhsState, Replay>
where
    Key: Eq + std::hash::Hash + Clone,
    V1: Clone,
    V2: Clone,
    Lhs: Stream<Item = (Key, V1)>,
    Rhs: Stream<Item = (Key, V2)>,
    LhsState: HalfJoinState<Key, V1, V2>,
    RhsState: HalfJoinState<Key, V2, V1>,
    Replay: Iterator<Item = (Key, (V1, V2))>,
{
    type Item = (Key, (V1, V2));

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        let lhs_state = this.lhs_state.borrow_mut();
        let rhs_state = this.rhs_state.borrow_mut();
        loop {
            if let Some((k, v2, v1)) = lhs_state.pop_match() {
                return Poll::Ready(Some((k, (v1, v2))));
            }
            if let Some((k, v1, v2)) = rhs_state.pop_match() {
                return Poll::Ready(Some((k, (v1, v2))));
            }

            let lhs_poll = this.lhs.as_mut().poll_next(cx);
            if let Poll::Ready(Some((k, v1))) = lhs_poll {
                if this.lhs_state.build(k.clone(), &v1)
                    && let Some((k, v1, v2)) = this.rhs_state.probe(&k, &v1)
                {
                    return Poll::Ready(Some((k, (v1, v2))));
                }
                continue;
            }

            let rhs_poll = this.rhs.as_mut().poll_next(cx);
            if let Poll::Ready(Some((k, v2))) = rhs_poll {
                if this.rhs_state.build(k.clone(), &v2)
                    && let Some((k, v2, v1)) = this.lhs_state.probe(&k, &v2)
                {
                    return Poll::Ready(Some((k, (v1, v2))));
                }
                continue;
            }

            let _none = ready!(lhs_poll);
            let _none = ready!(rhs_poll);
            return Poll::Ready(None);
        }
    }
}

pub fn symmetric_hash_join_into_iter<'a, Key, Lhs, V1, Rhs, V2, LhsState, RhsState>(
    mut lhs: Lhs,
    mut rhs: Rhs,
    lhs_state: &'a mut LhsState,
    rhs_state: &'a mut RhsState,
    is_new_tick: bool,
) -> impl 'a + Iterator<Item = (Key, (V1, V2))>
where
    Key: 'a + Eq + std::hash::Hash + Clone,
    V1: 'a + Clone,
    V2: 'a + Clone,
    Lhs: 'a + Iterator<Item = (Key, V1)>,
    Rhs: 'a + Iterator<Item = (Key, V2)>,
    LhsState: HalfJoinState<Key, V1, V2>,
    RhsState: HalfJoinState<Key, V2, V1>,
{
    if is_new_tick {
        for (k, v1) in lhs.by_ref() {
            lhs_state.build(k.clone(), &v1);
        }

        for (k, v2) in rhs.by_ref() {
            rhs_state.build(k.clone(), &v2);
        }

        Either::Left(if lhs_state.len() < rhs_state.len() {
            Either::Left(lhs_state.iter().flat_map(|(k, sv)| {
                sv.iter().flat_map(|v1| {
                    rhs_state
                        .full_probe(k)
                        .map(|v2| (k.clone(), (v1.clone(), v2.clone())))
                })
            }))
        } else {
            Either::Right(rhs_state.iter().flat_map(|(k, sv)| {
                sv.iter().flat_map(|v2| {
                    lhs_state
                        .full_probe(k)
                        .map(|v1| (k.clone(), (v1.clone(), v2.clone())))
                })
            }))
        })
    } else {
        Either::Right(SymmetricHashJoin {
            lhs,
            rhs,
            lhs_state,
            rhs_state,
        })
    }
}

pub fn symmetric_hash_join_into_stream<'a, Key, Lhs, V1, Rhs, V2, LhsState, RhsState>(
    mut lhs: Lhs,
    mut rhs: Rhs,
    lhs_state: &'a RefCell<LhsState>,
    rhs_state: &'a RefCell<RhsState>,
    is_new_tick: bool,
) -> impl 'a + Stream<Item = (Key, (V1, V2))>
where
    Key: 'a + Eq + std::hash::Hash + Clone,
    V1: 'a + Clone,
    V2: 'a + Clone,
    Lhs: 'a + Stream<Item = (Key, V1)>,
    Rhs: 'a + Stream<Item = (Key, V2)>,
    LhsState: HalfJoinState<Key, V1, V2>,
    RhsState: HalfJoinState<Key, V2, V1>,
{
    fn assert_stream<St>(stream: St) -> St
    where
        St: Stream,
    {
        stream
    }

    let replay = if is_new_tick {
        // For Stream, we don't bother to pre-build all the new items this tick.
        // Instead, just replay, then do SymmetricHashJoin.

        Either::Left(if lhs_state.borrow().len() < rhs_state.borrow().len() {
            Either::Left(lhs_state.borrow().iter().flat_map(|(k, sv)| {
                sv.iter().flat_map(|v1| {
                    rhs_state
                        .borrow()
                        .full_probe(k)
                        .map(|v2| (k.clone(), (v1.clone(), v2.clone())))
                })
            }))
        } else {
            Either::Right(rhs_state.borrow().iter().flat_map(|(k, sv)| {
                sv.iter().flat_map(|v2| {
                    lhs_state
                        .borrow()
                        .full_probe(k)
                        .map(|v1| (k.clone(), (v1.clone(), v2.clone())))
                })
            }))
        })
    } else {
        Either::Right(std::iter::empty())
    };

    assert_stream(SymmetricHashJoin {
        lhs: lhs.fuse(),
        rhs: rhs.fuse(),
        lhs_state,
        rhs_state,
        replay,
    })
}

#[cfg(test)]
mod tests {
    use super::super::HalfSetJoinState;
    use super::*;

    use std::collections::HashSet;

    #[test]
    fn hash_join() {
        let lhs = (0..10).map(|x| (x, format!("left {}", x)));
        let rhs = (6..15).map(|x| (x / 2, format!("right {} / 2", x)));

        let (mut lhs_state, mut rhs_state) =
            (HalfSetJoinState::default(), HalfSetJoinState::default());
        let join = symmetric_hash_join_into_iter(lhs, rhs, &mut lhs_state, &mut rhs_state, true);

        let joined = join.collect::<HashSet<_>>();

        assert!(joined.contains(&(3, ("left 3".into(), "right 6 / 2".into()))));
        assert!(joined.contains(&(3, ("left 3".into(), "right 7 / 2".into()))));
        assert!(joined.contains(&(4, ("left 4".into(), "right 8 / 2".into()))));
        assert!(joined.contains(&(4, ("left 4".into(), "right 9 / 2".into()))));
        assert!(joined.contains(&(5, ("left 5".into(), "right 10 / 2".into()))));
        assert!(joined.contains(&(5, ("left 5".into(), "right 11 / 2".into()))));
        assert!(joined.contains(&(6, ("left 6".into(), "right 12 / 2".into()))));
        assert!(joined.contains(&(7, ("left 7".into(), "right 14 / 2".into()))));
    }

    #[crate::test]
    async fn hash_join_subsequent_ticks_do_produce_even_if_nothing_is_changed() {
        let (lhs_tx, lhs_rx) = tokio::sync::mpsc::unbounded_channel::<(usize, usize)>();
        let (rhs_tx, rhs_rx) = tokio::sync::mpsc::unbounded_channel::<(usize, usize)>();
        let lhs_rx = tokio_stream::wrappers::UnboundedReceiverStream::new(lhs_rx);
        let rhs_rx = tokio_stream::wrappers::UnboundedReceiverStream::new(rhs_rx);

        lhs_tx.send((7, 3)).unwrap();
        rhs_tx.send((7, 3)).unwrap();

        let (lhs_state, rhs_state) = (
            RefCell::new(HalfSetJoinState::default()),
            RefCell::new(HalfSetJoinState::default()),
        );
        let mut join =
            symmetric_hash_join_into_stream(lhs_rx, rhs_rx, &lhs_state, &rhs_state, true);

        assert_eq!(join.next().await, Some((7, (3, 3))));
        assert_eq!(join.next().await, None);

        lhs_tx.send((7, 3)).unwrap();
        rhs_tx.send((7, 3)).unwrap();

        assert_eq!(join.next().await, None);
    }
}
