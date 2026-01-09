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
    let (c_to_proposers_complete_cycle, c_to_proposers) =
        clients.forward_ref::<Stream<_, _, _, TotalOrder>>();

    let (c_latencies, c_throughput_batches, c_new_payloads) = sliced! {
        let mut next_virtual_client = use::state(|l| Optional::from(l.singleton(q!(0u32))));
        let mut timers = use::state_null::<KeyedSingleton<u32, Instant, _, _>>();

        let transaction_results = use(transaction_cycle(c_to_proposers).into_keyed(), nondet!(
            /// because the transaction processor is required to handle arbitrary reordering
            /// across *different* keys, we are safe because delaying a transaction result for a key
            /// will only affect when the next request for that key is emitted with respect to other keys
        ));

        // Set up virtual clients - spawn new ones each tick until we reach the limit
        let new_virtual_client = next_virtual_client.clone();
        next_virtual_client = new_virtual_client.clone().filter_map(
            q!(move |virtual_id| {
                if virtual_id < num_clients_per_node as u32 {
                    Some(virtual_id + 1)
                } else {
                    None
                }
            }),
        );

        let new_virtual_client_stream = new_virtual_client.into_stream();

        let c_new_payloads_on_start = new_virtual_client_stream
            .clone()
            .map(q!(|virtual_id| (virtual_id, None)))
            .into_keyed();

        let c_received_quorum_payloads = transaction_results
            .map(q!(|payload| Some(payload)));

        // Track statistics - timers for latency measurement
        let c_new_timers_when_leader_elected =
            new_virtual_client_stream.map(q!(|virtual_id| (virtual_id, Instant::now()))).into_keyed();
        let c_updated_timers = c_received_quorum_payloads
            .clone()
            .map(q!(|_payload| Instant::now()));

        let c_latencies = timers
            .clone()
            .get_many_if_present(c_updated_timers.clone())
            .values()
            .map(q!(
                |(prev_time, curr_time)| curr_time.duration_since(prev_time)
            ));

        timers = timers // Update timers in tick+1 so we can record differences during this tick (to track latency)
            .into_keyed_stream()
            .chain(c_new_timers_when_leader_elected)
            .chain(c_updated_timers)
            .reduce(q!(|curr_time, new_time| {
                if new_time > *curr_time {
                    *curr_time = new_time;
                }
            }, commutative = ManualProof(/* max is commutative */)));

        // Throughput tracking
        let c_throughput_new_batch = c_received_quorum_payloads
            .clone()
            .values()
            .count();

        let c_new_payloads = c_new_payloads_on_start.chain(c_received_quorum_payloads);

        (c_latencies, c_throughput_new_batch.into_stream(), c_new_payloads)
    };

    let c_new_payloads = workload_generator(clients, c_new_payloads.entries());
    c_to_proposers_complete_cycle.complete(c_new_payloads.assume_ordering::<TotalOrder>(nondet!(
        /// We don't send a new write for the same key until the previous one is committed,
        /// so this contains only a single write per key, and we don't care about order
        /// across keys.
    )));

    let c_latencies = c_latencies.fold(
        q!(move || Rc::new(RefCell::new(Histogram::<u64>::new(3).unwrap()))),
        q!(
            move |latencies, latency| {
                latencies
                    .borrow_mut()
                    .record(latency.as_nanos() as u64)
                    .unwrap();
            },
            commutative = ManualProof(
                /* adding elements to histogram is commutative */
            )
        ),
    );

    let throughput_with_timers = c_throughput_batches
        .map(q!(|batch_size| (batch_size, false)))
        .merge_ordered(
            clients
                .source_interval(q!(Duration::from_secs(1)), nondet_throughput_window)
                .map(q!(|_| (0, true))),
            nondet_throughput_window,
        );

    let c_throughput = throughput_with_timers
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
        .map(q!(|(_, stats)| stats));

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
        .send(aggregator, TCP.bincode());

    let latest_throughputs = keyed_throughputs.reduce(q!(
        |combined, new| {
            *combined = new;
        },
        idempotent = ManualProof(/* assignment is idempotent */)
    ));

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

    let combined_throughputs = sliced! {
        let latest_throughput_snapshot = use(latest_throughputs, nondet_sampling);
        latest_throughput_snapshot
            .values()
            .reduce(q!(|combined, new| {
                combined.add(new);
            }, commutative = ManualProof(/* rolling average is commutative */)))
    };

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
        .send(aggregator, TCP.bincode());

    let most_recent_histograms = keyed_latencies
        .map(q!(|histogram| histogram.histogram.borrow().clone()))
        .reduce(q!(
            |combined, new| {
                // get the most recent histogram for each client
                *combined = new;
            },
            idempotent = ManualProof(/* assignment is idempotent */)
        ));

    let combined_latencies = sliced! {
        let latencies = use(most_recent_histograms, nondet_sampling);
        latencies
            .values()
            .reduce(q!(|combined, new| {
                combined.add(new).unwrap();
            }, commutative = ManualProof(/* combining histories is commutative */)))
    };

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
