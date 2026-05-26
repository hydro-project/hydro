//! Failover test — same pipeline as ec2_demo2.rs.
//! PUTs, kills primary, sends another PUT to trigger FD, verifies GETs work.

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

const F: usize = 2;
const N: usize = 2 * F + 1;

// Same pipeline as ec2_demo2.rs
#[cfg(stageleft_runtime)]
fn build_pipeline<'a>(
    external: &External<'a, ()>,
    replicas: &Cluster<'a, TransparentReplica>,
    proposers: &Cluster<'a, Proposer>,
    acceptors: &Cluster<'a, Acceptor>,
    coordinator: &Process<'a, Coordinator>,
    config: ReplicateConfig,
) -> (
    ExternalBincodeSink<String>,
    ExternalBincodeStream<String, hydro_lang::live_collections::stream::NoOrder>,
) {
+    use hydro_transparent_replicate::protocol::coordinator_failure_detector;
+
+    let initial_member_count = config.initial_members.len();
+    let timeout_ms = config.commit_timeout_ms;
+
    let (cmd_sink, cmds_at_coord) = coordinator.source_external_bincode(external);

+    // Clone cmds for the FD to track outstanding requests.
+
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

+    // Forward ref to break the cycle: FD needs responses, responses need raw output.
+    let (proposals_complete, proposals_ref) =
+        replicas.forward_ref::<Stream<hydro_transparent_replicate::View, _, Unbounded, hydro_lang::live_collections::stream::NoOrder>>();
+
    let raw = replicate_service_raw::<String>(
-        replicas, proposers, acceptors, puts_at_replicas, config,
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
+
+    // Coordinator-driven failure detection.
+    // requests = all commands sent by coordinator, responses = replies from replicas.
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
async fn failover_after_put() {
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

    let config = ReplicateConfig {
        f: F,
        initial_members: (0..N as u32).collect(),
        commit_timeout_ms: 3000,
        notification_interval_ms: 200,
        paxos_config: hydro_transparent_replicate::config::PaxosConfig {
            f: F,
            i_am_leader_send_timeout: 10,
            i_am_leader_check_timeout: 30,
            i_am_leader_check_timeout_delay_multiplier: 15,
        },
        ..ReplicateConfig::default()
    };

    #[cfg(stageleft_runtime)]
    let (cmd_sink, resp_stream) = build_pipeline(
        &external, &replicas, &proposers, &acceptors, &coordinator, config,
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

    let mut nonce: u64 = 1;

    // Step 1: PUT x=42
    #[cfg(stageleft_runtime)]
    {
        sender.send(format!("PUT:x:42:{}", nonce)).await.unwrap();
        nonce += 1;
        let t = tokio::time::timeout(std::time::Duration::from_secs(30), async {
            while let Some(resp) = receiver.next().await {
                if resp.starts_with("OK") { println!("  {}", resp); break; }
            }
        }).await;
        assert!(t.is_ok(), "Timed out waiting for PUT x");
    }

    // Step 2: Wait for notification with commits to propagate (200ms interval).
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    // Step 3: Kill primary.
    println!("Killing primary (replica 0, proposer 0)...");
    {
        use hydro_deploy::Service;
        use hydro_lang::deploy::DeployCrateWrapper;
        nodes.get_cluster(&replicas).members()[0].underlying().stop().await.unwrap();
        nodes.get_cluster(&proposers).members()[0].underlying().stop().await.unwrap();
    }

    // Step 4: Send another PUT to keep the system active (triggers FD via lack of response).
    #[cfg(stageleft_runtime)]
    {
        sender.send(format!("PUT:y:99:{}", nonce)).await.unwrap();
        nonce += 1;
    }

    // Step 5: Wait for failover and retry GETs.
    #[cfg(stageleft_runtime)]
    {
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(30);
        let mut got_response = false;

        while !got_response {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() { break; }

            sender.send(format!("GET:x:{}", nonce)).await.unwrap();
            nonce += 1;

            let t = tokio::time::timeout(std::time::Duration::from_secs(3), async {
                while let Some(resp) = receiver.next().await {
                    if resp.starts_with("VALUE") { return Some(resp); }
                    if resp.starts_with("OK") { println!("  (PUT ack: {})", resp); }
                }
                None
            }).await;

            match t {
                Ok(Some(resp)) => {
                    println!("  Post-failover GET: {}", resp);
                    assert!(resp.contains("x=42"), "Wrong value: {}", resp);
                    got_response = true;
                }
                _ => {
                    println!("  (no response, retrying...)");
                    // Send another PUT to keep FD active
                    sender.send(format!("PUT:z:{}:{}", nonce, nonce)).await.unwrap();
                    nonce += 1;
                }
            }
        }

        assert!(got_response, "Failover never completed — GETs timed out after 30s");
        println!("PASS: failover works");
    }
}
