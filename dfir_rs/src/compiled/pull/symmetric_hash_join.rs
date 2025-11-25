use itertools::Either;

use super::HalfJoinState;

/// Custom [`Iterator`] for the `join` operator.
pub struct SymmetricHashJoin<'a, Key, I1, V1, I2, V2, LhsState, RhsState>
where
    Key: Eq + std::hash::Hash + Clone,
    V1: Clone,
    V2: Clone,
    I1: Iterator<Item = (Key, V1)>,
    I2: Iterator<Item = (Key, V2)>,
    LhsState: HalfJoinState<Key, V1, V2>,
    RhsState: HalfJoinState<Key, V2, V1>,
{
    lhs: I1,
    rhs: I2,
    lhs_state: &'a mut LhsState,
    rhs_state: &'a mut RhsState,
}

impl<Key, I1, V1, I2, V2, LhsState, RhsState> Iterator
    for SymmetricHashJoin<'_, Key, I1, V1, I2, V2, LhsState, RhsState>
where
    Key: Eq + std::hash::Hash + Clone,
    V1: Clone,
    V2: Clone,
    I1: Iterator<Item = (Key, V1)>,
    I2: Iterator<Item = (Key, V2)>,
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

/// Creates a custom [`Iterator`] for the `join` operator.
pub fn symmetric_hash_join_into_iter<'a, Key, I1, V1, I2, V2, LhsState, RhsState>(
    mut lhs: I1,
    mut rhs: I2,
    lhs_state: &'a mut LhsState,
    rhs_state: &'a mut RhsState,
    is_new_tick: bool,
) -> impl 'a + Iterator<Item = (Key, (V1, V2))>
where
    Key: 'a + Eq + std::hash::Hash + Clone,
    V1: 'a + Clone,
    V2: 'a + Clone,
    I1: 'a + Iterator<Item = (Key, V1)>,
    I2: 'a + Iterator<Item = (Key, V2)>,
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
