use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;

use hydro_lang::live_collections::stream::{NoOrder, TotalOrder};
use hydro_lang::prelude::*;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

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

    let (r_buffered_payloads_complete_cycle, r_buffered_payloads) =
        replica_tick
            .cycle::<Stream<SequencedKv<K, V>, Tick<Cluster<'a, Replica>>, Bounded, TotalOrder>>();
    // p_to_replicas.inspect(q!(|payload: ReplicaPayload| println!("Replica received payload: {:?}", payload)));
    let r_sorted_payloads = p_to_replicas.batch(&replica_tick, nondet!(
            /// because we fill slots one-by-one, we can safely batch
            /// because non-determinism is resolved when we sort by slots
        ))
        .chain(r_buffered_payloads) // Combine with all payloads that we've received and not processed yet
        .sort();
    // Create a cycle since we'll use this seq before we define it
    let (r_next_slot_complete_cycle, r_next_slot) =
        replica_tick.cycle_with_initial(replica_tick.singleton(q!(0)));
    // Find highest the sequence number of any payload that can be processed in this tick. This is the payload right before a hole.
    let r_next_slot_after_processing_payloads = r_sorted_payloads
        .clone()
        .cross_singleton(r_next_slot.clone())
        .fold(
            q!(|| 0),
            q!(|new_next_slot, (sorted_payload, next_slot)| {
                if sorted_payload.seq == std::cmp::max(*new_next_slot, next_slot) {
                    *new_next_slot = sorted_payload.seq + 1;
                }
            }),
        );
    // Find all payloads that can and cannot be processed in this tick.
    let r_processable_payloads = r_sorted_payloads
        .clone()
        .cross_singleton(r_next_slot_after_processing_payloads.clone())
        .filter(q!(
            |(sorted_payload, highest_seq)| sorted_payload.seq < *highest_seq
        ))
        .map(q!(|(sorted_payload, _)| { sorted_payload }));
    let r_new_non_processable_payloads = r_sorted_payloads
        .clone()
        .cross_singleton(r_next_slot_after_processing_payloads.clone())
        .filter(q!(
            |(sorted_payload, highest_seq)| sorted_payload.seq > *highest_seq
        ))
        .map(q!(|(sorted_payload, _)| { sorted_payload }));
    // Save these, we can process them once the hole has been filled
    r_buffered_payloads_complete_cycle.complete_next_tick(r_new_non_processable_payloads);

    let r_kv_store = r_processable_payloads
        .clone()
        .persist() // Optimization: all_ticks() + fold() = fold<static>, where the state of the previous fold is saved and persisted values are deleted.
        .fold(q!(|| (HashMap::new(), 0)), q!(|(kv_store, next_slot), payload| {
            if let Some(kv) = payload.kv {
                kv_store.insert(kv.key, kv.value);
            }
            *next_slot = payload.seq + 1;
        }));
    // Update the highest seq for the next tick
    r_next_slot_complete_cycle
        .complete_next_tick(r_kv_store.map(q!(|(_kv_store, next_slot)| next_slot)));

    // Send checkpoints to the acceptors when we've processed enough payloads
    let (r_checkpointed_seqs_complete_cycle, r_checkpointed_seqs) =
        replica_tick.cycle::<Optional<usize, _, _>>();
    let r_max_checkpointed_seq = r_checkpointed_seqs
        .into_stream()
        .persist()
        .max()
        .into_singleton();
    let r_checkpoint_seq_new = r_max_checkpointed_seq
        .zip(r_next_slot)
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
