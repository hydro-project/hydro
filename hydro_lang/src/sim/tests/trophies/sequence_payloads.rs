//! Originally discovered and fixed in https://github.com/hydro-project/hydro/pull/1701.
//!
//! The original implementation of `sequence_payloads` for the KV replica in Paxos had a subtle
//! bug that causes it to silently drop buffered payloads when they are received out of order.
//! This implementation folds the payloads that are ready to be processed to track the highest
//! sequence number that can be processed, and then filters the payloads based on that. In ticks
//! where _at least one_ payload is ready to be processed, the result of this fold will be
//! non-None.
//!
//! But in a tick where no payloads are ready to be processed, the result of the fold will be
//! None, and so the `cross_singleton` that produces `r_new_non_processable_payloads` will drop
//! all of the buffered payloads.
//!
//! The comments in the code are from the original implementation before the bug was discovered.

use serde::{Deserialize, Serialize};

use crate::forward_handle::TickCycleHandle;
use crate::live_collections::stream::NoOrder;
use crate::location::{Location, NoTick};
use crate::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Clone, Debug)]
struct SequencedKv {
    seq: usize,
}

#[expect(clippy::type_complexity, reason = "Paxos internals")]
fn sequence_payloads_old<'a, L: Location<'a> + NoTick>(
    replica_tick: &Tick<L>,
    p_to_replicas: Stream<SequencedKv, L, Unbounded, NoOrder>,
) -> (
    Stream<SequencedKv, Tick<L>, Bounded>,
    TickCycleHandle<'a, Optional<usize, Tick<L>, Bounded>>,
) {
    let (r_buffered_payloads_complete_cycle, r_buffered_payloads) =
        replica_tick.cycle::<Stream<SequencedKv, Tick<L>, Bounded>>();

    let r_sorted_payloads = p_to_replicas
        .batch(replica_tick, nondet!(
            /// because we fill slots one-by-one, we can safely batch
            /// because non-determinism is resolved when we sort by slots
        ))
        .chain(r_buffered_payloads) // Combine with all payloads that we've received and not processed yet
        .sort();
    // Create a cycle since we'll use this seq before we define it
    let (r_highest_seq_complete_cycle, r_highest_seq) =
        replica_tick.cycle::<Optional<usize, _, _>>();
    // Find highest the sequence number of any payload that can be processed in this tick. This is the payload right before a hole.
    let r_highest_seq_processable_payload = r_sorted_payloads
        .clone()
        .cross_singleton(r_highest_seq.into_singleton())
        .fold(
            q!(|| None),
            q!(|filled_slot, (sorted_payload, highest_seq)| {
                let expected_next_slot = std::cmp::max(
                    filled_slot.map(|v| v + 1).unwrap_or(0),
                    highest_seq.map(|v| v + 1).unwrap_or(0),
                );

                if sorted_payload.seq == expected_next_slot {
                    *filled_slot = Some(sorted_payload.seq);
                }
            }),
        )
        .filter_map(q!(|v| v));
    // Find all payloads that can and cannot be processed in this tick.
    let r_processable_payloads = r_sorted_payloads
        .clone()
        .cross_singleton(r_highest_seq_processable_payload.clone())
        .filter(q!(
            |(sorted_payload, highest_seq)| sorted_payload.seq <= *highest_seq
        ))
        .map(q!(|(sorted_payload, _)| { sorted_payload }));
    let r_new_non_processable_payloads = r_sorted_payloads
        .clone()
        .cross_singleton(r_highest_seq_processable_payload.clone())
        .filter(q!(
            |(sorted_payload, highest_seq)| sorted_payload.seq > *highest_seq
        ))
        .map(q!(|(sorted_payload, _)| { sorted_payload }));
    // Save these, we can process them once the hole has been filled
    r_buffered_payloads_complete_cycle.complete_next_tick(r_new_non_processable_payloads);
    (r_processable_payloads, r_highest_seq_complete_cycle)
}

#[test]
#[should_panic]
fn test() {
    // run as PATH="$PATH:." cargo sim -p hydro_lang --features sim -- trophies::sequence_payloads
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let tick = node.tick();

    let (in_send, input_payloads) = node.sim_input();
    let (sequenced, complete_next_slot) = sequence_payloads_old(&tick, input_payloads);

    // original behavior from the KV replica (unrelated to the bug)
    complete_next_slot.complete_next_tick(
        sequenced
            .clone()
            .across_ticks(|s| {
                s.fold(
                    q!(|| None),
                    q!(|next_slot, payload: SequencedKv| {
                        *next_slot = Some(payload.seq);
                    }),
                )
            })
            .filter_map(q!(|v| v)),
    );

    let out_recv = sequenced.all_ticks().sim_output();

    flow.sim().fuzz(async || {
        in_send.send_many_unordered([
            SequencedKv { seq: 0 },
            SequencedKv { seq: 1 },
            SequencedKv { seq: 2 },
            SequencedKv { seq: 3 },
        ]);

        out_recv
            .assert_yields_only([
                SequencedKv { seq: 0 },
                SequencedKv { seq: 1 },
                SequencedKv { seq: 2 },
                SequencedKv { seq: 3 },
            ])
            .await;
    });
}

#[test]
#[cfg_attr(target_os = "windows", ignore)] // trace locations don't work on Windows right now
fn trace_snapshot() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let tick = node.tick();

    let (in_send, input_payloads) = node.sim_input();
    let (sequenced, complete_next_slot) = sequence_payloads_old(&tick, input_payloads);

    // original behavior from the KV replica (unrelated to the bug)
    complete_next_slot.complete_next_tick(
        sequenced
            .clone()
            .across_ticks(|s| {
                s.fold(
                    q!(|| None),
                    q!(|next_slot, payload: SequencedKv| {
                        *next_slot = Some(payload.seq);
                    }),
                )
            })
            .filter_map(q!(|v| v)),
    );

    let out_recv = sequenced.all_ticks().sim_output();

    let repro_bytes = std::fs::read(
        "./src/sim/tests/trophies/sim-failures/hydro_lang__sim__tests__trophies__sequence_payloads__test.bin",
    )
    .unwrap();

    let mut log_out = Vec::new();
    colored::control::set_override(false);

    flow.sim()
        .compiled()
        .fuzz_repro(repro_bytes, async |compiled| {
            let schedule = compiled.schedule_with_logger(&mut log_out);
            let rest = async move {
                in_send.send_many_unordered([
                    SequencedKv { seq: 0 },
                    SequencedKv { seq: 1 },
                    SequencedKv { seq: 2 },
                    SequencedKv { seq: 3 },
                ]);

                let _all_out = out_recv.collect::<Vec<_>>().await;
            };

            tokio::select! {
                biased;
                _ = rest => {},
                _ = schedule => {},
            };
        });

    let log_str = String::from_utf8(log_out).unwrap();
    hydro_build_utils::assert_snapshot!(log_str);
}
