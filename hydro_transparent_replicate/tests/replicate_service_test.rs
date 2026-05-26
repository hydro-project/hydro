//! Local tests of the full replicate_service_raw pipeline with RedbApplierState.
//! Uses Paxos-backed view changes — same pipeline as ec2_demo2.

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

    // No failover in this test — empty proposals stream.
    let empty_tick = replicas.tick();
    let empty_proposals =
        replicas.source_iter(q!(Vec::<hydro_transparent_replicate::View>::new()))
            .batch(&empty_tick, nondet!(/** empty */))
            .weaken_ordering::<hydro_lang::live_collections::stream::NoOrder>()
            .all_ticks();

    let raw = replicate_service_raw::<String>(
        replicas, proposers, acceptors, puts_at_replicas, empty_proposals, config,
    );

    let scan_tick = replicas.tick();

    // Heartbeat keeps scan_tick firing so is_primary updates after view changes.
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

// ─── Test 1: Basic PUT/GET ────────────────────────────────────────────────────

#[tokio::test]
async fn replicate_service_put_get() {
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
        build_pipeline(&external, &replicas, &proposers, &acceptors, &coordinator, ReplicateConfig::default());

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

    let values = vec![42u64, 100, 200, 300, 400, 500, 600, 700, 800, 999];
    let mut nonce: u64 = 1;

    #[cfg(stageleft_runtime)]
    for (i, &v) in values.iter().enumerate() {
        sender.send(format!("PUT:k{}:{}:{}", i, v, nonce)).await.unwrap();
        nonce += 1;
    }

    #[cfg(stageleft_runtime)]
    {
        let mut n = 0;
        let t = tokio::time::timeout(std::time::Duration::from_secs(60), async {
            while let Some(resp) = receiver.next().await {
                if resp.starts_with("OK") { n += 1; if n >= 10 { break; } }
            }
        }).await;
        assert!(t.is_ok(), "Timed out waiting for PUTs");
        println!("10 PUTs committed.");
    }

    let get_keys = [0usize, 5, 9];
    #[cfg(stageleft_runtime)]
    for &k in &get_keys {
        sender.send(format!("GET:k{}:{}", k, nonce)).await.unwrap();
        nonce += 1;
    }

    #[cfg(stageleft_runtime)]
    {
        let mut responses: Vec<String> = Vec::new();
        let t = tokio::time::timeout(std::time::Duration::from_secs(30), async {
            while let Some(resp) = receiver.next().await {
                if resp.starts_with("VALUE") {
                    println!("  {}", resp);
                    responses.push(resp);
                    if responses.len() >= 3 { break; }
                }
            }
        }).await;
        assert!(t.is_ok(), "Timed out waiting for GETs");
        assert_eq!(responses.len(), 3, "Expected 3 GET responses");

        for resp in &responses {
            if let Some(get_part) = resp.split("GET ").nth(1) {
                let mut kv = get_part.splitn(2, '=');
                if let (Some(key_str), Some(val_str)) = (kv.next(), kv.next()) {
                    if let Ok(key_idx) = key_str.trim_start_matches('k').parse::<usize>() {
                        if key_idx < values.len() {
                            assert_eq!(
                                val_str.trim(),
                                values[key_idx].to_string(),
                                "Wrong value for k{}: expected {}, got {}",
                                key_idx, values[key_idx], val_str.trim()
                            );
                        }
                    }
                }
            }
        }
        println!("PASS: all GET responses correct via replicate_service_raw + Paxos");
    }
}

// ─── Test 2: Failover ─────────────────────────────────────────────────────────

// ─── Tests 2 & 3: Failover and View Catchup ──────────────────────────────────
// REMOVED: These tests used the replica-side failure detector which has been
// deprecated in favor of coordinator-driven failure detection (task 21).
// The demo (ec2_demo2.rs) implements coordinator-driven failover correctly.
// TODO: Rewrite these tests to use the coordinator-driven approach.
