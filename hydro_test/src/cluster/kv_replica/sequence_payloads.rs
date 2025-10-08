use hydro_lang::forward_handle::TickCycleHandle;
use hydro_lang::live_collections::stream::NoOrder;
use hydro_lang::location::{Location, NoTick};
use hydro_lang::prelude::*;

use super::{KvKey, KvValue, SequencedKv};

#[expect(clippy::type_complexity, reason = "Paxos internals")]
pub fn sequence_payloads<'a, K: KvKey, V: KvValue, L: Location<'a> + NoTick>(
    replica_tick: &Tick<L>,
    p_to_replicas: Stream<SequencedKv<K, V>, L, Unbounded, NoOrder>,
) -> (
    Stream<SequencedKv<K, V>, Tick<L>, Bounded>,
    TickCycleHandle<'a, Singleton<usize, Tick<L>, Bounded>>,
) {
    let (r_buffered_payloads_complete_cycle, r_buffered_payloads) =
        replica_tick.cycle::<Stream<SequencedKv<K, V>, Tick<L>, Bounded>>();
    // p_to_replicas.inspect(q!(|payload: ReplicaPayload| println!("Replica received payload: {:?}", payload)));
    let r_sorted_payloads = p_to_replicas.batch(replica_tick, nondet!(
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

    (r_processable_payloads, r_next_slot_complete_cycle)
}

#[cfg(test)]
mod tests {
    use hydro_lang::prelude::*;

    use super::super::SequencedKv;
    use super::*;

    #[test]
    fn sequence_payloads_sequences_all() {
        let flow = FlowBuilder::new();
        let external = flow.external::<()>();
        let node = flow.process::<()>();
        let tick = node.tick();

        let (input_port, input_payloads) = node.source_external_bincode(&external);
        let (sequenced, complete_next_slot) =
            sequence_payloads(&tick, input_payloads.weaken_ordering());

        complete_next_slot.complete_next_tick(sequenced.clone()
            .persist() // Optimization: all_ticks() + fold() = fold<static>, where the state of the previous fold is saved and persisted values are deleted.
            .fold(q!(|| 0), q!(|next_slot, payload: SequencedKv<(), ()>| {
                *next_slot = payload.seq + 1;
            })));

        let out_port = sequenced.all_ticks().send_bincode_external(&external);

        flow.sim().exhaustive(async |mut compiled| {
            let in_send = compiled.connect(&input_port);
            let out_recv = compiled.connect(&out_port);
            compiled.launch();

            in_send.send(SequencedKv { seq: 0, kv: None }).unwrap();
            in_send.send(SequencedKv { seq: 1, kv: None }).unwrap();
            in_send.send(SequencedKv { seq: 2, kv: None }).unwrap();
            in_send.send(SequencedKv { seq: 3, kv: None }).unwrap();

            let all_out = out_recv.collect::<Vec<_>>().await;
            assert_eq!(
                all_out,
                vec![
                    SequencedKv { seq: 0, kv: None },
                    SequencedKv { seq: 1, kv: None },
                    SequencedKv { seq: 2, kv: None },
                    SequencedKv { seq: 3, kv: None },
                ]
            );
        });
    }
}
