use std::pin::Pin;
use std::task::{Context, Poll, ready};

use futures::stream::{Fuse, FusedStream, Stream, StreamExt};
use itertools::Either;
use pin_project_lite::pin_project;
use smallvec::SmallVec;

use super::HalfJoinState;

pin_project! {
    pub struct SymmetricHashJoin<'a, Lhs, Rhs, LhsState, RhsState> {
        #[pin]
        lhs: Lhs,
        #[pin]
        rhs: Rhs,

        lhs_state: &'a mut LhsState,
        rhs_state: &'a mut RhsState,
    }
}

impl<Key, Lhs, V1, Rhs, V2, LhsState, RhsState> Iterator
    for SymmetricHashJoin<'_, Lhs, Rhs, LhsState, RhsState>
where
    Key: Eq + std::hash::Hash + Clone,
    V1: Clone,
    V2: Clone,
    Lhs: Iterator<Item = (Key, V1)>,
    Rhs: Iterator<Item = (Key, V2)>,
    LhsState: HalfJoinState<Key, V1, V2>,
    RhsState: HalfJoinState<Key, V2, V1>,
{
    type Item = (Key, (V1, V2));

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some((k, v2, v1)) = self.lhs_state.pop_match() {
                return Some((k, (v1, v2)));
            }
            if let Some((k, v1, v2)) = self.rhs_state.pop_match() {
                return Some((k, (v1, v2)));
            }

            if let Some((k, v1)) = self.lhs.next() {
                if self.lhs_state.build(k.clone(), &v1)
                    && let Some((k, v1, v2)) = self.rhs_state.probe(&k, &v1)
                {
                    return Some((k, (v1, v2)));
                }
                continue;
            }
            if let Some((k, v2)) = self.rhs.next() {
                if self.rhs_state.build(k.clone(), &v2)
                    && let Some((k, v2, v1)) = self.lhs_state.probe(&k, &v2)
                {
                    return Some((k, (v1, v2)));
                }
                continue;
            }

            return None;
        }
    }
}

impl<Key, Lhs, V1, Rhs, V2, LhsState, RhsState> Stream
    for SymmetricHashJoin<'_, Fuse<Lhs>, Fuse<Rhs>, LhsState, RhsState>
where
    Key: Eq + std::hash::Hash + Clone,
    V1: Clone,
    V2: Clone,
    Lhs: Stream<Item = (Key, V1)>,
    Rhs: Stream<Item = (Key, V2)>,
    LhsState: HalfJoinState<Key, V1, V2>,
    RhsState: HalfJoinState<Key, V2, V1>,
{
    type Item = (Key, (V1, V2));

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        loop {
            if let Some((k, v2, v1)) = this.lhs_state.pop_match() {
                return Poll::Ready(Some((k, (v1, v2))));
            }
            if let Some((k, v1, v2)) = this.rhs_state.pop_match() {
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
    lhs_state: &'a mut LhsState,
    rhs_state: &'a mut RhsState,
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

    if is_new_tick {
        // For Stream, we don't bother to pre-build all the new items this tick.
        // Instead, just replay, then do SymmetricHashJoin.

        let replay = if lhs_state.len() < rhs_state.len() {
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
        };
        futures::future::Either::Left(futures::stream::iter(replay).chain(assert_stream(
            SymmetricHashJoin {
                lhs: lhs.fuse(),
                rhs: rhs.fuse(),
                lhs_state,
                rhs_state,
            },
        )))
    } else {
        futures::future::Either::Right(assert_stream(SymmetricHashJoin {
            lhs: lhs.fuse(),
            rhs: rhs.fuse(),
            lhs_state,
            rhs_state,
        }))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use crate::compiled::pull::{HalfSetJoinState, symmetric_hash_join_into_iter};

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

    #[test]
    fn hash_join_subsequent_ticks_do_produce_even_if_nothing_is_changed() {
        let (lhs_tx, lhs_rx) = std::sync::mpsc::channel::<(usize, usize)>();
        let (rhs_tx, rhs_rx) = std::sync::mpsc::channel::<(usize, usize)>();

        lhs_tx.send((7, 3)).unwrap();
        rhs_tx.send((7, 3)).unwrap();

        let (mut lhs_state, mut rhs_state) =
            (HalfSetJoinState::default(), HalfSetJoinState::default());
        let mut join = symmetric_hash_join_into_iter(
            lhs_rx.try_iter(),
            rhs_rx.try_iter(),
            &mut lhs_state,
            &mut rhs_state,
            true,
        );

        assert_eq!(join.next(), Some((7, (3, 3))));
        assert_eq!(join.next(), None);

        lhs_tx.send((7, 3)).unwrap();
        rhs_tx.send((7, 3)).unwrap();

        assert_eq!(join.next(), None);
    }
}
