use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use hdrhistogram::Histogram;
use hdrhistogram::serialization::{Deserializer, Serializer, V2Serializer};
use hydro_lang::*;
use serde::{Deserialize, Serialize};

pub mod rolling_average;
use rolling_average::RollingAverage;

pub struct SerializableHistogramWrapper {
    pub histogram: Rc<RefCell<Histogram<u64>>>,
}

impl Serialize for SerializableHistogramWrapper {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut vec = Vec::new();
        V2Serializer::new()
            .serialize(&self.histogram.borrow(), &mut vec)
            .unwrap();
        serializer.serialize_bytes(&vec)
    }
}
impl<'a> Deserialize<'a> for SerializableHistogramWrapper {
    fn deserialize<D: serde::Deserializer<'a>>(deserializer: D) -> Result<Self, D::Error> {
        let mut bytes: &[u8] = Deserialize::deserialize(deserializer)?;
        let mut histogram = Deserializer::new().deserialize(&mut bytes).unwrap();
        // Allow auto-resizing to prevent error when combining
        histogram.auto(true);
        Ok(SerializableHistogramWrapper {
            histogram: Rc::new(RefCell::new(histogram)),
        })
    }
}
pub struct BenchResult<'a, Client> {
    pub latency_histogram: Singleton<Rc<RefCell<Histogram<u64>>>, Cluster<'a, Client>, Unbounded>,
    pub throughput: Singleton<RollingAverage, Cluster<'a, Client>, Unbounded>,
}

/// Benchmarks transactional workloads by concurrently submitting workloads
/// (up to `num_clients_per_node` per machine), measuring the latency
/// of each transaction and throughput over the entire workload.
/// * `workload_generator` - Generates a payload `P` for each virtual client
/// * `transaction_cycle` - Processes the payloads and returns after processing
///
/// # Non-Determinism
/// This function uses non-deterministic wall-clock windows for measuring throughput.
pub fn bench_client<'a, Client, Payload>(
    clients: &Cluster<'a, Client>,
    workload_generator: impl FnOnce(
        &Cluster<'a, Client>,
        Stream<(u32, Option<Payload>), Cluster<'a, Client>, Unbounded, NoOrder>,
    )
        -> Stream<(u32, Payload), Cluster<'a, Client>, Unbounded, NoOrder>,
    transaction_cycle: impl FnOnce(
        Stream<(u32, Payload), Cluster<'a, Client>, Unbounded>,
    )
        -> Stream<(u32, Payload), Cluster<'a, Client>, Unbounded, NoOrder>,
    num_clients_per_node: usize,
    nondet_throughput_window: NonDet,
) -> BenchResult<'a, Client>
where
    Payload: Clone,
{
    let client_tick = clients.tick();

    // Set up an initial set of payloads on the first tick
    let start_this_tick = client_tick.optional_first_tick(q!(()));

    let c_new_payloads_on_start = start_this_tick.clone().flat_map_ordered(q!(move |_| (0
        ..num_clients_per_node as u32)
        .map(move |virtual_id| { (virtual_id, None) })));

    let (c_to_proposers_complete_cycle, c_to_proposers) =
        clients.forward_ref::<Stream<_, _, _, TotalOrder>>();

    // Whenever all replicas confirm that a payload was committed, send another payload
    let c_received_quorum_payloads = transaction_cycle(c_to_proposers)
        .batch(
            &client_tick,
            nondet!(
                /// because the transaction processor is required to handle arbitrary reordering
                /// across *different* keys, we are safe because delaying a transaction result for a key
                /// will only affect when the next request for that key is emitted with respect to other keys
            ),
        )
        .map(q!(|(virtual_id, payload)| (virtual_id, Some(payload))));

    let c_new_payloads = workload_generator(
        clients,
        c_new_payloads_on_start
            .chain(c_received_quorum_payloads.clone())
            .all_ticks(),
    );
    c_to_proposers_complete_cycle.complete(c_new_payloads.assume_ordering::<TotalOrder>(nondet!(
        /// We don't send a new write for the same key until the previous one is committed,
        /// so this contains only a single write per key, and we don't care about order
        /// across keys.
    )));

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
        .map(q!(|(key, _payload)| (key as usize, Instant::now())));
    let c_new_timers = c_timers
        .clone() // Update c_timers in tick+1 so we can record differences during this tick (to track latency)
        .chain(c_new_timers_when_leader_elected)
        .chain(c_updated_timers.clone())
        .into_keyed()
        .reduce_commutative(q!(|curr_time, new_time| {
            if new_time > *curr_time {
                *curr_time = new_time;
            }
        }))
        .entries();
    c_timers_complete_cycle.complete_next_tick(c_new_timers);

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

    let c_stats_output_timer = clients
        .source_interval(q!(Duration::from_secs(1)), nondet_throughput_window)
        .batch(&client_tick, nondet_throughput_window)
        .first();

    let c_throughput_new_batch = c_received_quorum_payloads
        .count()
        .continue_unless(c_stats_output_timer.clone())
        .map(q!(|batch_size| (batch_size, false)));

    let c_throughput_reset = c_stats_output_timer.map(q!(|_| (0, true))).defer_tick();

    let c_throughput = c_throughput_new_batch
        .union(c_throughput_reset)
        .all_ticks()
        .fold(
            q!(|| (0, { RollingAverage::new() })),
            q!(|(total, stats), (batch_size, reset)| {
                if reset {
                    if *total > 0 {
                        stats.add_sample(*total as f64);
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
pub fn print_bench_results<'a, Client: 'a, Aggregator>(
    results: BenchResult<'a, Client>,
    aggregator: &Process<'a, Aggregator>,
    clients: &Cluster<'a, Client>,
) {
    let nondet_sampling = nondet!(/** non-deterministic samping only affects logging */);
    let keyed_latencies = results
        .latency_histogram
        .sample_every(q!(Duration::from_millis(1000)), nondet_sampling)
        .map(q!(|latencies| {
            SerializableHistogramWrapper {
                histogram: latencies,
            }
        }))
        .send_bincode(aggregator);

    let combined_latencies = keyed_latencies
        .map(q!(|histogram| histogram.histogram.borrow().clone()))
        .batch(&aggregator.tick(), nondet_sampling)
        .reduce_idempotent(q!(|combined, new| {
            *combined = new;
        }))
        .values()
        .reduce_commutative(q!(|combined, new| {
            combined.add(new).unwrap();
        }))
        .latest();

    combined_latencies
        .sample_every(q!(Duration::from_millis(1000)), nondet_sampling)
        .for_each(q!(move |latencies| {
            println!(
                "Latency p50: {:.3} | p99 {:.3} ms | p999 {:.3} ms ({:} samples)",
                Duration::from_nanos(latencies.value_at_quantile(0.5)).as_micros() as f64 / 1000.0,
                Duration::from_nanos(latencies.value_at_quantile(0.99)).as_micros() as f64 / 1000.0,
                Duration::from_nanos(latencies.value_at_quantile(0.999)).as_micros() as f64
                    / 1000.0,
                latencies.len()
            );
        }));

    let keyed_throughputs = results
        .throughput
        .sample_every(q!(Duration::from_millis(1000)), nondet_sampling)
        .send_bincode(aggregator);

    let combined_throughputs = keyed_throughputs
        .reduce_idempotent(q!(|combined, new| {
            *combined = new;
        }))
        .snapshot(&aggregator.tick(), nondet_sampling)
        .values()
        .reduce_commutative(q!(|combined, new| {
            combined.add(new);
        }))
        .latest();

    let client_members = clients.members();
    combined_throughputs
        .sample_every(q!(Duration::from_millis(1000)), nondet_sampling)
        .for_each(q!(move |throughputs| {
            if throughputs.sample_count() >= 2 {
                let num_client_machines = client_members.len();
                let mean = throughputs.sample_mean() * num_client_machines as f64;

                if let Some((lower, upper)) = throughputs.confidence_interval_99() {
                    println!(
                        "Throughput: {:.2} - {:.2} - {:.2} requests/s",
                        lower * num_client_machines as f64,
                        mean,
                        upper * num_client_machines as f64
                    );
                }
            }
        }));
}
