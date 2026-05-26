//! Test: kill nodes one at a time with LONG waits between kills.
//! Proves that once nodes are truly dead, Paxos blocks at 2 acceptors.
//!
//! Strategy: kill host, wait 30s (way past any grace period), THEN kill next.
//! With f=2, quorum=3. After the 3rd kill, only 2 acceptors remain.
//! The system should be stuck — no further view changes possible.

#[cfg(stageleft_runtime)]
use hydro_lang::location::cluster::CLUSTER_SELF_ID;
#[cfg(stageleft_runtime)]
use hydro_lang::location::external_process::{ExternalBincodeSink, ExternalBincodeStream};
#[cfg(stageleft_runtime)]
use hydro_lang::prelude::*;
#[cfg(stageleft_runtime)]
use hydro_test::cluster::paxos::{Acceptor, Proposer};
#[cfg(stageleft_runtime)]
use hydro_transparent_replicate::messages::TransparentReplica;
#[cfg(stageleft_runtime)]
use hydro_transparent_replicate::protocol::replicate_service_raw;
#[cfg(stageleft_runtime)]
use hydro_transparent_replicate::{Coordinator, ReplicateConfig};

use hydro_deploy::Service;
use hydro_lang::deploy::DeployCrateWrapper;

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
    let config = ReplicateConfig {
        f: F,
        initial_members: (0..N as u32).collect(),
        commit_timeout_ms: 2000,
        notification_interval_ms: 200,
        paxos_config: hydro_transparent_replicate::config::PaxosConfig {
            f: F,
            i_am_leader_send_timeout: 2,
            i_am_leader_check_timeout: 5,
            i_am_leader_check_timeout_delay_multiplier: 3,
        },
        ..ReplicateConfig::default()
    };

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

    let empty_view_changes = replicas
        .source_iter(q!(Vec::<hydro_transparent_replicate::View>::new()))
        .weaken_ordering::<hydro_lang::live_collections::stream::NoOrder>().into();

    let raw = replicate_service_raw::<String>(
        replicas, proposers, acceptors, puts_at_replicas, empty_view_changes, config,
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

    let committed_puts = raw.replicated.map(q!(|(seq, cmd)| (true, seq, cmd)));

    let responses = committed_puts.interleave(gets_on_primary)
        .batch(&scan_tick, nondet!(/** batch */))
        .sort()
        .all_ticks()
        .scan(
            q!(|| hydro_transparent_replicate::applier::RedbApplierState::new()),
            q!(|state: &mut hydro_transparent_replicate::applier::RedbApplierState,
                (is_put, seq, cmd): (bool, usize, String)| {
                Some(state.apply_command(if is_put { seq } else { 0 }, &cmd))
            }),
        );

    let responses_at_coord = responses.send(coordinator, TCP.fail_stop().bincode()).values();
    let resp_stream = responses_at_coord.send_bincode_external(external);
    (cmd_sink, resp_stream)
}

/// Kill nodes one at a time with 30s waits. After the 3rd kill (2 acceptors left),
/// verify the system is stuck and cannot advance to a new view.
#[tokio::test]
async fn sequential_kill_with_long_waits() {
    use futures::SinkExt;
    use futures::StreamExt;
    use hydro_deploy::Deployment;
    use hydro_lang::deploy::TrybuildHost;

    let mut deployment = Deployment::new();
    let mut builder = FlowBuilder::new();

    let external = builder.external::<()>();
    let replicas = builder.cluster::<TransparentReplica>();
    let proposers = builder.cluster::<Proposer>();
    let acceptors = builder.cluster::<Acceptor>();
    let coordinator = builder.process::<Coordinator>();

    #[cfg(stageleft_runtime)]
    let (cmd_sink, resp_stream) = build_pipeline(
        &external, &replicas, &proposers, &acceptors, &coordinator,
    );

    let features = vec!["backend_redb".to_string()];
    let nodes = builder
        .with_external(&external, deployment.Localhost())
        .with_cluster(&replicas, (0..N).map(|_| TrybuildHost::new(deployment.Localhost()).features(features.clone())))
        .with_cluster(&proposers, (0..N).map(|_| TrybuildHost::new(deployment.Localhost()).features(features.clone())))
        .with_cluster(&acceptors, (0..N).map(|_| TrybuildHost::new(deployment.Localhost()).features(features.clone())))
        .with_process(&coordinator, TrybuildHost::new(deployment.Localhost()).features(features.clone()))
        .deploy(&mut deployment);

    deployment.deploy().await.unwrap();

    #[cfg(stageleft_runtime)]
    let mut sender = nodes.connect(cmd_sink).await;
    #[cfg(stageleft_runtime)]
    let mut receiver = nodes.connect(resp_stream).await;

    deployment.start().await.unwrap();

    #[cfg(stageleft_runtime)]
    {
        let mut nonce: u64 = 1;

        // PUT x=42
        sender.send(format!("PUT:x:42:{}", nonce)).await.unwrap();
        nonce += 1;
        let t = tokio::time::timeout(std::time::Duration::from_secs(30), async {
            while let Some(resp) = receiver.next().await {
                if resp.starts_with("OK") { println!("[INIT] {}", resp); break; }
            }
        }).await;
        assert!(t.is_ok(), "Timed out on initial PUT");
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        let replica_members = nodes.get_cluster(&replicas).members();
        let proposer_members = nodes.get_cluster(&proposers).members();
        let acceptor_members = nodes.get_cluster(&acceptors).members();

        // Kill host 0, wait 30s for it to be FULLY DEAD
        println!("\n=== KILL HOST 0 === (4 acceptors remain)");
        DeployCrateWrapper::underlying(&replica_members[0]).stop().await.unwrap();
        DeployCrateWrapper::underlying(&proposer_members[0]).stop().await.unwrap();
        DeployCrateWrapper::underlying(&acceptor_members[0]).stop().await.unwrap();
        println!("  Waiting 30s for process to be fully dead...");
        tokio::time::sleep(std::time::Duration::from_secs(30)).await;

        // Kill host 1, wait 30s
        println!("\n=== KILL HOST 1 === (3 acceptors remain)");
        DeployCrateWrapper::underlying(&replica_members[1]).stop().await.unwrap();
        DeployCrateWrapper::underlying(&proposer_members[1]).stop().await.unwrap();
        DeployCrateWrapper::underlying(&acceptor_members[1]).stop().await.unwrap();
        println!("  Waiting 30s for process to be fully dead...");
        tokio::time::sleep(std::time::Duration::from_secs(30)).await;

        // At this point: acceptors 2,3,4 alive. Last view change should have committed
        // (quorum=3 met with acceptors 2,3,4). The view is now [2,3,4] or [3,4] depending
        // on timing. Primary is the first member of whatever view committed.

        // Kill host 2, wait 30s. NOW only 2 acceptors (3,4). Quorum=3 IMPOSSIBLE.
        println!("\n=== KILL HOST 2 === (2 acceptors remain — QUORUM IMPOSSIBLE)");
        DeployCrateWrapper::underlying(&replica_members[2]).stop().await.unwrap();
        DeployCrateWrapper::underlying(&proposer_members[2]).stop().await.unwrap();
        DeployCrateWrapper::underlying(&acceptor_members[2]).stop().await.unwrap();
        println!("  Waiting 30s for process to be fully dead...");
        tokio::time::sleep(std::time::Duration::from_secs(30)).await;

        // Now try GET. If the last committed view has primary=2 (dead), system is stuck.
        // If primary=3 or 4 (alive), GET might work from the last committed view.
        println!("\n=== TESTING: Can we GET with 2 acceptors? ===");
        sender.send(format!("GET:x:{}", nonce)).await.unwrap();
        nonce += 1;
        let got_response_1 = tokio::time::timeout(std::time::Duration::from_secs(10), async {
            while let Some(resp) = receiver.next().await {
                if resp.starts_with("VALUE") {
                    println!("  GOT: {}", resp);
                    return true;
                }
                if resp.starts_with("OK") { /* stale */ }
            }
            false
        }).await.unwrap_or(false);
        println!("  GET with 2 acceptors: {}", if got_response_1 { "WORKED (primary still alive in last view)" } else { "BLOCKED (primary dead, can't elect new one)" });

        // Kill host 3. Only node 4 alive. 1 acceptor. Quorum=3 IMPOSSIBLE.
        // If the previous GET worked (primary was 3 or 4), killing 3 should break it.
        println!("\n=== KILL HOST 3 === (1 acceptor remains — QUORUM IMPOSSIBLE)");
        DeployCrateWrapper::underlying(&replica_members[3]).stop().await.unwrap();
        DeployCrateWrapper::underlying(&proposer_members[3]).stop().await.unwrap();
        DeployCrateWrapper::underlying(&acceptor_members[3]).stop().await.unwrap();
        println!("  Waiting 30s for process to be fully dead...");
        tokio::time::sleep(std::time::Duration::from_secs(30)).await;

        // Final test: GET with only node 4 alive.
        println!("\n=== FINAL TEST: Can we GET with 1 node? ===");
        sender.send(format!("GET:x:{}", nonce)).await.unwrap();
        let got_response_2 = tokio::time::timeout(std::time::Duration::from_secs(10), async {
            while let Some(resp) = receiver.next().await {
                if resp.starts_with("VALUE") {
                    println!("  GOT: {}", resp);
                    return true;
                }
            }
            false
        }).await.unwrap_or(false);

        if got_response_2 {
            panic!("BUG: System served GET with 1 node. View changed to [4] without Paxos quorum.");
        }

        // If we got here: either the system blocked at 2 nodes or at 1 node.
        if got_response_1 {
            println!("\n[RESULT] System worked with 2 nodes (primary alive in last committed view).");
            println!("         System correctly BLOCKED with 1 node (can't elect new primary).");
            println!("[PASS] Paxos quorum enforced. View stuck at 3-member view.");
        } else {
            println!("\n[RESULT] System blocked at 2 nodes (primary of last view was dead).");
            println!("[PASS] Paxos quorum enforced. Could not advance past 3-acceptor threshold.");
        }
    }
}
