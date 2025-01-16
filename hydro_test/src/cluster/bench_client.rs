use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use hydro_lang::*;
use tokio::time::Instant;

pub struct Client {}

pub fn bench_client<'a>(
    clients: &Cluster<'a, Client>,
    transaction_cycle: impl FnOnce(
        Stream<(u32, u32), Cluster<'a, Client>, Unbounded>,
    ) -> Stream<(u32, u32), Cluster<'a, Client>, Unbounded, NoOrder>,
    num_clients_per_node: usize,
    median_latency_window_size: usize,
) {
    let client_tick = clients.tick();
    // r_to_clients_payload_applied.clone().inspect(q!(|payload: &(u32, ReplicaPayload)| println!("Client received payload: {:?}", payload)));

    // Set up an initial set of payloads on the first tick
    let start_this_tick = client_tick.singleton_first_tick(q!(()));

    let c_new_payloads_on_start = start_this_tick.clone().flat_map_ordered(q!(move |_| (0
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
        c_new_payloads_on_start
            .chain(unsafe {
                // SAFETY: we don't send a new write for the same key until the previous one is committed,
                // so this contains only a single write per key, and we don't care about order
                // across keys
                c_new_payloads_when_committed.assume_ordering::<TotalOrder>()
            })
            .all_ticks()
            .drop_timestamp(),
    );

    // Track statistics
    let (c_timers_complete_cycle, c_timers) =
        client_tick.cycle::<Stream<(usize, Instant), _, _, NoOrder>>();
    let c_new_timers_when_leader_elected = start_this_tick
        .map(q!(|_| Instant::now()))
        .flat_map_ordered(q!(
            move |now| (0..num_clients_per_node).map(move |virtual_id| (virtual_id, now))
        ));
    let c_updated_timers = c_received_quorum_payloads
        .clone()
        .map(q!(|(key, _prev_count)| (key as usize, Instant::now())));
    let c_new_timers = c_timers
        .clone() // Update c_timers in tick+1 so we can record differences during this tick (to track latency)
        .chain(c_new_timers_when_leader_elected)
        .chain(c_updated_timers.clone())
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
        .chain(c_latency_reset.into_stream())
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
