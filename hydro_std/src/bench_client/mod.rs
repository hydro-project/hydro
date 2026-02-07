use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, SystemTime};

use hdrhistogram::Histogram;
use hdrhistogram::serialization::{Deserializer, Serializer, V2Serializer};
use hydro_lang::live_collections::stream::{ExactlyOnce, NoOrder, TotalOrder};
use hydro_lang::prelude::*;
use serde::{Deserialize, Serialize};

pub mod rolling_average;

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

pub struct BenchResult<L> {
    pub latency_histogram:
        Stream<Rc<RefCell<Histogram<u64>>>, L, Unbounded, TotalOrder, ExactlyOnce>,
    pub throughput: Stream<usize, L, Unbounded, TotalOrder, ExactlyOnce>,
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
    let new_payload_ids = sliced! {
        let _dummy_batched = use(dummy, nondet!(/** temp */));
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
            .defer_tick() // Get the start_times before they were overwritten with the newly generated input
            .join_keyed_singleton(end_times_and_output)
            .map(q!(|(start_time, (end_time, output))| (output, end_time.duration_since(start_time).unwrap())))
            .into_keyed_stream()
            .weaken_ordering()
    }
}

/// Computes the throughput and latency of transactions and outputs it every `interval_millis`.
/// An output is produced even if there are no transactions.
///
/// # Non-Determinism
/// This function uses non-deterministic wall-clock windows for measuring throughput.
pub fn compute_throughput_latency<'a, Client: 'a>(
    clients: &Cluster<'a, Client>,
    latencies: Stream<Duration, Cluster<'a, Client>, Unbounded, NoOrder>,
    interval_millis: u64,
    nondet_measurement_window: NonDet,
) -> BenchResult<Cluster<'a, Client>> {
    let punctuation = clients.source_interval(
        q!(Duration::from_millis(interval_millis)),
        nondet_measurement_window,
    );

    let (interval_throughput, interval_latency) = sliced! {
        let punctuation = use(punctuation, nondet_measurement_window);
        let latencies = use(latencies, nondet_measurement_window);
        let mut latency_histogram = use::state(|l| l.singleton(q!(Rc::new(RefCell::new(Histogram::<u64>::new(3).unwrap())))));
        let mut throughput = use::state(|l| l.singleton(q!(0usize)));

        let punctuation_option = punctuation.first();
        let batched_latency_histogram = latencies.clone().fold(
            q!(move || Histogram::<u64>::new(3).unwrap()),
            q!(move |latencies, latency| {
                    latencies
                        .record(latency.as_nanos() as u64)
                        .unwrap();
                },
                commutative = ManualProof(
                    /* adding elements to histogram is commutative */
                )
            ),
        );

        // Output every punctuation
        let interval_throughput = throughput.clone().filter_if_some(punctuation_option.clone());
        let interval_latency = latency_histogram.clone().filter_if_some(punctuation_option.clone());

        let batched_throughput = latencies.count();
        // Clear every punctuation
        let prev_throughput = throughput.filter_if_none(punctuation_option.clone());
        // Merge new values
        throughput = batched_throughput
            .clone()
            .zip(prev_throughput.clone())
            .map(q!(|(new, old)| new + old))
            .unwrap_or(batched_throughput.clone());

        // Clear every punctuation
        let prev_histogram = latency_histogram.filter_if_none(punctuation_option);
        // Merge new values
        latency_histogram = batched_latency_histogram
            .clone()
            .zip(prev_histogram.clone())
            .map(q!(|(new, old)| {
                old.borrow_mut().add(new).expect("Error adding value to histogram");
                old
            }))
            .unwrap_or(batched_latency_histogram.map(q!(|histogram| Rc::new(RefCell::new(histogram)))));

        (interval_throughput.into_stream(), interval_latency.into_stream())
    };

    BenchResult {
        latency_histogram: interval_latency,
        throughput: interval_throughput,
    }
}

/// Returns transaction throughput and latency results.
/// Aggregates results from clients and outputs every `output_interval_millis`.
///
/// Note: Inconsistent windowing may result in unexpected outputs unless `output_interval_millis` >> `interval_millis`.
pub fn aggregate_bench_results<'a, Client: 'a, Aggregator>(
    results: BenchResult<Cluster<'a, Client>>,
    aggregator: &Process<'a, Aggregator>,
    output_interval_millis: u64,
) -> BenchResult<Process<'a, Aggregator>> {
    let nondet_sampling = nondet!(/** non-deterministic samping only affects logging */);
    let punctuation = aggregator.source_interval(
        q!(Duration::from_millis(output_interval_millis)),
        nondet_sampling,
    );

    let a_throughputs = results
        .throughput
        .send(aggregator, TCP.fail_stop().bincode())
        .values();

    let a_latencies = results
        .latency_histogram
        .map(q!(|latencies| {
            SerializableHistogramWrapper {
                histogram: latencies,
            }
        }))
        .send(aggregator, TCP.fail_stop().bincode())
        .values()
        .map(q!(|wrapper| wrapper.histogram));

    let (combined_throughputs, combined_latencies) = sliced! {
        let punctuation = use(punctuation, nondet_sampling);
        let a_throughputs = use(a_throughputs, nondet_sampling);
        let a_latencies = use(a_latencies, nondet_sampling);
        let mut latency_histogram = use::state(|l| l.singleton(q!(Rc::new(RefCell::new(Histogram::<u64>::new(3).unwrap())))));
        let mut throughput = use::state(|l| l.singleton(q!(0usize)));

        let punctuation_option = punctuation.first();

        // Output every punctuation
        let interval_throughput = throughput.clone().filter_if_some(punctuation_option.clone());
        let interval_latency = latency_histogram.clone().filter_if_some(punctuation_option.clone());

        // Clear every punctuation
        let prev_throughput = throughput.filter_if_none(punctuation_option.clone()).into_stream();
        // Merge new values
        throughput = a_throughputs
            .chain(prev_throughput)
            .fold(q!(|| 0usize), q!(|curr, new| {
                    *curr += new;
                },
                commutative = ManualProof(/* Addition is commutative */)
            ));

        // Merge new values
        let merged_new_histograms = a_latencies
            .reduce(
                q!(|curr, new| {
                    curr.borrow_mut().add(&*new.borrow_mut()).expect("Error adding value to histogram");
                },
                commutative = ManualProof(/* Merge is commutative */)
            ));
        // Clear every punctuation
        latency_histogram = latency_histogram
            .zip(merged_new_histograms.into_singleton())
            .zip(punctuation_option.into_singleton())
            .map(q!(|((old, new), reset)| {
                if reset.is_some() {
                    // Use replace instead of clear, since interval_latency is pointing to the Histogram too
                    old.replace(Histogram::<u64>::new(3).unwrap());
                }
                if let Some(new) = new {
                    old.borrow_mut().add(&*new.borrow_mut()).expect("Error adding value to histogram");
                }
                old
            }));

        (interval_throughput.into_stream(), interval_latency.into_stream())
    };

    BenchResult {
        throughput: combined_throughputs,
        latency_histogram: combined_latencies,
    }
}

/// Pretty prints output of `aggregate_bench_results`.
///
/// Prints the throughput, and the 50th, 99th, and 99.9th percentile latencies.
pub fn pretty_print_bench_results<'a, Aggregator>(
    aggregate_results: BenchResult<Process<'a, Aggregator>>,
) {
    aggregate_results.throughput.for_each(q!(|throughput| {
        println!("Throughput: {:.2} requests/s", throughput);
    }));
    aggregate_results
        .latency_histogram
        .map(q!(move |latencies| (
            // Convert to milliseconds but include floating point (as_millis is for whole numbers only)
            Duration::from_nanos(latencies.borrow().value_at_quantile(0.5)).as_micros() as f64
                / 1000.0,
            Duration::from_nanos(latencies.borrow().value_at_quantile(0.99)).as_micros() as f64
                / 1000.0,
            Duration::from_nanos(latencies.borrow().value_at_quantile(0.999)).as_micros() as f64
                / 1000.0,
            latencies.borrow().len(),
        )))
        .for_each(q!(move |(p50, p99, p999, num_samples)| {
            println!(
                "Latency p50: {:.3} | p99 {:.3} | p999 {:.3} ms ({:} samples)",
                p50, p99, p999, num_samples
            );
        }));
}
