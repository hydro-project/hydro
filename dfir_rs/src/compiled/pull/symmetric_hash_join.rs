use std::borrow::Cow;
use std::pin::Pin;
use std::task::{Context, Poll, ready};

use futures::future::Either as FutEither;
use futures::stream::{FusedStream, Stream, StreamExt};
use itertools::Either as IterEither;
use pin_project_lite::pin_project;

use super::{ForEach, HalfJoinState};

pin_project! {
    /// Stream combinator for symmetric hash join operations.
    #[must_use = "streams do nothing unless polled"]
    pub struct SymmetricHashJoin<'a, Lhs, Rhs, LhsState, RhsState>
    {
        #[pin]
        lhs: Lhs,
        #[pin]
        rhs: Rhs,

        lhs_state: &'a mut LhsState,
        rhs_state: &'a mut RhsState,
    }
}

impl<'a, Key, Lhs, V1, Rhs, V2, LhsState, RhsState> Stream
    for SymmetricHashJoin<'a, Lhs, Rhs, LhsState, RhsState>
where
    Key: Eq + std::hash::Hash + Clone,
    V1: Clone,
    V2: Clone,
    Lhs: FusedStream<Item = (Key, V1)>,
    Rhs: FusedStream<Item = (Key, V2)>,
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
                if this.lhs_state.build(k.clone(), Cow::Borrowed(&v1))
                    && let Some((k, v1, v2)) = this.rhs_state.probe(&k, &v1)
                {
                    return Poll::Ready(Some((k, (v1, v2))));
                }
                continue;
            }

            let rhs_poll = this.rhs.as_mut().poll_next(cx);
            if let Poll::Ready(Some((k, v2))) = rhs_poll {
                if this.rhs_state.build(k.clone(), Cow::Borrowed(&v2))
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

/// Creates a symmetric hash join stream from two input streams and their join states.
pub async fn symmetric_hash_join_into_stream<'a, Key, Lhs, V1, Rhs, V2, LhsState, RhsState>(
    lhs: Lhs,
    rhs: Rhs,
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
    if is_new_tick {
        ForEach::new(lhs, |(k, v1)| {
            lhs_state.build(k.clone(), Cow::Owned(v1));
        })
        .await;

        ForEach::new(rhs, |(k, v2)| {
            rhs_state.build(k.clone(), Cow::Owned(v2));
        })
        .await;

        let iter = if lhs_state.len() < rhs_state.len() {
            IterEither::Left(lhs_state.iter().flat_map(|(k, sv)| {
                sv.iter().flat_map(|v1| {
                    rhs_state
                        .full_probe(k)
                        .map(|v2| (k.clone(), (v1.clone(), v2.clone())))
                })
            }))
        } else {
            IterEither::Right(rhs_state.iter().flat_map(|(k, sv)| {
                sv.iter().flat_map(|v2| {
                    lhs_state
                        .full_probe(k)
                        .map(|v1| (k.clone(), (v1.clone(), v2.clone())))
                })
            }))
        };
        FutEither::Left(futures::stream::iter(iter))
    } else {
        FutEither::Right(SymmetricHashJoin {
            lhs: lhs.fuse(),
            rhs: rhs.fuse(),
            lhs_state,
            rhs_state,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::super::HalfSetJoinState;
    use super::*;

    #[crate::test]
    async fn hash_join() {
        let lhs = futures::stream::iter((0..10).map(|x| (x, format!("left {}", x))));
        let rhs = futures::stream::iter((6..15).map(|x| (x / 2, format!("right {} / 2", x))));

        let (mut lhs_state, mut rhs_state) =
            (HalfSetJoinState::default(), HalfSetJoinState::default());
        let join =
            symmetric_hash_join_into_stream(lhs, rhs, &mut lhs_state, &mut rhs_state, true).await;

        let joined = join.collect::<HashSet<_>>().await;

        assert!(joined.contains(&(3, ("left 3".into(), "right 6 / 2".into()))));
        assert!(joined.contains(&(3, ("left 3".into(), "right 7 / 2".into()))));
        assert!(joined.contains(&(4, ("left 4".into(), "right 8 / 2".into()))));
        assert!(joined.contains(&(4, ("left 4".into(), "right 9 / 2".into()))));
        assert!(joined.contains(&(5, ("left 5".into(), "right 10 / 2".into()))));
        assert!(joined.contains(&(5, ("left 5".into(), "right 11 / 2".into()))));
        assert!(joined.contains(&(6, ("left 6".into(), "right 12 / 2".into()))));
        assert!(joined.contains(&(6, ("left 6".into(), "right 13 / 2".into()))));
        assert!(joined.contains(&(7, ("left 7".into(), "right 14 / 2".into()))));
        assert_eq!(9, joined.len());
    }

    #[crate::test]
    async fn hash_join_subsequent_ticks_do_produce_even_if_nothing_is_changed() {
        let (lhs_tx, lhs_rx) = tokio::sync::mpsc::unbounded_channel::<(usize, usize)>();
        let (rhs_tx, rhs_rx) = tokio::sync::mpsc::unbounded_channel::<(usize, usize)>();
        let lhs_rx = tokio_stream::wrappers::UnboundedReceiverStream::new(lhs_rx);
        let rhs_rx = tokio_stream::wrappers::UnboundedReceiverStream::new(rhs_rx);

        lhs_tx.send((7, 3)).unwrap();
        rhs_tx.send((7, 3)).unwrap();

        let (mut lhs_state, mut rhs_state) =
            (HalfSetJoinState::default(), HalfSetJoinState::default());
        let mut join =
            symmetric_hash_join_into_stream(lhs_rx, rhs_rx, &mut lhs_state, &mut rhs_state, false)
                .await;

        assert_eq!(join.next().await, Some((7, (3, 3))));

        lhs_tx.send((7, 3)).unwrap();
        rhs_tx.send((7, 3)).unwrap();
        drop((lhs_tx, rhs_tx));

        assert_eq!(join.next().await, None);
    }
}
