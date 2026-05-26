//! Interactive EC2 demo: type PUT <key> <value> or GET <key> at the prompt.
//!
//! f=2: 5 replicas, each running replica + proposer + acceptor.
//! Tolerates 2 simultaneous failures. Kill any 2 nodes — system keeps working.
//!
//! Uses rusqlite (SQLite) as the storage backend.
//!
//! Run with:
//!   AWS_ACCESS_KEY_ID=... cargo run -p hydro_transparent_replicate --features backend_rusqlite --example ec2_demo2_rusqlite

use std::sync::Arc;
use futures::SinkExt;
use futures::StreamExt;
use hydro_deploy::{Deployment, Host};
use hydro_lang::deploy::TrybuildHost;
use hydro_lang::location::cluster::CLUSTER_SELF_ID;
use hydro_lang::location::external_process::{ExternalBincodeSink, ExternalBincodeStream};
use hydro_lang::prelude::*;
use hydro_test::cluster::paxos::{Acceptor, Proposer};
use hydro_transparent_replicate::messages::TransparentReplica;
use hydro_transparent_replicate::protocol::replicate_service_raw;
use hydro_transparent_replicate::{Coordinator, ReplicateConfig};

const F: usize = 2;
const N: usize = 2 * F + 1;

#[cfg(stageleft_runtime)]
fn build_pipeline<'a>(
    external: &External<'a, ()>,
    replicas: &Cluster<'a, TransparentReplica>,
    proposers: &Cluster<'a, Proposer>,
    acceptors: &Cluster<'a, Acceptor>,
    coordinator: &Process<'a, Coordinator>,
) -> (
    ExternalBincodeSink<String>,
    ExternalBincodeStream<String, hydro_lang::live_collections::stream::NoOrder>,
) {
    let (cmd_sink, cmds_at_coord) = coordinator.source_external_bincode(external);

    let puts_at_coord = cmds_at_coord.clone()
        .filter(q!(|cmd: &String| cmd.starts_with("PUT:")));
    let gets_at_coord = cmds_at_coord.clone()
        .filter(q!(|cmd: &String| cmd.starts_with("GET:")));

    let puts_at_replicas: Stream<String, Cluster<'a, TransparentReplica>, Unbounded> =
        puts_at_coord
            .broadcast(replicas, TCP.fail_stop().bincode(), nondet!(/** broadcast PUTs */))
            .assume_ordering(nondet!(/** ordering */));

    let gets_at_replicas: Stream<String, Cluster<'a, TransparentReplica>, Unbounded> =
        gets_at_coord
            .broadcast(replicas, TCP.fail_stop().bincode(), nondet!(/** broadcast GETs */));

    // Coordinator-driven failure detection
    let coord_timeout = cmds_at_coord
        .map(q!(|_| ()))
        .timeout(
            q!(std::time::Duration::from_secs(10)),
            nondet!(/** timeout on commands without responses */),
        );

    let initial_member_count = N;
    let coord_view: Singleton<hydro_transparent_replicate::View, _, Unbounded> = coordinator
        .source_iter(q!(Vec::<hydro_transparent_replicate::View>::new()))
        .fold(
            q!(move || hydro_transparent_replicate::View {
                view_num: 0,
                members: (0..initial_member_count as u32).collect(),
            }),
            q!(|current, new: hydro_transparent_replicate::View| {
                if new.view_num > current.view_num { *current = new; }
            }),
        )
        .into();

    let coord_fd_tick = coordinator.tick();
    coordinator
        .source_interval(q!(std::time::Duration::from_millis(500)), nondet!(/** fd tick */))
        .batch(&coord_fd_tick, nondet!(/** fd tick batch */))
        .for_each(q!(|_| {}));

    let timeout_in_tick = coord_timeout.snapshot(
        &coord_fd_tick, nondet!(/** stale timeout ok */));
    let view_in_tick = coord_view.snapshot(
        &coord_fd_tick, nondet!(/** stale view ok */));

    let view_change_from_coord = timeout_in_tick
        .if_some_then(view_in_tick)
        .filter_map(q!(|view: hydro_transparent_replicate::View| {
            let primary = view.primary();
            let mut new_members: Vec<u32> = view.members.iter()
                .copied()
                .filter(|&m| m != primary)
                .collect();
            if new_members.is_empty() {
                return None;
            }
            new_members.sort();
            println!("[COORD-FD] Proposing view change: {:?} -> {:?}", view.members, new_members);
            Some(hydro_transparent_replicate::View {
                view_num: view.view_num + 1,
                members: new_members,
            })
        }))
        .into_stream()
        .all_ticks()
        .broadcast(replicas, TCP.fail_stop().bincode(), nondet!(/** broadcast view change */))
        .weaken_ordering::<hydro_lang::live_collections::stream::NoOrder>();

    let config = ReplicateConfig {
        f: F,
        initial_members: (0..N as u32).collect(),
        commit_timeout_ms: 3000,
        notification_interval_ms: 200,
        paxos_config: hydro_transparent_replicate::config::PaxosConfig {
            f: F,
            i_am_leader_send_timeout: 2,
            i_am_leader_check_timeout: 5,
            i_am_leader_check_timeout_delay_multiplier: 3,
        },
        ..ReplicateConfig::default()
    };

    let raw = replicate_service_raw::<String>(
        replicas, proposers, acceptors, puts_at_replicas, view_change_from_coord, config,
    );

    let scan_tick = replicas.tick();

    let _hb = replicas
        .source_interval(q!(std::time::Duration::from_millis(100)), nondet!(/** heartbeat */))
        .batch(&scan_tick, nondet!(/** heartbeat tick */));

    let is_primary = raw.current_view
        .snapshot(&scan_tick, nondet!(/** stale view ok */))
        .filter(q!(move |v: &hydro_transparent_replicate::messages::View| {
            CLUSTER_SELF_ID.get_raw_id() == v.primary()
        }))
        .map(q!(|_| ()));

    let gets_on_primary = gets_at_replicas
        .batch(&scan_tick, nondet!(/** batch GETs */))
        .filter_if_some(is_primary)
        .weaken_ordering::<hydro_lang::live_collections::stream::NoOrder>()
        .all_ticks()
        .map(q!(|cmd| (false, 0usize, cmd)));

    let committed_puts = raw.replicated
        .map(q!(|(seq, cmd)| (true, seq, cmd)));

    let responses = committed_puts
        .interleave(gets_on_primary)
        .batch(&scan_tick, nondet!(/** batch */))
        .sort()
        .all_ticks()
        .scan(
            q!(|| hydro_transparent_replicate::applier::RusqliteApplierState::new()),
            q!(|state: &mut hydro_transparent_replicate::applier::RusqliteApplierState,
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

    println!("=== EC2 Interactive Rusqlite Demo ===");
    println!("  f={f}: {n} nodes (replica+proposer+acceptor each), 1 coordinator = {} EC2 instances",
        N + 1, f = F, n = N);
    println!("  Kill any {} nodes — system keeps working.", F);
    println!("Commands: PUT <key> <value>  |  GET <key>  |  quit");
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

    let hosts: Vec<Arc<dyn Host>> = (0..N)
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
    let proposers = builder.cluster::<Proposer>();
    let acceptors = builder.cluster::<Acceptor>();
    let coordinator = builder.process::<Coordinator>();

    #[cfg(stageleft_runtime)]
    let (cmd_sink, resp_stream) =
        build_pipeline(&external, &replicas, &proposers, &acceptors, &coordinator);

    let features = vec!["backend_rusqlite".to_string()];
    let nodes = builder
        .with_external(&external, deployment.Localhost())
        .with_cluster(&replicas, hosts.iter().map(|h|
            TrybuildHost::new(h.clone()).rustflags(rustflags).features(features.clone())))
        .with_cluster(&proposers, hosts.iter().map(|h|
            TrybuildHost::new(h.clone()).rustflags(rustflags).features(features.clone())))
        .with_cluster(&acceptors, hosts.iter().map(|h|
            TrybuildHost::new(h.clone()).rustflags(rustflags).features(features.clone())))
        .with_process(&coordinator, TrybuildHost::new(coordinator_host.clone()).rustflags(rustflags).features(features.clone()))
        .deploy(&mut deployment);

    println!("Deploying to EC2 ({} instances)...", N + 1);
    deployment.deploy().await.unwrap();

    #[cfg(stageleft_runtime)]
    let mut sender = nodes.connect(cmd_sink).await;
    #[cfg(stageleft_runtime)]
    let mut receiver = nodes.connect(resp_stream).await;

    deployment.start().await.unwrap();
    println!("All processes started. Ready.");
    println!();

    #[cfg(stageleft_runtime)]
    {
        let mut nonce: u64 = 1;
        let stdin = tokio::io::stdin();
        let mut lines = tokio::io::AsyncBufReadExt::lines(tokio::io::BufReader::new(stdin));

        loop {
            print!("> ");
            use std::io::Write;
            std::io::stdout().flush().unwrap();

            let line = match lines.next_line().await {
                Ok(Some(l)) => l,
                _ => break,
            };
            let line = line.trim().to_string();
            if line.is_empty() { continue; }
            if line == "quit" || line == "exit" { break; }

            let parts: Vec<&str> = line.splitn(3, ' ').collect();
            let cmd_str = match parts[0].to_uppercase().as_str() {
                "PUT" if parts.len() == 3 => format!("PUT:{}:{}:{}", parts[1], parts[2], nonce),
                "GET" if parts.len() == 2 => format!("GET:{}:{}", parts[1], nonce),
                _ => {
                    println!("Usage: PUT <key> <value>  |  GET <key>  |  quit");
                    continue;
                }
            };

            let current_nonce = nonce;
            nonce += 1;

            sender.send(cmd_str).await.unwrap();

            let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(30);
            let result = loop {
                let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
                if remaining.is_zero() { break None; }
                match tokio::time::timeout(remaining, receiver.next()).await {
                    Ok(Some(resp)) => {
                        if let Some(n) = parse_nonce(&resp) {
                            if n == current_nonce { break Some(resp); }
                        }
                    }
                    _ => break None,
                }
            };

            match result {
                Some(resp) => println!("{}", resp),
                None => println!("❌ No response (timeout — failover in progress, retry in a moment)"),
            }
        }
    }

    println!("Bye.");
}
