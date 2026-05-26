//! EC2 deployment: redb replication with real crash and failover.
//!
//! Architecture: coordinator (EC2) receives commands from External (localhost),
//! broadcasts to replica cluster. Each replica applies committed PUTs and serves
//! GETs from its own local RedbApplierState. Only the primary responds to GETs.
//!
//! Run with:
//!   AWS_ACCESS_KEY_ID=... cargo run -p hydro_transparent_replicate --features backend_redb --example ec2_failover

use std::sync::Arc;
use futures::SinkExt;
use futures::StreamExt;
use hydro_deploy::{Deployment, Host};
use hydro_lang::deploy::TrybuildHost;
use hydro_lang::location::cluster::CLUSTER_SELF_ID;
use hydro_lang::location::external_process::{ExternalBincodeSink, ExternalBincodeStream};
use hydro_lang::prelude::*;
use hydro_transparent_replicate::messages::TransparentReplica;
use hydro_transparent_replicate::protocol::core_replication_module;
use hydro_transparent_replicate::Coordinator;

#[cfg(stageleft_runtime)]
fn build_pipeline<'a>(
    external: &External<'a, ()>,
    replicas: &Cluster<'a, TransparentReplica>,
    coordinator: &Process<'a, Coordinator>,
) -> (
    ExternalBincodeSink<String>,
    ExternalBincodeStream<String, hydro_lang::live_collections::stream::NoOrder>,
) {
    let (cmd_sink, cmds_at_coord) = coordinator.source_external_bincode(external);

    let puts_at_coord = cmds_at_coord.clone()
        .filter(q!(|cmd: &String| cmd.starts_with("PUT:")));
    let gets_at_coord = cmds_at_coord
        .filter(q!(|cmd: &String| cmd.starts_with("GET:")));

    let puts_at_replicas: Stream<String, Cluster<'a, TransparentReplica>, Unbounded> =
        puts_at_coord
            .broadcast(replicas, TCP.fail_stop().bincode(), nondet!(/** broadcast PUTs */))
            .assume_ordering(nondet!(/** ordering */));

    let gets_at_replicas: Stream<String, Cluster<'a, TransparentReplica>, Unbounded> =
        gets_at_coord
            .broadcast(replicas, TCP.fail_stop().bincode(), nondet!(/** broadcast GETs */));

    let read_only: Stream<String, _, Unbounded> = replicas
        .source_iter(q!(Vec::<String>::new()))
        .into();

    let current_view: Singleton<hydro_transparent_replicate::View, _, _> = replicas
        .source_iter(q!([hydro_transparent_replicate::View { view_num: 0, members: vec![0, 1, 2] }]))
        .fold(
            q!(|| hydro_transparent_replicate::View { view_num: 0, members: vec![0, 1, 2] }),
            q!(|current, new| { *current = new; }),
        )
        .into();

    let reconciled_seq: Optional<usize, _, Unbounded> = replicas
        .source_iter(q!(Vec::<usize>::new()))
        .max()
        .into();

    let tick = replicas.tick();
    let core = core_replication_module(
        replicas, &tick, puts_at_replicas, read_only, current_view.clone(), reconciled_seq,
    );

    let is_primary = current_view
        .snapshot(&tick, nondet!(/** stale view ok */))
        .filter(q!(move |v| CLUSTER_SELF_ID.get_raw_id() == v.primary()))
        .map(q!(|_| ()));

    let gets_on_primary = gets_at_replicas
        .batch(&tick, nondet!(/** batch GETs */))
        .filter_if_some(is_primary)
        .weaken_ordering::<hydro_lang::live_collections::stream::NoOrder>()
        .all_ticks()
        .map(q!(|cmd| (false, 0usize, cmd)));

    let committed_puts = core.replicated
        .map(q!(|(seq, cmd)| (true, seq, cmd)));

    let replica_tick = replicas.tick();
    let responses = committed_puts
        .interleave(gets_on_primary)
        .batch(&replica_tick, nondet!(/** batch */))
        .sort()
        .all_ticks()
        .scan(
            q!(|| hydro_transparent_replicate::applier::RedbApplierState::new()),
            q!(|state: &mut hydro_transparent_replicate::applier::RedbApplierState,
                (is_put, seq, cmd): (bool, usize, String)| {
                Some(state.apply_command(if is_put { seq } else { 0 }, &cmd))
            }),
        );

    let responses_at_coord = responses
        .send(coordinator, TCP.fail_stop().bincode())
        .values();

    let resp_stream = responses_at_coord.send_bincode_external(external);
    (cmd_sink, resp_stream)
}

fn parse_nonce(resp: &str) -> Option<u64> {
    resp.split("nonce=").nth(1)?.split_whitespace().next()?.parse().ok()
}

#[tokio::main]
async fn main() {
    let region = "us-east-1";
    let ami = "ami-0e95a5e2743ec9ec9";
    let instance_type = "t3.micro";
    let rustflags = "-C opt-level=3 -C codegen-units=1";

    println!("=== EC2 Redb Failover Demo ===");
    println!();

    let mut deployment = Deployment::new();
    let existing_vpc = hydro_deploy::AwsNetwork::new(
        region,
        Some(hydro_deploy::aws::NetworkResources::new(
            "vpc-041b334556d749bfb",
            "subnet-099a08497349a365a",
            "sg-05d8b0bfd0d33b8c4",
        )),
    );

    let hosts: Vec<Arc<dyn Host>> = (0..3)
        .map(|i| -> Arc<dyn Host> {
            let name = if i == 0 { "PRIMARY".to_string() } else { format!("backup-{}", i) };
            deployment.AwsEc2Host()
                .region(region).instance_type(instance_type).ami(ami)
                .network(existing_vpc.clone()).display_name(name).add()
        })
        .collect();
    let coordinator_host: Arc<dyn Host> = deployment.AwsEc2Host()
        .region(region).instance_type(instance_type).ami(ami)
        .network(existing_vpc.clone()).display_name("coordinator".to_string()).add();

    let mut builder = FlowBuilder::new();
    let external = builder.external::<()>();
    let replicas = builder.cluster::<TransparentReplica>();
    let coordinator = builder.process::<Coordinator>();

    #[cfg(stageleft_runtime)]
    let (cmd_sink, resp_stream) = build_pipeline(&external, &replicas, &coordinator);

    let features = vec!["backend_redb".to_string()];
    let nodes = builder
        .with_external(&external, deployment.Localhost())
        .with_cluster(&replicas, hosts.iter().map(|h|
            TrybuildHost::new(h.clone()).rustflags(rustflags).features(features.clone())))
        .with_process(&coordinator, TrybuildHost::new(coordinator_host.clone()).rustflags(rustflags).features(features.clone()))
        .deploy(&mut deployment);

    println!("Deploying to EC2...");
    deployment.deploy().await.unwrap();

    #[cfg(stageleft_runtime)]
    let mut sender = nodes.connect(cmd_sink).await;
    #[cfg(stageleft_runtime)]
    let mut receiver = nodes.connect(resp_stream).await;

    deployment.start().await.unwrap();
    println!("All processes started.");
    println!();

    let mut rng: u64 = 0xdeadbeef;
    let mut expected: Vec<u64> = Vec::new();
    for _ in 0..1000 {
        rng ^= rng << 13; rng ^= rng >> 7; rng ^= rng << 17;
        expected.push(rng % 1_000_000);
    }

    println!("Sending 1000 PUTs...");
    let mut nonce: u64 = 1;
    #[cfg(stageleft_runtime)]
    for i in 0..1000usize {
        sender.send(format!("PUT:k{}:{}:{}", i, expected[i], nonce)).await.unwrap();
        nonce += 1;
    }

    let mut last_acked_key: usize = 0;
    #[cfg(stageleft_runtime)]
    {
        let mut n = 0usize;
        let nonce_to_key: std::collections::HashMap<u64, usize> = (0..1000usize)
            .map(|i| ((i as u64) + 1, i))
            .collect();
        let t = tokio::time::timeout(std::time::Duration::from_secs(120), async {
            while let Some(resp) = receiver.next().await {
                if resp.starts_with("OK") {
                    if let Some(resp_nonce) = parse_nonce(&resp) {
                        if let Some(&key_idx) = nonce_to_key.get(&resp_nonce) {
                            last_acked_key = key_idx;
                        }
                    }
                    n += 1;
                    if n % 100 == 0 { println!("  {} commits...", n); }
                    if n >= 1000 { break; }
                }
            }
        }).await;
        if t.is_err() { eprintln!("ERROR: Timed out waiting for 1000 PUT acks"); return; }
    }
    println!("  1000 writes committed. Last acknowledged write: k{}={}", last_acked_key, expected[last_acked_key]);
    println!();

    let verify_keys = vec![0usize, 100, 200, 300, 400, 500, 600, 700, 800, 999];
    println!(">>> In the EC2 console, TERMINATE whichever instances you want.");
    println!(">>> - Kill only *-PRIMARY: failover should succeed, reads return correct values");
    println!(">>> - Kill ALL instances: reads should FAIL (timeout)");
    println!(">>> Press ENTER when done.");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();
    println!();

    println!("Sending {} GETs after crash (waiting up to 60s)...", verify_keys.len());
    let get_nonce_start = nonce;
    #[cfg(stageleft_runtime)]
    for &k in &verify_keys {
        sender.send(format!("GET:k{}:{}", k, nonce)).await.unwrap();
        nonce += 1;
    }
    #[cfg(stageleft_runtime)]
    {
        let nonce_to_get_key: std::collections::HashMap<u64, usize> = verify_keys.iter()
            .enumerate()
            .map(|(i, &k)| (get_nonce_start + i as u64, k))
            .collect();
        let mut responses_after: std::collections::HashMap<usize, String> = std::collections::HashMap::new();

        let t = tokio::time::timeout(std::time::Duration::from_secs(60), async {
            while let Some(resp) = receiver.next().await {
                if resp.starts_with("VALUE") {
                    if let Some(resp_nonce) = parse_nonce(&resp) {
                        if let Some(&key_idx) = nonce_to_get_key.get(&resp_nonce) {
                            println!("  {}", resp);
                            responses_after.insert(key_idx, resp);
                            if responses_after.len() >= verify_keys.len() { break; }
                        }
                    }
                }
            }
        }).await;

        if t.is_err() || responses_after.is_empty() {
            println!();
            println!("❌ No responses after crash — all replicas are dead.");
            println!("   Data is unavailable. This is correct behavior for total cluster failure.");
            return;
        }

        let mut all_correct = true;
        for (&key_idx, resp) in &responses_after {
            if let Some(get_part) = resp.split("GET ").nth(1) {
                if let Some(val_str) = get_part.splitn(2, '=').nth(1) {
                    let expected_val = expected[key_idx];
                    if val_str.trim() != expected_val.to_string() {
                        eprintln!("❌ k{}: expected {}, got {}", key_idx, expected_val, val_str.trim());
                        all_correct = false;
                    }
                }
            }
        }

        if all_correct {
            println!();
            println!("✅ All reads returned correct values after failover.");
        } else {
            eprintln!("❌ Data corruption detected.");
        }
    }
}
