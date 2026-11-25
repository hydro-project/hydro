use std::time::Instant;

use futures::StreamExt;
use hydro_deploy::Deployment;
use hydro_lang::deploy::TrybuildHost;
use hydro_lang::location::Location;
use hydro_lang::location::external_process::ExternalBincodeStream;
use hydro_test::cluster::raft::RaftConfig;
use stageleft::q;

/// `bench_client` starts virtual clients gradually, and throughput with 20k clients is
/// still rising 15 s after the first sample, so the measured tail starts later.
const BENCH_DURATION_SECS: u64 = 60;
const TAIL_SAMPLES: usize = 5;
/// Max wait for leader election (first positive sample); measurement starts after it.
const FIRST_THROUGHPUT_TIMEOUT_SECS: u64 = 60;

#[tokio::main]
async fn main() {
    let mut deployment = Deployment::new();
    let localhost = deployment.Localhost();

    let mut builder = hydro_lang::compile::builder::FlowBuilder::new();
    let external = builder.external::<()>();

    let cluster_size = 3;
    let num_clients = 1;
    // The leader only sends `AppendEntries` on a heartbeat, so each closed-loop request
    // takes at least one heartbeat and throughput is at most clients / heartbeat. Fewer
    // clients than about 10k (at a 10 ms heartbeat) measure the heartbeat, not Raft.
    let num_clients_per_node = 20000;
    let election_timeout_millis = 500;
    let heartbeat_interval_millis = 10;
    let print_result_frequency = 1000; // millis

    let replicas = builder.cluster();
    let clients = builder.cluster();
    let client_aggregator = builder.process();

    // Capture the ExternalBincodeStream handle from inside the closure.
    let mut throughput_handle: Option<ExternalBincodeStream<usize>> = None;

    hydro_test::cluster::raft_bench::raft_bench(
        &replicas,
        RaftConfig { cluster_size },
        election_timeout_millis,
        heartbeat_interval_millis,
        &clients,
        clients.singleton(q!(std::env::var("NUM_CLIENTS_PER_NODE")
            .unwrap()
            .parse::<usize>()
            .unwrap())),
        &client_aggregator,
        print_result_frequency / 10,
        print_result_frequency,
        |bench_result| {
            throughput_handle = Some(bench_result.throughput.send_bincode_external(&external));
        },
    );

    let throughput_handle = throughput_handle.expect("closure should have set throughput_handle");

    let rustflags = "-C opt-level=3 -C codegen-units=1 -C strip=none -C debuginfo=2 -C lto=off";

    let nodes = builder
        .with_cluster(
            &replicas,
            (0..cluster_size).map(|_| TrybuildHost::new(localhost.clone()).rustflags(rustflags)),
        )
        .with_cluster(
            &clients,
            (0..num_clients).map(|_| {
                TrybuildHost::new(localhost.clone())
                    .rustflags(rustflags)
                    .env("NUM_CLIENTS_PER_NODE", num_clients_per_node.to_string())
            }),
        )
        .with_process(
            &client_aggregator,
            TrybuildHost::new(localhost.clone()).rustflags(rustflags),
        )
        .with_external(&external, localhost.clone())
        .deploy(&mut deployment);

    deployment.deploy().await.unwrap();

    let mut throughput_recv = nodes.connect(throughput_handle).await;

    deployment.start().await.unwrap();

    // Wait for the leader to be elected: the first positive sample.
    let warmup_start = Instant::now();
    let warmup_deadline =
        warmup_start + std::time::Duration::from_secs(FIRST_THROUGHPUT_TIMEOUT_SECS);
    let mut zero_samples = 0usize;
    loop {
        let timeout = warmup_deadline.saturating_duration_since(Instant::now());
        match tokio::time::timeout(timeout, throughput_recv.next()).await {
            Ok(Some(throughput)) if 0 < throughput => {
                eprintln!(
                    "First positive throughput sample after {:.1?} ({} zero samples): {} ops/s",
                    warmup_start.elapsed(),
                    zero_samples,
                    throughput
                );
                break;
            }
            Ok(Some(_zero)) => zero_samples += 1,
            Ok(None) => {
                eprintln!(
                    "Throughput stream closed after {:.1?} without a positive sample ({} zero samples)",
                    warmup_start.elapsed(),
                    zero_samples
                );
                std::process::exit(1);
            }
            Err(_) => {
                eprintln!(
                    "No positive throughput sample within {} seconds ({} zero samples)",
                    FIRST_THROUGHPUT_TIMEOUT_SECS, zero_samples
                );
                std::process::exit(1);
            }
        }
    }

    // Measure steady-state throughput for `BENCH_DURATION_SECS` after election.
    let start = Instant::now();
    let deadline = start + std::time::Duration::from_secs(BENCH_DURATION_SECS);
    let mut samples: Vec<f64> = Vec::new();

    while Instant::now() < deadline {
        let timeout = deadline.saturating_duration_since(Instant::now());
        match tokio::time::timeout(timeout, throughput_recv.next()).await {
            Ok(Some(throughput)) => {
                eprintln!("Throughput sample: {} ops/s", throughput);
                samples.push(throughput as f64);
            }
            Ok(None) => break,
            Err(_) => break,
        }
    }

    let tail: &[f64] = if samples.len() >= TAIL_SAMPLES {
        &samples[samples.len() - TAIL_SAMPLES..]
    } else {
        &samples
    };

    if tail.is_empty() {
        eprintln!(
            "No throughput samples received within {} seconds",
            BENCH_DURATION_SECS
        );
        std::process::exit(1);
    }

    let avg = tail.iter().sum::<f64>() / tail.len() as f64;
    if avg <= 0.0 {
        eprintln!(
            "Throughput dropped to zero: the last {} samples were {:?}",
            tail.len(),
            tail
        );
        std::process::exit(1);
    }
    let variance = tail.iter().map(|s| (s - avg).powi(2)).sum::<f64>() / tail.len() as f64;
    let stddev = variance.sqrt();

    println!(
        "test raft_bench ... bench: {:.2} ops/s (+/- {:.2})",
        avg, stddev
    );
}
