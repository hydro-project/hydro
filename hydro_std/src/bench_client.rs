use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use hdrhistogram::Histogram;
use hydro_lang::*;
use stats_ci::mean::Arithmetic;
use stats_ci::{Confidence, StatisticsOps};

pub struct BenchResult<'a, Client> {
    pub latency_histogram: Singleton<Rc<RefCell<Histogram<u64>>>, Cluster<'a, Client>, Unbounded>,
    pub throughput: Singleton<Arithmetic<f64>, Cluster<'a, Client>, Unbounded>,
}

/// Benchmarks transactional workloads by concurrently submitting workloads
/// (up to `num_clients_per_node` per machine), measuring the latency
/// of each transaction and throughput over the entire workload.
///
/// # Safety
/// This function uses non-deterministic time-based samples, and also updates results
/// at non-deterministic points in time.
pub unsafe fn bench_client<'a, Client>(
    clients: &Cluster<'a, Client>,
    transaction_cycle: impl FnOnce(
        Stream<(u32, u32), Cluster<'a, Client>, Unbounded>,
    ) -> Stream<(u32, u32), Cluster<'a, Client>, Unbounded, NoOrder>,
    num_clients_per_node: usize,
) -> BenchResult<'a, Client> {
    let client_tick = clients.tick();

    // Set up an initial set of payloads on the first tick
    let start_this_tick = client_tick.optional_first_tick(q!(()));

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
        transaction_cycle(c_to_proposers).tick_batch(&client_tick)
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
            .all_ticks(),
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
            .tick_batch(&client_tick)
    }
    .first();

    let c_latencies = c_timers
        .join(c_updated_timers)
        .map(q!(
            |(_virtual_id, (prev_time, curr_time))| curr_time.duration_since(prev_time)
        ))
        .all_ticks()
        .fold_commutative(
            q!(move || Rc::new(RefCell::new(Histogram::<u64>::new(3).unwrap()))),
            q!(move |latencies, latency| {
                latencies
                    .borrow_mut()
                    .record(latency.as_nanos() as u64)
                    .unwrap();
            }),
        );

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
            q!(|| (0, { stats_ci::mean::Arithmetic::new() })),
            q!(|(total, stats), (batch_size, reset)| {
                if reset {
                    if *total > 0 {
                        stats.extend(&[*total as f64]).unwrap();
                    }

                    *total = 0;
                } else {
                    *total += batch_size;
                }
            }),
        )
        .map(q!(|(_, stats)| { stats }));

    BenchResult {
        latency_histogram: c_latencies,
        throughput: c_throughput,
    }
}

/// Prints transaction latency and throughput results to stdout,
/// with percentiles for latency and a confidence interval for throughput.
pub fn print_bench_results<Client>(results: BenchResult<Client>) {
    unsafe {
        // SAFETY: intentional non-determinism
        results
            .latency_histogram
            .sample_every(q!(Duration::from_millis(1000)))
    }
    .for_each(q!(move |latencies| {
        let latencies = latencies.borrow();
        println!(
            "Latency p50: {:.3} | p99 {:.3} ms | p999 {:.3} ms ({:} samples)",
            Duration::from_nanos(latencies.value_at_quantile(0.5)).as_micros() as f64 / 1000.0,
            Duration::from_nanos(latencies.value_at_quantile(0.99)).as_micros() as f64 / 1000.0,
            Duration::from_nanos(latencies.value_at_quantile(0.999)).as_micros() as f64 / 1000.0,
            latencies.len()
        );
    }));

    unsafe {
        // SAFETY: intentional non-determinism
        results
            .throughput
            .sample_every(q!(Duration::from_millis(1000)))
    }
    .for_each(q!(move |throughputs| {
        let confidence = Confidence::new(0.99);

        if throughputs.sample_count() >= 2 {
            // ci_mean crashes if there are fewer than two samples
            if let Ok(interval) = throughputs.ci_mean(confidence) {
                if let Some(lower) = interval.left() {
                    if let Some(upper) = interval.right() {
                        println!(
                            "Throughput 99% interval: {:.2} - {:.2} requests/s",
                            lower, upper
                        );
                    }
                }
            }
        }
    }));
}
