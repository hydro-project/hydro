use std::time::Instant;

use futures::StreamExt;
use hydro_deploy::Deployment;
use hydro_lang::deploy::TrybuildHost;
use hydro_lang::location::Location;
use hydro_lang::location::external_process::ExternalBincodeStream;
use hydro_test::cluster::paxos::{CorePaxos, PaxosConfig};
use stageleft::q;

const BENCH_DURATION_SECS: u64 = 15;
const TAIL_SAMPLES: usize = 5;

#[tokio::main]
async fn main() {
    let mut deployment = Deployment::new();
    let localhost = deployment.Localhost();

    let mut builder = hydro_lang::compile::builder::FlowBuilder::new();
    let external = builder.external::<()>();

    let f = 1;
    let num_clients = 1;
    let num_clients_per_node = 100;
    let checkpoint_frequency = 1000;
    let i_am_leader_send_timeout = 5;
    let i_am_leader_check_timeout = 10;
    let i_am_leader_check_timeout_delay_multiplier = 15;
    let print_result_frequency = 1000; // millis

    let proposers = builder.cluster();
    let acceptors = builder.cluster();
    let clients = builder.cluster();
    let client_aggregator = builder.process();
    let replicas = builder.cluster();

    // Capture the ExternalBincodeStream handle from inside the closure.
    let mut throughput_handle: Option<ExternalBincodeStream<usize>> = None;

    hydro_test::cluster::paxos_bench::paxos_bench(
        checkpoint_frequency,
        f,
        f + 1,
        CorePaxos {
            proposers: proposers.clone(),
            acceptors: acceptors.clone(),
            paxos_config: PaxosConfig {
                f,
                i_am_leader_send_timeout,
                i_am_leader_check_timeout,
                i_am_leader_check_timeout_delay_multiplier,
            },
        },
        &clients,
        clients.singleton(q!(std::env::var("NUM_CLIENTS_PER_NODE")
            .unwrap()
            .parse::<usize>()
            .unwrap())),
        &client_aggregator,
        &replicas,
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
            &proposers,
            (0..f + 1).map(|_| TrybuildHost::new(localhost.clone()).rustflags(rustflags)),
        )
        .with_cluster(
            &acceptors,
            (0..2 * f + 1).map(|_| TrybuildHost::new(localhost.clone()).rustflags(rustflags)),
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
        .with_cluster(
            &replicas,
            (0..f + 1).map(|_| TrybuildHost::new(localhost.clone()).rustflags(rustflags)),
        )
        .with_external(&external, localhost.clone())
        .deploy(&mut deployment);

    deployment.deploy().await.unwrap();

    let mut throughput_recv = nodes.connect(throughput_handle).await;

    deployment.start().await.unwrap();

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
    let variance = tail.iter().map(|s| (s - avg).powi(2)).sum::<f64>() / tail.len() as f64;
    let stddev = variance.sqrt();

    println!(
        "test paxos_bench ... bench: {:.2} ops/s (+/- {:.2})",
        avg, stddev
    );
}
