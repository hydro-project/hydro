use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;

use hydro_lang::live_collections::stream::NoOrder;
use hydro_lang::prelude::*;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

mod sequence_payloads;

pub struct Replica {}

pub trait KvKey: Serialize + DeserializeOwned + Hash + Eq + Clone + Debug {}
impl<K: Serialize + DeserializeOwned + Hash + Eq + Clone + Debug> KvKey for K {}

pub trait KvValue: Serialize + DeserializeOwned + Eq + Clone + Debug {}
impl<V: Serialize + DeserializeOwned + Eq + Clone + Debug> KvValue for V {}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
pub struct KvPayload<K, V> {
    pub key: K,
    pub value: V,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
pub struct SequencedKv<K, V> {
    // Note: Important that seq is the first member of the struct for sorting
    pub seq: usize,
    pub kv: Option<KvPayload<K, V>>,
}

impl<K: KvKey, V: KvValue> Ord for SequencedKv<K, V> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.seq.cmp(&other.seq)
    }
}

impl<K: KvKey, V: KvValue> PartialOrd for SequencedKv<K, V> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

// Replicas. All relations for replicas will be prefixed with r. Expects ReplicaPayload on p_to_replicas, outputs a stream of (client address, ReplicaPayload) after processing.
#[expect(clippy::type_complexity, reason = "internal paxos code // TODO")]
pub fn kv_replica<'a, K: KvKey, V: KvValue>(
    replicas: &Cluster<'a, Replica>,
    p_to_replicas: impl Into<
        Stream<(usize, Option<KvPayload<K, V>>), Cluster<'a, Replica>, Unbounded, NoOrder>,
    >,
    checkpoint_frequency: usize,
) -> (
    Stream<usize, Cluster<'a, Replica>, Unbounded>,
    Stream<KvPayload<K, V>, Cluster<'a, Replica>, Unbounded>,
) {
    let p_to_replicas: Stream<SequencedKv<K, V>, Cluster<'a, Replica>, Unbounded, NoOrder> =
        p_to_replicas
            .into()
            .map(q!(|(slot, kv)| SequencedKv { seq: slot, kv }));

    let replica_tick = replicas.tick();

    let (r_processable_payloads, r_next_slot_complete_cycle) =
        sequence_payloads::sequence_payloads(&replica_tick, p_to_replicas);

    let r_kv_store = r_processable_payloads
        .clone()
        .all_ticks_atomic()
        .fold(q!(|| (HashMap::new(), 0)), q!(|(kv_store, next_slot), payload| {
            if let Some(kv) = payload.kv {
                kv_store.insert(kv.key, kv.value);
            }
            *next_slot = payload.seq + 1;
        }))
        .snapshot_atomic(nondet!(/** always up to date with batch being processed */));
    // Update the highest seq for the next tick
    let r_next_slot = r_kv_store.map(q!(|(_kv_store, next_slot)| next_slot));
    r_next_slot_complete_cycle.complete_next_tick(r_next_slot.clone());

    // Send checkpoints to the acceptors when we've processed enough payloads
    let (r_checkpointed_seqs_complete_cycle, r_checkpointed_seqs) =
        replica_tick.cycle::<Optional<usize, _, _>>();
    let r_max_checkpointed_seq = r_checkpointed_seqs
        .into_stream()
        .all_ticks_atomic()
        .max()
        .snapshot_atomic(nondet!(/** always up to date with batch being processed */))
        .into_singleton();
    let r_checkpoint_seq_new = r_max_checkpointed_seq
        .zip(
            Optional::from(r_next_slot)
                .defer_tick()
                .unwrap_or(replica_tick.singleton(q!(0))),
        )
        .filter_map(q!(
            move |(max_checkpointed_seq, next_slot)| if max_checkpointed_seq
                .map(|m| next_slot - m >= checkpoint_frequency)
                .unwrap_or(true)
            {
                Some(next_slot)
            } else {
                None
            }
        ));
    r_checkpointed_seqs_complete_cycle.complete_next_tick(r_checkpoint_seq_new.clone());

    // Tell clients that the payload has been committed. All ReplicaPayloads contain the client's machine ID (to string) as value.
    let r_to_clients = r_processable_payloads
        .filter_map(q!(|payload| payload.kv))
        .all_ticks();
    (r_checkpoint_seq_new.all_ticks(), r_to_clients)
}
