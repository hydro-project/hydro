//! Failover test: write keys, kill primary, read keys back from new primary.
//!
//! Uses f=1 (3 replicas). Kills replica 0 (the initial primary).
//! Verifies that after the view change, reads return correct values.

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
fn build_failover_pipeline<'a>(
    external: &External<'a, ()>,
    replicas: &Cluster<'a, TransparentReplica>,
    proposers: &Cluster<'a, Proposer>,
    acceptors: &Cluster<'a, Acceptor>,
    coordinator: &Process<'a, Coordinator>,
) -> (
    ExternalBincodeSink<String>,
    ExternalBincodeStream<String, hydro_lang::live_collections::stream::NoOrder>,
) {
+    use hydro_transparent_replicate::protocol::coordinator_failure_detector;
+
    let config = ReplicateConfig {
        f: 1,
        initial_members: vec![0, 1, 2],
-        commit_timeout_ms: 2000,       // short timeout for fast failover
-        notification_interval_ms: 500,  // frequent notifications
+        commit_timeout_ms: 2000,
+        notification_interval_ms: 500,
        backup_apply: true,
        paxos_config: hydro_transparent_replicate::config::PaxosConfig {
            f: 1,
            i_am_leader_send_timeout: 5,
            i_am_leader_check_timeout: 10,
            i_am_leader_check_timeout_delay_multiplier: 15,
        },
    };

+    let initial_member_count = config.initial_members.len();
+    let timeout_ms = config.commit_timeout_ms;
+
    let (cmd_sink, cmds_at_coord) = coordinator.source_external_bincode(external);

+
    let puts_at_coord = cmds_at_coord.clone()
        .filter(q!(|cmd: &String| cmd.starts_with("PUT:")));
    let gets_at_coord = cmds_at_coord
        .filter(q!(|cmd: &String| cmd.starts_with("GET:")));

-    // PUTs broadcast to all replicas (only primary sequences them)
    let puts_at_replicas: Stream<String, Cluster<'a, TransparentReplica>, Unbounded> =
        puts_at_coord
            .broadcast(replicas, TCP.fail_stop().bincode(), nondet!(/** broadcast PUTs */))
            .assume_ordering(nondet!(/** ordering */));

-    // GETs broadcast to all replicas (only primary responds)
    let gets_at_replicas: Stream<String, Cluster<'a, TransparentReplica>, Unbounded> =
        gets_at_coord
            .broadcast(replicas, TCP.fail_stop().bincode(), nondet!(/** broadcast GETs */));

+    // Forward ref to break the cycle.
+    let (proposals_complete, proposals_ref) =
+        replicas.forward_ref::<Stream<hydro_transparent_replicate::View, _, Unbounded, hydro_lang::live_collections::stream::NoOrder>>();
+
    let raw = replicate_service_raw::<String>(
-        replicas,
-        proposers,
-        acceptors,
-        puts_at_replicas,
-        config,
+        replicas, proposers, acceptors, puts_at_replicas, proposals_ref, config,
    );

    let scan_tick = replicas.tick();

    let _hb = replicas
        .source_interval(q!(std::time::Duration::from_millis(100)), nondet!(/** heartbeat */))
        .batch(&scan_tick, nondet!(/** heartbeat tick */));

-    let is_primary = raw.current_view
+    let is_primary = raw.current_view.clone()
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
            q!(|| hydro_transparent_replicate::applier::RedbApplierState::new()),
            q!(|state: &mut hydro_transparent_replicate::applier::RedbApplierState,
                (is_put, seq, cmd): (bool, usize, String)| {
-                let resp = state.apply_command(if is_put { seq } else { 0 }, &cmd);
-                // Only emit responses for operations that need client responses
-                // PUTs: always respond (primary needs to ack to client)
-                // GETs: always respond (primary serves reads)
-                Some(resp)
+                Some(state.apply_command(if is_put { seq } else { 0 }, &cmd))
            }),
        );

    let responses_at_coord = responses
        .send(coordinator, TCP.fail_stop().bincode())
        .values();

+    // Coordinator-driven failure detection.
+    let fd_proposals = coordinator_failure_detector(
+        coordinator,
+        replicas,
+        responses_at_coord.clone(),
+        raw.current_view,
+        timeout_ms,
+        initial_member_count,
+    );
+    proposals_complete.complete(fd_proposals);
+
    let resp_stream = responses_at_coord.send_bincode_external(external);
    (cmd_sink, resp_stream)
}

#[tokio::test]
async fn failover_read_after_write() {
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
        build_failover_pipeline(&external, &replicas, &proposers, &acceptors, &coordinator);

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

    // Phase 1: Write some keys while primary is alive
    let mut nonce: u64 = 1;

    #[cfg(stageleft_runtime)]
    {
        // Write 3 keys
        for (k, v) in [("x", "42"), ("y", "severine"), ("z", "100")] {
            sender.send(format!("PUT:{}:{}:{}", k, v, nonce)).await.unwrap();
            nonce += 1;
        }

        // Wait for 3 PUT acks
        let mut put_acks = 0;
        let t = tokio::time::timeout(std::time::Duration::from_secs(30), async {
            while let Some(resp) = receiver.next().await {
                if resp.starts_with("OK") {
                    put_acks += 1;
                    println!("[PRE-CRASH] PUT ack {}: {}", put_acks, resp);
                    if put_acks >= 3 { break; }
                }
            }
        }).await;
        assert!(t.is_ok(), "Timed out waiting for PUTs before crash");
        println!("[PRE-CRASH] All 3 PUTs committed.");

        // Wait for notification broadcaster to send at least one notification with commits.
        // This ensures the failure detector's cold-start gate is satisfied.
        println!("[PRE-CRASH] Waiting 2s for notifications to propagate...");
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }

    // Phase 2: Kill the primary (replica 0)
    #[cfg(stageleft_runtime)]
    {
        let replica_members = nodes.get_cluster(&replicas).members();
        println!("[CRASH] Killing replica 0 (primary)...");
        DeployCrateWrapper::underlying(&replica_members[0]).stop().await.unwrap();
        println!("[CRASH] Replica 0 is dead.");

        // Wait for the view change to complete (commit_timeout_ms = 2000ms + Paxos round)
        println!("[FAILOVER] Waiting for view change (up to 15s)...");
        tokio::time::sleep(std::time::Duration::from_secs(12)).await;
    }

    // Phase 3: Read keys back from the new primary
    #[cfg(stageleft_runtime)]
    {
        println!("[POST-CRASH] Sending GETs...");
        for k in ["x", "y", "z"] {
            sender.send(format!("GET:{}:{}", k, nonce)).await.unwrap();
            nonce += 1;
        }

        let mut get_responses: Vec<String> = Vec::new();
        let t = tokio::time::timeout(std::time::Duration::from_secs(30), async {
            while let Some(resp) = receiver.next().await {
                if resp.starts_with("VALUE") {
                    println!("[POST-CRASH] {}", resp);
                    get_responses.push(resp);
                    if get_responses.len() >= 3 { break; }
                }
            }
        }).await;

        if t.is_err() {
            panic!(
                "FAILOVER FAILED: Timed out waiting for GETs after primary crash. \
                 Got {} responses: {:?}. \
                 The view change may not have completed, or the new primary is not serving reads.",
                get_responses.len(), get_responses
            );
        }

        // Verify values
        assert!(get_responses.iter().any(|r| r.contains("GET x=42")),
            "x should be 42: {:?}", get_responses);
        assert!(get_responses.iter().any(|r| r.contains("GET y=severine")),
            "y should be severine: {:?}", get_responses);
        assert!(get_responses.iter().any(|r| r.contains("GET z=100")),
            "z should be 100: {:?}", get_responses);

        println!("[PASS] All reads correct after primary failover!");
    }
}
