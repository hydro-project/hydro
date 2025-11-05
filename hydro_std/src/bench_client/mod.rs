use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use hdrhistogram::Histogram;
use hdrhistogram::serialization::{Deserializer, Serializer, V2Serializer};
use hydro_lang::live_collections::stream::{NoOrder, TotalOrder};
use hydro_lang::prelude::*;
use serde::{Deserialize, Serialize};

pub mod rolling_average;
use rolling_average::RollingAverage;

use crate::membership::track_membership;

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
    let initial_virtual_client = client_tick.optional_first_tick(q!(0u32));
    let (next_virtual_client_complete_cycle, next_virtual_client) = client_tick.cycle();
    let new_virtual_client = initial_virtual_client.or(next_virtual_client);
    next_virtual_client_complete_cycle.complete_next_tick(new_virtual_client.clone().filter_map(
        q!(move |virtual_id| {
            if virtual_id < num_clients_per_node as u32 {
                Some(virtual_id + 1)
            } else {
                None
            }
        }),
    ));

    let new_virtual_client_stream = new_virtual_client.into_stream();

    let c_new_payloads_on_start = new_virtual_client_stream
        .clone()
        .map(q!(|virtual_id| (virtual_id, None)));

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
        client_tick.cycle::<Stream<(u32, Instant), _, _, NoOrder>>();
    let c_new_timers_when_leader_elected =
        new_virtual_client_stream.map(q!(|virtual_id| (virtual_id, Instant::now())));
    let c_updated_timers = c_received_quorum_payloads
        .clone()
        .map(q!(|(key, _payload)| (key, Instant::now())));
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
        .filter_if_none(c_stats_output_timer.clone())
        .map(q!(|batch_size| (batch_size, false)));

    let c_throughput_reset = c_stats_output_timer.map(q!(|_| (0, true))).defer_tick();

    let c_throughput = c_throughput_new_batch
        .into_stream()
        .chain(c_throughput_reset.into_stream())
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
    let nondet_client_count = nondet!(/** client count is stable in bench */);
    let nondet_sampling = nondet!(/** non-deterministic samping only affects logging */);
    let print_tick = aggregator.tick();
    let client_members = aggregator.source_cluster_members(clients);
    let client_count = track_membership(client_members)
        .snapshot(&print_tick, nondet_client_count)
        .filter(q!(|b| *b))
        .key_count();

    let keyed_throughputs = results
        .throughput
        .sample_every(q!(Duration::from_millis(1000)), nondet_sampling)
        .send_bincode(aggregator);

    let latest_throughputs = keyed_throughputs.reduce_idempotent(q!(|combined, new| {
        *combined = new;
    }));

    let clients_with_throughputs_count = latest_throughputs
        .clone()
        .snapshot(&print_tick, nondet_client_count)
        // Remove throughputs from clients that have yet to actually record process
        .filter(q!(|throughputs| throughputs.sample_mean() > 0.0))
        .key_count();

    let waiting_for_clients = client_count
        .clone()
        .zip(clients_with_throughputs_count)
        .filter_map(q!(|(num_clients, num_clients_with_throughput)| {
            if num_clients > num_clients_with_throughput {
                Some(num_clients - num_clients_with_throughput)
            } else {
                None
            }
        }));

    waiting_for_clients
        .clone()
        .all_ticks()
        .sample_every(q!(Duration::from_millis(1000)), nondet_sampling)
        .assume_retries(nondet!(/** extra logs due to duplicate samples are okay */))
        .for_each(q!(|num_clients_not_responded| println!(
            "Awaiting {} clients",
            num_clients_not_responded
        )));

    let combined_throughputs = sliced!(|
        use(latest_throughputs, nondet_sampling) as latest_throughput_snapshot
    | {
        latest_throughput_snapshot
            .values()
            .reduce_commutative(q!(|combined, new| {
                combined.add(new);
            }))
    });

    combined_throughputs
        .sample_every(q!(Duration::from_millis(1000)), nondet_sampling)
        .batch(&print_tick, nondet_client_count)
        .cross_singleton(client_count.clone())
        .filter_if_none(waiting_for_clients.clone())
        .all_ticks()
        .assume_retries(nondet!(/** extra logs due to duplicate samples are okay */))
        .for_each(q!(move |(throughputs, num_client_machines)| {
            if throughputs.sample_count() >= 2 {
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

    let keyed_latencies = results
        .latency_histogram
        .sample_every(q!(Duration::from_millis(1000)), nondet_sampling)
        .map(q!(|latencies| {
            SerializableHistogramWrapper {
                histogram: latencies,
            }
        }))
        .send_bincode(aggregator);

    let most_recent_histograms = keyed_latencies
        .map(q!(|histogram| histogram.histogram.borrow().clone()))
        .reduce_idempotent(q!(|combined, new| {
            // get the most recent histogram for each client
            *combined = new;
        }));

    let combined_latencies = sliced!(|
        use(most_recent_histograms, nondet_sampling) as latencies
    | {
        latencies
            .values()
            .reduce_commutative(q!(|combined, new| {
                combined.add(new).unwrap();
            }))
    });

    combined_latencies
        .sample_every(q!(Duration::from_millis(1000)), nondet_sampling)
        .batch(&print_tick, nondet_client_count)
        .filter_if_none(waiting_for_clients)
        .all_ticks()
        .assume_retries(nondet!(/** extra logs due to duplicate samples are okay */))
        .for_each(q!(move |latencies| {
            println!(
                "Latency p50: {:.3} | p99 {:.3} | p999 {:.3} ms ({:} samples)",
                Duration::from_nanos(latencies.value_at_quantile(0.5)).as_micros() as f64 / 1000.0,
                Duration::from_nanos(latencies.value_at_quantile(0.99)).as_micros() as f64 / 1000.0,
                Duration::from_nanos(latencies.value_at_quantile(0.999)).as_micros() as f64
                    / 1000.0,
                latencies.len()
            );
        }));
}
