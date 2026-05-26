//! View maximality test: after killing the primary, the new view MUST include
//! ALL surviving replicas. Proves this by doing two successive failovers:
//!   1. Kill replica 0 → view should be [1, 2]
//!   2. Kill replica 1 (new primary) → replica 2 takes over
//! If the first view change was non-maximal (e.g., [1] only), the second
//! failover would fail because replica 2 wouldn't be in the view.

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
    use hydro_transparent_replicate::protocol::coordinator_failure_detector;

    let config = ReplicateConfig {
        f: 1,
        initial_members: vec![0, 1, 2],
        commit_timeout_ms: 2000,
        notification_interval_ms: 500,
        backup_apply: true,
        paxos_config: hydro_transparent_replicate::config::PaxosConfig {
            f: 1,
            i_am_leader_send_timeout: 5,
            i_am_leader_check_timeout: 10,
            i_am_leader_check_timeout_delay_multiplier: 15,
        },
    };

    let initial_member_count = config.initial_members.len();
    let timeout_ms = config.commit_timeout_ms;

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

    let (proposals_complete, proposals_ref) =
        replicas.forward_ref::<Stream<hydro_transparent_replicate::View, _, Unbounded, hydro_lang::live_collections::stream::NoOrder>>();

    let raw = replicate_service_raw::<String>(
        replicas, proposers, acceptors, puts_at_replicas, proposals_ref, config,
    );

    let scan_tick = replicas.tick();
    let _hb = replicas
        .source_interval(q!(std::time::Duration::from_millis(100)), nondet!(/** heartbeat */))
        .batch(&scan_tick, nondet!(/** heartbeat tick */));

    let is_primary = raw.current_view.clone()
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

    let fd_proposals = coordinator_failure_detector(
        coordinator, replicas, responses_at_coord.clone(),
        raw.current_view, timeout_ms, initial_member_count,
    );
    proposals_complete.complete(fd_proposals);

    let resp_stream = responses_at_coord.send_bincode_external(external);
    (cmd_sink, resp_stream)
}

/// Helper: send GETs and wait for responses, retrying on timeout.
#[cfg(stageleft_runtime)]
async fn get_with_retry(
    sender: &mut (impl futures::Sink<String, Error = impl std::fmt::Debug> + Unpin),
    receiver: &mut (impl futures::Stream<Item = String> + Unpin),
    key: &str,
    nonce: &mut u64,
) -> Option<String> {
    use futures::{SinkExt, StreamExt};

    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(30);
    while tokio::time::Instant::now() < deadline {
        sender.send(format!("GET:{}:{}", key, *nonce)).await.unwrap();
        *nonce += 1;

        match tokio::time::timeout(std::time::Duration::from_secs(3), async {
            while let Some(resp) = receiver.next().await {
                if resp.starts_with("VALUE") { return Some(resp); }
            }
            None
        }).await {
            Ok(Some(resp)) => return Some(resp),
            _ => {
                // Send a PUT to keep the FD active (creates outstanding requests)
                sender.send(format!("PUT:__probe__:1:{}", *nonce)).await.unwrap();
                *nonce += 1;
            }
        }
    }
    None
}

#[tokio::test]
async fn view_maximality_two_successive_failovers() {
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
    let (cmd_sink, resp_stream) =
        build_pipeline(&external, &replicas, &proposers, &acceptors, &coordinator);

    let features = vec!["backend_redb".to_string()];
    let nodes = builder
        .with_external(&external, deployment.Localhost())
        .with_cluster(
            &replicas,
            (0..3).map(|_| TrybuildHost::new(deployment.Localhost()).features(features.clone())),
        )
        .with_cluster(
            &proposers,
            (0..3).map(|_| TrybuildHost::new(deployment.Localhost()).features(features.clone())),
        )
        .with_cluster(
            &acceptors,
            (0..3).map(|_| TrybuildHost::new(deployment.Localhost()).features(features.clone())),
        )
        .with_process(
            &coordinator,
            TrybuildHost::new(deployment.Localhost()).features(features.clone()),
        )
        .deploy(&mut deployment);

    deployment.deploy().await.unwrap();

    #[cfg(stageleft_runtime)]
    let mut sender = nodes.connect(cmd_sink).await;
    #[cfg(stageleft_runtime)]
    let mut receiver = nodes.connect(resp_stream).await;

    deployment.start().await.unwrap();

    let mut nonce: u64 = 1;

    // Phase 1: Write a key while all 3 replicas are alive.
    #[cfg(stageleft_runtime)]
    {
        sender.send(format!("PUT:maximal:proof:{}", nonce)).await.unwrap();
        nonce += 1;

        let t = tokio::time::timeout(std::time::Duration::from_secs(30), async {
            while let Some(resp) = receiver.next().await {
                if resp.starts_with("OK") {
                    println!("[PHASE1] PUT committed: {}", resp);
                    break;
                }
            }
        }).await;
        assert!(t.is_ok(), "Timed out waiting for initial PUT");

        // Let the system stabilize.
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }

    // Phase 2: Kill replica 0 (initial primary). View should become [1, 2].
    #[cfg(stageleft_runtime)]
    {
        println!("[PHASE2] Killing replica 0 (initial primary)...");
        let replica_members = nodes.get_cluster(&replicas).members();
        DeployCrateWrapper::underlying(&replica_members[0]).stop().await.unwrap();

        // Wait for view change.
        println!("[PHASE2] Waiting for first failover...");
        tokio::time::sleep(std::time::Duration::from_secs(10)).await;

        // Verify reads work from new primary (replica 1).
        let resp = get_with_retry(&mut sender, &mut receiver, "maximal", &mut nonce).await;
        assert!(resp.is_some(), "First failover failed — no response to GET");
        let resp = resp.unwrap();
        assert!(resp.contains("maximal=proof"), "Wrong value after first failover: {}", resp);
        println!("[PHASE2] First failover OK: {}", resp);
    }

    // Phase 3: Kill replica 1 (new primary after first failover).
    // If the first view change was maximal [1, 2], then replica 2 is in the view
    // and can take over. If it was non-maximal [1], this will fail.
    #[cfg(stageleft_runtime)]
    {
        println!("[PHASE3] Killing replica 1 (second primary)...");
        let replica_members = nodes.get_cluster(&replicas).members();
        DeployCrateWrapper::underlying(&replica_members[1]).stop().await.unwrap();

        // Wait for second view change.
        println!("[PHASE3] Waiting for second failover...");
        tokio::time::sleep(std::time::Duration::from_secs(10)).await;

        // Verify reads work from replica 2 (the last survivor).
        let resp = get_with_retry(&mut sender, &mut receiver, "maximal", &mut nonce).await;
        assert!(
            resp.is_some(),
            "MAXIMALITY VIOLATION: Second failover failed. \
             This means the first view change was NOT maximal — \
             replica 2 was excluded from the view [1, 2] should have been the view after killing replica 0."
        );
        let resp = resp.unwrap();
        assert!(resp.contains("maximal=proof"), "Wrong value after second failover: {}", resp);
        println!("[PHASE3] Second failover OK: {}", resp);
        println!("[PASS] View maximality proven: both failovers succeeded, all surviving nodes were in the view.");
    }
}
