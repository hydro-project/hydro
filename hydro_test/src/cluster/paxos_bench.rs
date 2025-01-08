use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use hydro_lang::*;
use hydro_std::quorum::collect_quorum;
use tokio::time::Instant;

use super::paxos::{Acceptor, Ballot, Proposer};
use super::paxos_kv::{paxos_kv, KvPayload, Replica};

pub struct Client {}

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
    let proposers = flow.cluster::<Proposer>();
    let acceptors = flow.cluster::<Acceptor>();
    let clients = flow.cluster::<Client>();
    let replicas = flow.cluster::<Replica>();

    let (new_leader_elected_complete, new_leader_elected) =
        clients.forward_ref::<Stream<_, _, _, NoOrder>>();

    let client_tick = clients.tick();
    let cur_leader_id = new_leader_elected
        .inspect(q!(|ballot| println!(
            "Client notified that leader was elected: {:?}",
            ballot
        )))
        .max()
        .map(q!(|ballot: Ballot| ballot.proposer_id));

    let leader_changed = unsafe {
        // SAFETY: we are okay if we miss a transient leader ID, because we
        // will eventually get the latest one and can restart requests then
        cur_leader_id
            .clone()
            .timestamped(&client_tick)
            .latest_tick()
            .delta()
            .map(q!(|_| ()))
            .all_ticks()
            .drop_timestamp()
    };

    bench_client(
        &clients,
        leader_changed,
        |c_to_proposers| {
            let to_proposers = unsafe {
                // SAFETY: the risk here is that we send a batch of requests
                // with a stale leader ID, but because the leader ID comes from the
                // network there is no way to guarantee that it is up to date

                // TODO(shadaj): we should retry if we get an error due to sending
                // to a stale leader
                c_to_proposers
                    .timestamped(&client_tick)
                    .tick_batch()
                    .cross_singleton(cur_leader_id.timestamped(&client_tick).latest_tick())
                    .all_ticks()
            }
            .map(q!(move |((key, value), leader_id)| (
                leader_id,
                KvPayload {
                    key,
                    // we use our ID as part of the value and use that so the replica only notifies us
                    value: (CLUSTER_SELF_ID, value)
                }
            )))
            .send_bincode_interleaved(&proposers);

            let to_proposers = unsafe {
                // SAFETY: clients "own" certain keys, so interleaving elements from clients will not affect
                // the order of writes to the same key
                to_proposers.assume_ordering()
            };

            let (new_leader_elected, processed_payloads) = unsafe {
                // SAFETY: Non-deterministic leader notifications are handled in `to_proposers`. We do not
                // care about the order in which key writes are processed, which is the non-determinism in
                // `processed_payloads`.
                paxos_kv(
                    &proposers,
                    &acceptors,
                    &replicas,
                    to_proposers,
                    f,
                    i_am_leader_send_timeout,
                    i_am_leader_check_timeout,
                    i_am_leader_check_timeout_delay_multiplier,
                    checkpoint_frequency,
                )
            };

            new_leader_elected_complete
                .complete(new_leader_elected.broadcast_bincode_interleaved(&clients));

            let c_received_payloads = processed_payloads
                .map(q!(|payload| (
                    payload.value.0,
                    ((payload.key, payload.value.1), Ok(()))
                )))
                .send_bincode_interleaved(&clients);

            // we only mark a transaction as committed when all replicas have applied it
            let (c_quorum_payloads, _) = collect_quorum::<_, _, _, ()>(
                c_received_payloads.timestamped(&client_tick),
                f + 1,
                f + 1,
            );

            c_quorum_payloads.drop_timestamp()
        },
        num_clients_per_node,
        median_latency_window_size,
    );

    (proposers, acceptors, clients, replicas)
}

fn bench_client<'a>(
    clients: &Cluster<'a, Client>,
    trigger_restart: Stream<(), Cluster<'a, Client>, Unbounded>,
    transaction_cycle: impl FnOnce(
        Stream<(u32, u32), Cluster<'a, Client>, Unbounded>,
    ) -> Stream<(u32, u32), Cluster<'a, Client>, Unbounded, NoOrder>,
    num_clients_per_node: usize,
    median_latency_window_size: usize,
) {
    let client_tick = clients.tick();
    // r_to_clients_payload_applied.clone().inspect(q!(|payload: &(u32, ReplicaPayload)| println!("Client received payload: {:?}", payload)));

    // Whenever the leader changes, make all clients send a message
    let restart_this_tick = unsafe {
        // SAFETY: non-deterministic delay in restarting requests
        // is okay because once it is restarted statistics should reach
        // steady state regardless of when the restart happes
        trigger_restart
            .timestamped(&client_tick)
            .tick_batch()
            .last()
    };

    let c_new_payloads_when_restart = restart_this_tick.clone().flat_map_ordered(q!(move |_| (0
        ..num_clients_per_node)
        .map(move |i| (
            (CLUSTER_SELF_ID.raw_id * (num_clients_per_node as u32)) + i as u32,
            0
        ))));

    let (c_to_proposers_complete_cycle, c_to_proposers) =
        clients.forward_ref::<Stream<_, _, _, TotalOrder>>();
    let c_received_quorum_payloads = unsafe {
        // SAFETY: because the transaction processor is required to handle arbitrary reordering
        // across *different* keys, we are safe because delaying a transaction result for a key
        // will only affect when the next request for that key is emitted with respect to other
        // keys
        transaction_cycle(c_to_proposers)
            .timestamped(&client_tick)
            .tick_batch()
    };

    // Whenever all replicas confirm that a payload was committed, send another payload
    let c_new_payloads_when_committed = c_received_quorum_payloads
        .clone()
        .map(q!(|payload| (payload.0, payload.1 + 1)));
    c_to_proposers_complete_cycle.complete(
        c_new_payloads_when_restart
            .chain(unsafe {
                // SAFETY: we don't send a new write for the same key until the previous one is committed,
                // so this contains only a single write per key, and we don't care about order
                // across keys
                c_new_payloads_when_committed.assume_ordering()
            })
            .all_ticks()
            .drop_timestamp(),
    );

    // Track statistics
    let (c_timers_complete_cycle, c_timers) =
        client_tick.cycle::<Stream<(usize, Instant), _, _, NoOrder>>();
    let c_new_timers_when_leader_elected = restart_this_tick
        .map(q!(|_| Instant::now()))
        .flat_map_ordered(q!(
            move |now| (0..num_clients_per_node).map(move |virtual_id| (virtual_id, now))
        ));
    let c_updated_timers = c_received_quorum_payloads
        .clone()
        .map(q!(|(key, _prev_count)| (key as usize, Instant::now())));
    let c_new_timers = c_timers
        .clone() // Update c_timers in tick+1 so we can record differences during this tick (to track latency)
        .union(c_new_timers_when_leader_elected)
        .union(c_updated_timers.clone())
        .reduce_keyed_commutative(q!(|curr_time, new_time| {
            if new_time > *curr_time {
                *curr_time = new_time;
            }
        }));
    c_timers_complete_cycle.complete_next_tick(c_new_timers);

    let c_stats_output_timer = unsafe {
        // SAFETY: intentionally sampling statistics
        clients
            .source_interval(q!(Duration::from_secs(1)))
            .timestamped(&client_tick)
            .tick_batch()
    }
    .first();

    let c_latency_reset = c_stats_output_timer.clone().map(q!(|_| None)).defer_tick();

    let c_latencies = c_timers
        .join(c_updated_timers)
        .map(q!(|(_virtual_id, (prev_time, curr_time))| Some(
            curr_time.duration_since(prev_time)
        )))
        .union(c_latency_reset.into_stream())
        .all_ticks()
        .flatten_ordered()
        .fold_commutative(
            // Create window with ring buffer using vec + wraparound index
            // TODO: Would be nice if I could use vec![] instead, but that doesn't work in Hydro with RuntimeData *median_latency_window_size
            q!(move || (
                Rc::new(RefCell::new(Vec::<Duration>::with_capacity(
                    median_latency_window_size
                ))),
                0usize,
            )),
            q!(move |(latencies, write_index), latency| {
                let mut latencies_mut = latencies.borrow_mut();
                if *write_index < latencies_mut.len() {
                    latencies_mut[*write_index] = latency;
                } else {
                    latencies_mut.push(latency);
                }
                // Increment write index and wrap around
                *write_index = (*write_index + 1) % median_latency_window_size;
            }),
        )
        .map(q!(|(latencies, _)| latencies));

    let c_throughput_new_batch = c_received_quorum_payloads
        .clone()
        .count()
        .continue_unless(c_stats_output_timer.clone())
        .map(q!(|batch_size| (batch_size, false)));

    let c_throughput_reset = c_stats_output_timer
        .clone()
        .map(q!(|_| (0, true)))
        .defer_tick();

    let c_throughput = c_throughput_new_batch
        .union(c_throughput_reset)
        .all_ticks()
        .fold(
            q!(|| 0),
            q!(|total, (batch_size, reset)| {
                if reset {
                    *total = 0;
                } else {
                    *total += batch_size;
                }
            }),
        );

    unsafe {
        // SAFETY: intentionally sampling statistics
        c_latencies.zip(c_throughput).latest_tick()
    }
    .continue_if(c_stats_output_timer)
    .all_ticks()
    .for_each(q!(move |(latencies, throughput)| {
        let mut latencies_mut = latencies.borrow_mut();
        if latencies_mut.len() > 0 {
            let middle_idx = latencies_mut.len() / 2;
            let (_, median, _) = latencies_mut.select_nth_unstable(middle_idx);
            println!("Median latency: {}ms", median.as_micros() as f64 / 1000.0);
        }

        println!("Throughput: {} requests/s", throughput);
    }));
    // End track statistics
}

#[cfg(test)]
mod tests {
    use hydro_lang::deploy::DeployRuntime;
    use stageleft::RuntimeData;

    #[test]
    fn paxos_ir() {
        let builder = hydro_lang::FlowBuilder::new();
        let _ = super::paxos_bench(&builder, 1, 1, 1, 1, 1, 1, 1);
        let built = builder.with_default_optimize::<DeployRuntime>();

        hydro_lang::ir::dbg_dedup_tee(|| {
            insta::assert_debug_snapshot!(built.ir());
        });

        let _ = built.compile(&RuntimeData::new("FAKE"));
    }
}
