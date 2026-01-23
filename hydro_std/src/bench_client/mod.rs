use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, SystemTime};

use hdrhistogram::Histogram;
use hdrhistogram::serialization::{Deserializer, Serializer, V2Serializer};
use hydro_lang::live_collections::stream::NoOrder;
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
/// (up to `num_clients_per_node` per machine)
/// * `workload_generator`: Converts previous output (or None, for new virtual clients) into the next input payload
/// * `protocol`: The protocol to benchmark
///
/// ## Returns
/// A stream of latencies per completed client request
pub fn bench_client<'a, Client, Input, Output>(
    clients: &Cluster<'a, Client>,
    num_clients_per_node: usize,
    workload_generator: impl FnOnce(
        KeyedStream<u32, Option<Output>, Cluster<'a, Client>, Unbounded, NoOrder>,
    )
        -> KeyedStream<u32, Input, Cluster<'a, Client>, Unbounded, NoOrder>,
    protocol: impl FnOnce(
        KeyedStream<u32, Input, Cluster<'a, Client>, Unbounded, NoOrder>,
    ) -> KeyedStream<u32, Output, Cluster<'a, Client>, Unbounded, NoOrder>,
) -> KeyedStream<u32, (Output, Duration), Cluster<'a, Client>, Unbounded, NoOrder>
where
    Input: Clone,
    Output: Clone,
{
    let dummy = clients.singleton(q!(0));
    #[expect(unused_variables, reason = "sliced! requires at least 1 use statement")]
    let new_payload_ids = sliced! {
        let dummy_batched = use(dummy, nondet!(/** temp */));
        let mut next_virtual_client = use::state(|l| Optional::from(l.singleton(q!((0u32, None)))));

        // Set up virtual clients - spawn new ones each tick until we reach the limit
        let new_virtual_client = next_virtual_client.clone();
        next_virtual_client = new_virtual_client.clone().filter_map(
            q!(move |(virtual_id, _)| {
                if virtual_id < num_clients_per_node as u32 {
                    Some((virtual_id + 1, None))
                } else {
                    None
                }
            }),
        );

        new_virtual_client.into_stream().into_keyed()
    };

    let (protocol_outputs_complete, protocol_outputs) =
        clients.forward_ref::<KeyedStream<u32, Output, Cluster<'a, Client>, Unbounded, NoOrder>>();
    // Use new payload IDS and previous outputs to generate new payloads
    let protocol_inputs = workload_generator(
        new_payload_ids.interleave(protocol_outputs.map(q!(|payload| Some(payload)))),
    );
    // Feed new payloads to the protocol
    let protocol_outputs = protocol(protocol_inputs.clone());
    protocol_outputs_complete.complete(protocol_outputs.clone());

    // Persist start latency, overwrite on new value. Memory footprint = O(num_clients_per_node)
    let start_times = protocol_inputs
        .reduce(q!(
            |curr, new| {
                *curr = new;
            },
            commutative = ManualProof(/* The value will be thrown away */)
        ))
        .map(q!(|_input| SystemTime::now()));

    sliced! {
        let start_times = use(start_times, nondet!(/** Only one in-flight message per virtual client at any time, and outputs happen-after inputs, so if an output is received the start_times must contain its input time. */));
        let current_outputs = use(protocol_outputs, nondet!(/** Batching is required to compare output to input time, but does not actually affect the result. */));

        let end_times_and_output = current_outputs
            .assume_ordering(nondet!(/** Only one in-flight message per virtual client at any time, and they are causally dependent, so this just casts to KeyedSingleton */))
            .reduce(
                q!(
                    |curr, new| {
                        *curr = new;
                    },
                ),
            )
            .map(q!(|output| (SystemTime::now(), output)));

        start_times
            .join_keyed_singleton(end_times_and_output)
            .map(q!(|(start_time, (end_time, output))| (output, end_time.duration_since(start_time).unwrap())))
            .into_keyed_stream()
            .weakest_ordering()
    }
}

/// Computes the throughput and latency of transactions.
///
/// # Non-Determinism
/// This function uses non-deterministic wall-clock windows for measuring throughput.
pub fn compute_throughput_latency<'a, Client: 'a>(
    clients: &Cluster<'a, Client>,
    latencies: Stream<Duration, Cluster<'a, Client>, Unbounded, NoOrder>,
    nondet_measurement_window: NonDet,
) -> BenchResult<'a, Client> {
    // 1. Calculate latencies
    let latency_histogram = latencies.clone().fold(
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

    // 2. Calculate throughput
    let throughput_batch = sliced! {
        let latencies_batch = use(latencies, nondet_measurement_window);
        latencies_batch
            .count()
            .into_stream()
    };

    // Tuple of (batch_size, bool), where the bool is true if the existing throughputs should be placed in its own window, and a new window should be created
    let punctuated_throughput = throughput_batch
        .map(q!(|batch_size| (batch_size, false)))
        .merge_ordered(
            clients
                .source_interval(q!(Duration::from_secs(1)), nondet_measurement_window)
                .map(q!(|_| (0, true))),
            nondet_measurement_window,
        );

    let throughput = punctuated_throughput
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
        latency_histogram,
        throughput,
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
