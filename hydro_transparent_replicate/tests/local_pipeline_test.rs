//! Local test of the full pipeline using Localhost — no EC2 needed.
//! Verifies: 100 PUTs committed, 5 GETs return correct values from the same replica state.

#[cfg(stageleft_runtime)]
use hydro_lang::location::cluster::CLUSTER_SELF_ID;
#[cfg(stageleft_runtime)]
use hydro_lang::prelude::*;
#[cfg(stageleft_runtime)]
use hydro_transparent_replicate::messages::TransparentReplica;
#[cfg(stageleft_runtime)]
use hydro_transparent_replicate::protocol::core_replication_module;
#[cfg(stageleft_runtime)]
use hydro_transparent_replicate::Coordinator;

#[cfg(stageleft_runtime)]
fn build_local_pipeline<'a>(
    external: &External<'a, ()>,
    replicas: &Cluster<'a, TransparentReplica>,
    coordinator: &Process<'a, Coordinator>,
) -> (
    hydro_lang::location::external_process::ExternalBincodeSink<String>,
    hydro_lang::location::external_process::ExternalBincodeStream<
        String,
        hydro_lang::live_collections::stream::NoOrder,
    >,
) {
    let (cmd_sink, cmds_at_coord) = coordinator.source_external_bincode(external);

    let puts_at_coord = cmds_at_coord
        .clone()
        .filter(q!(|cmd: &String| cmd.starts_with("PUT:")));
    let gets_at_coord = cmds_at_coord
        .filter(q!(|cmd: &String| cmd.starts_with("GET:")));

    // PUTs go through the replication protocol.
    let puts_at_replicas: Stream<String, Cluster<'a, TransparentReplica>, Unbounded> =
        puts_at_coord
            .broadcast(replicas, TCP.fail_stop().bincode(), nondet!(/** broadcast PUTs */))
            .assume_ordering(nondet!(/** ordering */));

    // GETs are broadcast to all replicas; only the primary responds.
    let gets_at_replicas: Stream<String, Cluster<'a, TransparentReplica>, Unbounded> =
        gets_at_coord
            .broadcast(replicas, TCP.fail_stop().bincode(), nondet!(/** broadcast GETs */));

    let read_only: Stream<String, _, Unbounded> = replicas
        .source_iter(q!(Vec::<String>::new()))
        .into();

    let current_view: Singleton<hydro_transparent_replicate::View, _, _> = replicas
        .source_iter(q!([hydro_transparent_replicate::View {
            view_num: 0,
            members: vec![0, 1, 2]
        }]))
        .fold(
            q!(|| hydro_transparent_replicate::View {
                view_num: 0,
                members: vec![0, 1, 2]
            }),
            q!(|current, new| { *current = new; }),
        )
        .into();

    let reconciled_seq: Optional<usize, _, Unbounded> = replicas
        .source_iter(q!(Vec::<usize>::new()))
        .max()
        .into();

    let tick = replicas.tick();
    let core = core_replication_module(
        replicas,
        &tick,
        puts_at_replicas,
        read_only,
        current_view.clone(),
        reconciled_seq,
    );

    // Filter GETs to primary only.
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

    // Replicated PUTs tagged for the scan (fires on ALL replicas).
    let committed_puts = core.replicated
        .map(q!(|(seq, cmd)| (true, seq, cmd)));

    // One scan per replica: applies committed PUTs and serves GETs from the same state.
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

#[tokio::test]
async fn local_put_get_verify() {
    use futures::SinkExt;
    use futures::StreamExt;
    use hydro_deploy::Deployment;
    use hydro_lang::deploy::TrybuildHost;

    let mut deployment = Deployment::new();
    let mut builder = FlowBuilder::new();

    let external = builder.external::<()>();
    let replicas = builder.cluster::<TransparentReplica>();
    let coordinator = builder.process::<Coordinator>();

    #[cfg(stageleft_runtime)]
    let (cmd_sink, resp_stream) =
        build_local_pipeline(&external, &replicas, &coordinator);

    let features = vec!["backend_redb".to_string()];
    let nodes = builder
        .with_external(&external, deployment.Localhost())
        .with_cluster(
            &replicas,
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

    let mut rng: u64 = 0xdeadbeef;
    let mut expected: Vec<u64> = Vec::new();
    for _ in 0..100 {
        rng ^= rng << 13;
        rng ^= rng >> 7;
        rng ^= rng << 17;
        expected.push(rng % 1_000_000);
    }

    // Send 100 PUTs with nonces.
    let mut nonce: u64 = 1;
    #[cfg(stageleft_runtime)]
    for i in 0..100usize {
        sender
            .send(format!("PUT:k{}:{}:{}", i, expected[i], nonce))
            .await
            .unwrap();
        nonce += 1;
    }

    // Wait for 100 PUT acks.
    #[cfg(stageleft_runtime)]
    {
        let mut n = 0;
        let t = tokio::time::timeout(std::time::Duration::from_secs(60), async {
            while let Some(resp) = receiver.next().await {
                if resp.starts_with("OK") {
                    n += 1;
                    if n >= 100 { break; }
                }
            }
        })
        .await;
        assert!(t.is_ok(), "Timed out waiting for PUTs");
        println!("100 PUTs committed.");
    }

    // Send 5 GETs with nonces.
    let verify_keys = [0usize, 20, 40, 60, 80];
    #[cfg(stageleft_runtime)]
    for &k in &verify_keys {
        sender.send(format!("GET:k{}:{}", k, nonce)).await.unwrap();
        nonce += 1;
    }

    // Collect 5 GET responses and verify values.
    #[cfg(stageleft_runtime)]
    {
        let mut responses: Vec<String> = Vec::new();
        let t = tokio::time::timeout(std::time::Duration::from_secs(30), async {
            while let Some(resp) = receiver.next().await {
                if resp.starts_with("VALUE") {
                    println!("  {}", resp);
                    responses.push(resp);
                    if responses.len() >= 5 { break; }
                }
            }
        })
        .await;
        assert!(t.is_ok(), "Timed out waiting for GETs");
        assert_eq!(responses.len(), 5, "Expected 5 GET responses");

        for resp in &responses {
            if let Some(get_part) = resp.split("GET ").nth(1) {
                let mut kv = get_part.splitn(2, '=');
                if let (Some(key_str), Some(val_str)) = (kv.next(), kv.next()) {
                    if let Ok(key_idx) = key_str.trim_start_matches('k').parse::<usize>() {
                        if key_idx < expected.len() {
                            assert_eq!(
                                val_str.trim(),
                                expected[key_idx].to_string(),
                                "Wrong value for k{}: expected {}, got {}",
                                key_idx,
                                expected[key_idx],
                                val_str.trim()
                            );
                        }
                    }
                }
            }
        }
        println!("PASS: all GET responses correct");
    }
}
