//! Test: kill replicas one by one, observe whether view changes commit
//! when Paxos should NOT have quorum.

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
const N: usize = 2 * F + 1; // 5

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

/// Kill replicas one by one. After killing 3 (leaving only 2 acceptors),
/// Paxos should NOT be able to commit new views. Verify behavior.
#[tokio::test]
async fn systematic_kill_paxos_quorum_loss() {
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

        // Step 1: PUT x=42
        sender.send(format!("PUT:x:42:{}", nonce)).await.unwrap();
        nonce += 1;
        let t = tokio::time::timeout(std::time::Duration::from_secs(30), async {
            while let Some(resp) = receiver.next().await {
                if resp.starts_with("OK") { println!("[STEP1] {}", resp); break; }
            }
        }).await;
        assert!(t.is_ok(), "Timed out on initial PUT");

        // Wait for system to stabilize
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;

        let replica_members = nodes.get_cluster(&replicas).members();
        let proposer_members = nodes.get_cluster(&proposers).members();
        let acceptor_members = nodes.get_cluster(&acceptors).members();

        // Kill hosts 0, 1, 2 all at once — no time for race conditions
        println!("\n[KILL] Killing hosts 0, 1, 2 simultaneously...");
        DeployCrateWrapper::underlying(&replica_members[0]).stop().await.unwrap();
        DeployCrateWrapper::underlying(&proposer_members[0]).stop().await.unwrap();
        DeployCrateWrapper::underlying(&acceptor_members[0]).stop().await.unwrap();
        DeployCrateWrapper::underlying(&replica_members[1]).stop().await.unwrap();
        DeployCrateWrapper::underlying(&proposer_members[1]).stop().await.unwrap();
        DeployCrateWrapper::underlying(&acceptor_members[1]).stop().await.unwrap();
        DeployCrateWrapper::underlying(&replica_members[2]).stop().await.unwrap();
        DeployCrateWrapper::underlying(&proposer_members[2]).stop().await.unwrap();
        DeployCrateWrapper::underlying(&acceptor_members[2]).stop().await.unwrap();

        println!("[KILL] All 3 hosts killed. Only acceptors 3,4 remain. Paxos quorum=3 IMPOSSIBLE.");
        println!("[WAIT] Waiting 20s to ensure processes are fully dead...");
        tokio::time::sleep(std::time::Duration::from_secs(20)).await;

        // Now try a GET. The last committed view should be [0,1,2,3,4] (no view change
        // could have committed because we killed 3 at once). Primary=0 is dead.
        // System should be UNAVAILABLE.
        println!("[TEST] Sending GET after 20s wait...");
        sender.send(format!("GET:x:{}", nonce)).await.unwrap();
        nonce += 1;
        let t = tokio::time::timeout(std::time::Duration::from_secs(15), async {
            while let Some(resp) = receiver.next().await {
                if resp.starts_with("VALUE") {
                    println!("[TEST] GOT RESPONSE: {}", resp);
                    return Some(resp);
                }
                if resp.starts_with("OK") { println!("[TEST] (stale: {})", resp); }
            }
            None
        }).await;

        match t {
            Ok(Some(resp)) => {
                println!("[TEST] *** BUG CONFIRMED: Got '{}' with only 2 acceptors ***", resp);
                println!("[TEST] This means a view change committed WITHOUT Paxos quorum.");
                panic!("BUG: View change committed without Paxos quorum (only 2/5 acceptors alive).");
            }
            Ok(None) | Err(_) => {
                println!("[TEST] No response — system correctly unavailable.");
                println!("[PASS] Paxos correctly prevents view changes without quorum.");
            }
        }
    }
}
