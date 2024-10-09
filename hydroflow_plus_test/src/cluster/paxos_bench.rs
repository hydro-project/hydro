use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::{Duration, SystemTime};

use hydroflow_plus::*;
use serde::{Deserialize, Serialize};
use stageleft::*;

use super::paxos::{paxos_core, Acceptor, Ballot, PaxosPayload, Proposer};

pub trait LeaderElected: Ord + Clone {
    fn leader_id(&self) -> ClusterId<Proposer>;
}

pub struct Replica {}

pub struct Client {}

#[derive(Serialize, serde::Deserialize, PartialEq, Eq, PartialOrd, Ord, Clone, Debug)]
pub struct ReplicaPayload {
    // Note: Important that seq is the first member of the struct for sorting
    pub seq: i32,
    pub key: u32,
    pub value: String,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
pub struct ClientPayload {
    pub key: u32,
    pub value: String,
}

impl Default for ClientPayload {
    fn default() -> Self {
        Self {
            key: 0,
            value: "".to_string(),
        }
    }
}

impl PaxosPayload for ClientPayload {}

// Important: By convention, all relations that represent booleans either have a single "true" value or nothing.
// This allows us to use the continue_if_exists() and continue_if_empty() operators as if they were if (true) and if (false) statements.
#[expect(clippy::too_many_arguments, reason = "internal paxos code // TODO")]
pub fn paxos_bench<'a>(
    flow: &FlowBuilder<'a>,
    f: usize,
    num_clients_per_node: usize,
    median_latency_window_size: usize, /* How many latencies to keep in the window for calculating the median */
    checkpoint_frequency: usize,       // How many sequence numbers to commit before checkpointing
    i_am_leader_send_timeout: u64,     // How often to heartbeat
    i_am_leader_check_timeout: u64,    // How often to check if heartbeat expired
    i_am_leader_check_timeout_delay_multiplier: usize, /* Initial delay, multiplied by proposer pid, to stagger proposers checking for timeouts */
) -> (
    Cluster<'a, Proposer>,
    Cluster<'a, Acceptor>,
    Cluster<'a, Client>,
    Cluster<'a, Replica>,
) {
    let clients = flow.cluster::<Client>();
    let replicas = flow.cluster::<Replica>();

    let (c_to_proposers_complete_cycle, c_to_proposers) = clients.cycle();

    let (proposers, acceptors, p_to_clients_new_leader_elected, r_new_processed_payloads) =
        paxos_with_replica(
            flow,
            &replicas,
            c_to_proposers,
            f,
            i_am_leader_send_timeout,
            i_am_leader_check_timeout,
            i_am_leader_check_timeout_delay_multiplier,
            checkpoint_frequency,
        );

    c_to_proposers_complete_cycle.complete(bench_client(
        &clients,
        p_to_clients_new_leader_elected.broadcast_bincode_interleaved(&clients),
        r_new_processed_payloads.send_bincode(&clients),
        num_clients_per_node,
        median_latency_window_size,
        f,
    ));

    (proposers, acceptors, clients, replicas)
}

#[expect(
    clippy::type_complexity,
    clippy::too_many_arguments,
    reason = "internal paxos code // TODO"
)]
fn paxos_with_replica<'a>(
    flow: &FlowBuilder<'a>,
    replicas: &Cluster<'a, Replica>,
    c_to_proposers: Stream<
        (ClusterId<Proposer>, ClientPayload),
        Unbounded,
        NoTick,
        Cluster<'a, Client>,
    >,
    f: usize,
    i_am_leader_send_timeout: u64,
    i_am_leader_check_timeout: u64,
    i_am_leader_check_timeout_delay_multiplier: usize,
    checkpoint_frequency: usize,
) -> (
    Cluster<'a, Proposer>,
    Cluster<'a, Acceptor>,
    Stream<Ballot, Unbounded, NoTick, Cluster<'a, Proposer>>,
    Stream<(ClusterId<Client>, ReplicaPayload), Unbounded, NoTick, Cluster<'a, Replica>>,
) {
    let (r_to_acceptors_checkpoint_complete_cycle, r_to_acceptors_checkpoint) =
        replicas.cycle::<Stream<_, _, _, _>>();

    let (proposers, acceptors, p_to_clients_new_leader_elected, p_to_replicas) = paxos_core(
        flow,
        |acceptors| r_to_acceptors_checkpoint.broadcast_bincode(acceptors),
        |proposers| c_to_proposers.send_bincode_interleaved(proposers),
        f,
        i_am_leader_send_timeout,
        i_am_leader_check_timeout,
        i_am_leader_check_timeout_delay_multiplier,
    );

    let (r_to_acceptors_checkpoint_new, r_new_processed_payloads) = replica(
        replicas,
        p_to_replicas
            .map(q!(|(slot, data)| ReplicaPayload {
                seq: slot,
                key: data.key,
                value: data.value
            }))
            .broadcast_bincode_interleaved(replicas),
        checkpoint_frequency,
    );

    r_to_acceptors_checkpoint_complete_cycle.complete(r_to_acceptors_checkpoint_new);

    (
        proposers,
        acceptors,
        p_to_clients_new_leader_elected,
        r_new_processed_payloads,
    )
}

// Replicas. All relations for replicas will be prefixed with r. Expects ReplicaPayload on p_to_replicas, outputs a stream of (client address, ReplicaPayload) after processing.
#[expect(clippy::type_complexity, reason = "internal paxos code // TODO")]
pub fn replica<'a>(
    replicas: &Cluster<'a, Replica>,
    p_to_replicas: Stream<ReplicaPayload, Unbounded, NoTick, Cluster<'a, Replica>>,
    checkpoint_frequency: usize,
) -> (
    Stream<i32, Unbounded, NoTick, Cluster<'a, Replica>>,
    Stream<(ClusterId<Client>, ReplicaPayload), Unbounded, NoTick, Cluster<'a, Replica>>,
) {
    let (r_buffered_payloads_complete_cycle, r_buffered_payloads) = replicas.tick_cycle();
    // p_to_replicas.inspect(q!(|payload: ReplicaPayload| println!("Replica received payload: {:?}", payload)));
    let r_sorted_payloads = p_to_replicas
        .clone()
        .tick_batch()
        .union(r_buffered_payloads) // Combine with all payloads that we've received and not processed yet
        .sort();
    // Create a cycle since we'll use this seq before we define it
    let (r_highest_seq_complete_cycle, r_highest_seq) =
        replicas.tick_cycle::<Optional<i32, _, _, _>>();
    let empty_slot = replicas.singleton_first_tick(q!(-1));
    // Either the max sequence number executed so far or -1. Need to union otherwise r_highest_seq is empty and joins with it will fail
    let r_highest_seq_with_default = r_highest_seq.union(empty_slot);
    // Find highest the sequence number of any payload that can be processed in this tick. This is the payload right before a hole.
    let r_highest_seq_processable_payload = r_sorted_payloads
        .clone()
        .cross_singleton(r_highest_seq_with_default)
        .fold(
            q!(|| -1),
            q!(|filled_slot, (sorted_payload, highest_seq)| {
                // Note: This function only works if the input is sorted on seq.
                let next_slot = std::cmp::max(*filled_slot, highest_seq);

                *filled_slot = if sorted_payload.seq == next_slot + 1 {
                    sorted_payload.seq
                } else {
                    *filled_slot
                };
            }),
        );
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

    let r_kv_store = r_processable_payloads
        .clone()
        .persist() // Optimization: all_ticks() + fold() = fold<static>, where the state of the previous fold is saved and persisted values are deleted.
        .fold(q!(|| (HashMap::<u32, String>::new(), -1)), q!(|state, payload| {
            let kv_store = &mut state.0;
            let last_seq = &mut state.1;
            kv_store.insert(payload.key, payload.value);
            debug_assert!(payload.seq == *last_seq + 1, "Hole in log between seq {} and {}", *last_seq, payload.seq);
            *last_seq = payload.seq;
            // println!("Replica kv store: {:?}", kv_store);
        }));
    // Update the highest seq for the next tick
    let r_new_highest_seq = r_kv_store.map(q!(|(_kv_store, highest_seq)| highest_seq));
    r_highest_seq_complete_cycle.complete_next_tick(r_new_highest_seq.clone().into());

    // Send checkpoints to the acceptors when we've processed enough payloads
    let (r_checkpointed_seqs_complete_cycle, r_checkpointed_seqs) =
        replicas.tick_cycle::<Optional<i32, _, _, _>>();
    let r_max_checkpointed_seq = r_checkpointed_seqs
        .persist()
        .max()
        .unwrap_or(replicas.singleton(q!(-1)).latest_tick());
    let r_checkpoint_seq_new = r_max_checkpointed_seq
        .cross_singleton(r_new_highest_seq)
        .filter_map(q!(
            move |(max_checkpointed_seq, new_highest_seq)| if new_highest_seq - max_checkpointed_seq
                >= checkpoint_frequency as i32
            {
                Some(new_highest_seq)
            } else {
                None
            }
        ));
    r_checkpointed_seqs_complete_cycle.complete_next_tick(r_checkpoint_seq_new.clone());

    // Tell clients that the payload has been committed. All ReplicaPayloads contain the client's machine ID (to string) as value.
    let r_to_clients = p_to_replicas.map(q!(|payload| (
        ClusterId::from_raw(payload.value.parse::<u32>().unwrap()),
        payload
    )));
    (r_checkpoint_seq_new.all_ticks(), r_to_clients)
}

// Clients. All relations for clients will be prefixed with c. All ClientPayloads will contain the virtual client number as key and the client's machine ID (to string) as value. Expects p_to_clients_leader_elected containing Ballots whenever the leader is elected, and r_to_clients_payload_applied containing ReplicaPayloads whenever a payload is committed. Outputs (leader address, ClientPayload) when a new leader is elected or when the previous payload is committed.
fn bench_client<'a, B: LeaderElected + std::fmt::Debug>(
    clients: &Cluster<'a, Client>,
    p_to_clients_leader_elected: Stream<B, Unbounded, NoTick, Cluster<'a, Client>>,
    r_to_clients_payload_applied: Stream<
        (ClusterId<Replica>, ReplicaPayload),
        Unbounded,
        NoTick,
        Cluster<'a, Client>,
    >,
    num_clients_per_node: usize,
    median_latency_window_size: usize,
    f: usize,
) -> Stream<(ClusterId<Proposer>, ClientPayload), Unbounded, NoTick, Cluster<'a, Client>> {
    let c_id = clients.self_id();
    // r_to_clients_payload_applied.clone().inspect(q!(|payload: &(u32, ReplicaPayload)| println!("Client received payload: {:?}", payload)));
    // Only keep the latest leader
    let c_max_leader_ballot = p_to_clients_leader_elected
        .inspect(q!(|ballot| println!(
            "Client notified that leader was elected: {:?}",
            ballot
        )))
        .max();
    let c_new_leader_ballot = c_max_leader_ballot.clone().latest_tick().delta();
    // Whenever the leader changes, make all clients send a message
    let c_new_payloads_when_leader_elected =
        c_new_leader_ballot
            .clone()
            .flat_map(q!(move |leader_ballot| (0..num_clients_per_node).map(
                move |i| (
                    leader_ballot.leader_id(),
                    ClientPayload {
                        key: i as u32,
                        value: c_id.raw_id.to_string()
                    }
                )
            )));
    // Whenever replicas confirm that a payload was committed, collected it and wait for a quorum
    let (c_pending_quorum_payloads_complete_cycle, c_pending_quorum_payloads) =
        clients.tick_cycle();
    let c_received_payloads = r_to_clients_payload_applied
        .tick_batch()
        .map(q!(|(sender, replica_payload)| (
            replica_payload.key,
            sender
        )))
        .union(c_pending_quorum_payloads);
    let c_received_quorum_payloads = c_received_payloads
        .clone()
        .fold_keyed(
            q!(|| 0),
            q!(|curr_count, _sender| {
                *curr_count += 1; // Assumes the same replica will only send commit once
            }),
        )
        .filter_map(q!(move |(key, count)| {
            if count == f + 1 {
                Some(key)
            } else {
                None
            }
        }));
    let c_new_pending_quorum_payloads =
        c_received_payloads.anti_join(c_received_quorum_payloads.clone());
    c_pending_quorum_payloads_complete_cycle.complete_next_tick(c_new_pending_quorum_payloads);
    // Whenever all replicas confirm that a payload was committed, send another payload
    let c_new_payloads_when_committed = c_received_quorum_payloads
        .clone()
        .cross_singleton(c_max_leader_ballot.clone().latest_tick())
        .map(q!(move |(key, leader_ballot)| (
            leader_ballot.leader_id(),
            ClientPayload {
                key,
                value: c_id.raw_id.to_string()
            }
        )));
    let c_to_proposers = c_new_payloads_when_leader_elected
        .union(c_new_payloads_when_committed)
        .all_ticks();

    // Track statistics
    let (c_timers_complete_cycle, c_timers) =
        clients.tick_cycle::<Stream<(usize, SystemTime), _, _, _>>();
    let c_new_timers_when_leader_elected = c_new_leader_ballot
        .map(q!(|_| SystemTime::now()))
        .flat_map(q!(
            move |now| (0..num_clients_per_node).map(move |virtual_id| (virtual_id, now))
        ));
    let c_updated_timers = c_received_quorum_payloads
        .clone()
        .map(q!(|key| (key as usize, SystemTime::now())));
    let c_new_timers = c_timers
        .clone() // Update c_timers in tick+1 so we can record differences during this tick (to track latency)
        .union(c_new_timers_when_leader_elected)
        .union(c_updated_timers.clone())
        .reduce_keyed(q!(|curr_time, new_time| {
            if new_time > *curr_time {
                *curr_time = new_time;
            }
        }));
    c_timers_complete_cycle.complete_next_tick(c_new_timers);

    let c_stats_output_timer = clients.source_interval(q!(Duration::from_secs(1)));

    let c_latency_reset = c_stats_output_timer
        .clone()
        .latest_tick()
        .map(q!(|_| None))
        .defer_tick();

    let c_latencies = c_timers
        .join(c_updated_timers)
        .map(q!(|(_virtual_id, (prev_time, curr_time))| Some(
            curr_time.duration_since(prev_time).unwrap().as_micros()
        )))
        .union(c_latency_reset.into_stream())
        .all_ticks()
        .fold(
            // Create window with ring buffer using vec + wraparound index
            // TODO: Would be nice if I could use vec![] instead, but that doesn't work in HF+ with RuntimeData *median_latency_window_size
            q!(move || (
                Rc::new(RefCell::new(Vec::<u128>::with_capacity(
                    median_latency_window_size
                ))),
                0usize,
                false
            )),
            q!(move |(latencies, write_index, has_any_value), latency| {
                let mut latencies_mut = latencies.borrow_mut();
                if let Some(latency) = latency {
                    // Insert into latencies
                    if let Some(prev_latency) = latencies_mut.get_mut(*write_index) {
                        *prev_latency = latency;
                    } else {
                        latencies_mut.push(latency);
                    }
                    *has_any_value = true;
                    // Increment write index and wrap around
                    *write_index += 1;
                    if *write_index == median_latency_window_size {
                        *write_index = 0;
                    }
                } else {
                    // reset latencies
                    latencies_mut.clear();
                    *write_index = 0;
                    *has_any_value = false;
                }
            }),
        );
    let c_throughput_new_batch = c_received_quorum_payloads
        .clone()
        .count()
        .continue_unless(c_stats_output_timer.clone().latest_tick())
        .map(q!(|batch_size| (batch_size, false)));

    let c_throughput_reset = c_stats_output_timer
        .clone()
        .latest_tick()
        .map(q!(|_| (0, true)))
        .defer_tick();

    let c_throughput = c_throughput_new_batch
        .union(c_throughput_reset)
        .all_ticks()
        .fold(
            q!(|| (0, 0)),
            q!(|(total, num_ticks), (batch_size, reset)| {
                if reset {
                    *total = 0;
                    *num_ticks = 0;
                } else {
                    *total += batch_size as u32;
                    *num_ticks += 1;
                }
            }),
        );

    c_stats_output_timer
        .cross_singleton(c_latencies)
        .cross_singleton(c_throughput)
        .tick_samples()
        .for_each(q!(move |(
            (_, (latencies, _write_index, has_any_value)),
            (throughput, num_ticks),
        )| {
            let mut latencies_mut = latencies.borrow_mut();
            let median_latency = if has_any_value {
                let (_, median, _) =
                    latencies_mut.select_nth_unstable(median_latency_window_size / 2);
                *median
            } else {
                0
            };
            println!("Median latency: {}ms", median_latency as f64 / 1000.0);
            println!("Throughput: {} requests/s", throughput);
            println!("Num ticks per second: {}", num_ticks);
        }));
    // End track statistics
    c_to_proposers
}

#[cfg(test)]
mod tests {
    use hydroflow_plus::deploy::DeployRuntime;
    use stageleft::RuntimeData;

    #[test]
    fn paxos_ir() {
        let builder = hydroflow_plus::FlowBuilder::new();
        let _ = super::paxos_bench(&builder, 1, 1, 1, 1, 1, 1, 1);
        let built = builder.with_default_optimize();

        hydroflow_plus::ir::dbg_dedup_tee(|| {
            insta::assert_debug_snapshot!(built.ir());
        });

        let _ = built.compile::<DeployRuntime>(&RuntimeData::new("FAKE"));
    }
}
