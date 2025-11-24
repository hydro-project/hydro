use itertools::Either;
use rustc_hash::FxHashSet;

/// A custom [`Iterator`] for the `anti_join` operator.
pub struct AntiJoin<'a, Key, V, Ipos>
where
    Key: Eq + std::hash::Hash + Clone,
    V: Eq + std::hash::Hash + Clone,
    Ipos: Iterator<Item = (Key, V)>,
{
    input_pos: Ipos,
    neg_state: &'a mut FxHashSet<Key>,
    pos_state: &'a mut FxHashSet<(Key, V)>,
}

impl<Key, V, Ipos> Iterator for AntiJoin<'_, Key, V, Ipos>
where
    Key: Eq + std::hash::Hash + Clone,
    V: Eq + std::hash::Hash + Clone,
    Ipos: Iterator<Item = (Key, V)>,
{
    type Item = (Key, V);

    fn next(&mut self) -> Option<Self::Item> {
        for item in self.input_pos.by_ref() {
            if !self.neg_state.contains(&item.0) && !self.pos_state.contains(&item) {
                self.pos_state.insert(item.clone());
                return Some(item);
            }
        }

        None
    }
}

/// Creates a custom [`Iterator`] for the `anti_join` operator, replaying if `new_tick`.
pub fn anti_join_into_iter<'a, Key, V, Ipos>(
    input_pos: Ipos,
    state_neg: &'a mut FxHashSet<Key>,
    state_pos: &'a mut FxHashSet<(Key, V)>,
    new_tick: bool,
) -> impl 'a + Iterator<Item = (Key, V)>
where
    Key: Eq + std::hash::Hash + Clone,
    V: Eq + std::hash::Hash + Clone,
    Ipos: 'a + Iterator<Item = (Key, V)>,
{
    if new_tick {
        Either::Left(
            state_pos
                .iter()
                .filter(|kv| !state_neg.contains(&kv.0))
                .cloned()
                .collect::<Vec<_>>()
                .into_iter()
                .chain(input_pos.filter_map(|kv| {
                    if !state_neg.contains(&kv.0) {
                        state_pos.insert(kv.clone());
                        Some(kv)
                    } else {
                        None
                    }
                })),
        )
    } else {
        Either::Right(AntiJoin {
            input_pos,
            neg_state: state_neg,
            pos_state: state_pos,
        })
    }
}
