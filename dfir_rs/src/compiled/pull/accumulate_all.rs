use std::collections::HashMap;
use std::hash::{BuildHasher, Hash};
use std::pin::pin;

use futures::stream::{Stream, StreamExt};

use crate::util::accumulator::Accumulator;

/// Use the accumulator `accum` to accumulate all entries in the stream `st` into the `hash_map`.
pub async fn accumulate_all<Key, ValAccum, ValIn>(
    accum: &mut impl Accumulator<ValAccum, ValIn>,
    hash_map: &mut HashMap<Key, ValAccum, impl BuildHasher>,
    st: impl Stream<Item = (Key, ValIn)>,
) where
    Key: Eq + Hash,
{
    let mut st = pin!(st);
    while let Some((key, item)) = st.next().await {
        accum.accumulate(hash_map.entry(key), item);
    }
}

/// Use the accumulator `accum` to accumulate all entries in the `Pull` `prev` into the `hash_map`.
pub async fn accumulate_all_pull<Key, ValAccum, ValIn>(
    accum: &mut impl Accumulator<ValAccum, ValIn>,
    hash_map: &mut HashMap<Key, ValAccum, impl BuildHasher>,
    prev: impl dfir_pipes::Pull<Item = (Key, ValIn)>,
) where
    Key: Eq + Hash,
{
    let () = prev
        .for_each(|(key, item)| {
            accum.accumulate(hash_map.entry(key), item);
        })
        .await;
}
