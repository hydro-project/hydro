//! Non-interactive test of the full pipeline: PUT 1000 keys, GET 10, verify values.
//! Runs locally with Localhost() — no EC2 needed.
//! This lets us iterate on the verification logic without deploying.

#[cfg(stageleft_runtime)]
use hydro_lang::live_collections::stream::NoOrder;
#[cfg(stageleft_runtime)]
use hydro_lang::location::cluster::CLUSTER_SELF_ID;
#[cfg(stageleft_runtime)]
use hydro_lang::location::MemberId;
#[cfg(stageleft_runtime)]
use hydro_lang::prelude::*;
#[cfg(stageleft_runtime)]
use hydro_transparent_replicate::messages::TransparentReplica;
#[cfg(stageleft_runtime)]
use hydro_transparent_replicate::protocol::core_replication_module;
#[cfg(stageleft_runtime)]
use hydro_transparent_replicate::{Coordinator, ReplicaDb};

#[cfg(stageleft_runtime)]
fn build_pipeline<'a>(
    external: &External<'a, ()>,
    replicas: &Cluster<'a, TransparentReplica>,
    replica_dbs: &Cluster<'a, ReplicaDb>,
    coordinator: &Process<'a, Coordinator>,
) -> (
    hydro_lang::location::external_process::ExternalBincodeSink<String>,
    hydro_lang::location::external_process::ExternalBincodeStream<
        String,
        hydro_lang::live_collections::stream::NoOrder,
    >,
) {
    let (cmd_sink, cmds_at_coord) = coordinator.source_external_bincode(external);

    let cmds_at_replica_dbs: Stream<String, Cluster<'a, ReplicaDb>, Unbounded> =
        cmds_at_coord.broadcast(replica_dbs, TCP.fail_stop().bincode(), nondet!(/** broadcast */));

    let cmds_at_replicas: Stream<String, Cluster<'a, TransparentReplica>, Unbounded> =
        cmds_at_replica_dbs
            .broadcast(replicas, TCP.fail_stop().bincode(), nondet!(/** broadcast */))
            .values()
            .assume_ordering(nondet!(/** ordering within tick */));

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
        cmds_at_replicas,
        read_only,
        current_view,
        reconciled_seq,
    );

    // Each replica sends committed to its co-located replica_db.
    let committed_at_replica_dbs = core
        .committed
        .map(q!(move |(seq, cmd)| (
            hydro_transparent_replicate::MemberId::<hydro_transparent_replicate::ReplicaDb>::from_raw_id(
                CLUSTER_SELF_ID.get_raw_id()
            ),
            (seq, cmd)
        )))
        .demux(replica_dbs, TCP.fail_stop().bincode())
        .values();

    let replica_db_tick = replica_dbs.tick();
    let responses_at_replica_dbs = committed_at_replica_dbs
        .batch(&replica_db_tick, nondet!(/** batch */))
        .sort()
        .across_ticks(|s| {
            s.fold(
                q!(|| (
                    hydro_transparent_replicate::applier::RedbApplierState::new(),
                    Vec::<String>::new()
                )),
                q!(|(state, responses): &mut (
                    hydro_transparent_replicate::applier::RedbApplierState,
                    Vec<String>
                ),
                   (seq, cmd): (usize, String)| {
                    let response = state.apply_command(seq, &cmd);
                    responses.push(response);
                }),
            )
        })
        .map(q!(|(_, responses)| responses))
        .all_ticks()
        .flat_map_ordered(q!(|responses| responses));

    let responses_at_coord = responses_at_replica_dbs
        .send(coordinator, TCP.fail_stop().bincode())
        .values();

    // Coordinator deduplicates: only forward the first response per seq number.
    // Multiple replica_dbs respond to every command — we only want one.
    let coord_tick = coordinator.tick();
    let deduped = sliced! {
        let mut seen_seqs = use::state(|l| l.singleton(q!(std::collections::HashSet::<usize>::new())));
        let batch = use(responses_at_coord, nondet!(/** coord dedup */));

        let new_responses = batch.filter_map(q!(|resp: String| {
            resp.split("seq=").nth(1)
                .and_then(|s| s.split_whitespace().next())
                .and_then(|s| s.parse::<usize>().ok())
                .map(|seq| (seq, resp))
        }));

        let unique = new_responses.filter(q!(|(seq, _)| !seen_seqs.contains(seq)));
        seen_seqs = unique.clone().map(q!(|(seq, _)| seq))
            .fold(q!(|| seen_seqs.clone()), q!(|set, seq| { set.insert(seq); }));

        unique.map(q!(|(_, resp)| resp)).into_stream()
    }.all_ticks();

    let resp_stream = deduped.send_bincode_external(external);
    (cmd_sink, resp_stream)
}

#[tokio::test]
async fn put_get_verify_local() {
    use futures::SinkExt;
    use futures::StreamExt;
    use hydro_deploy::Deployment;
    use hydro_lang::deploy::{DeployCrateWrapper, TrybuildHost};

    let mut deployment = Deployment::new();
    let mut builder = FlowBuilder::new();

    let external = builder.external::<()>();
    let replicas = builder.cluster::<TransparentReplica>();
    let replica_dbs = builder.cluster::<ReplicaDb>();
    let coordinator = builder.process::<Coordinator>();

    #[cfg(stageleft_runtime)]
    let (cmd_sink, resp_stream) =
        build_pipeline(&external, &replicas, &replica_dbs, &coordinator);

    let features = vec!["backend_redb".to_string()];
    let nodes = builder
        .with_external(&external, deployment.Localhost())
        .with_cluster(
            &replicas,
            (0..3).map(|_| TrybuildHost::new(deployment.Localhost()).features(features.clone())),
        )
        .with_cluster(
            &replica_dbs,
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

    // Deterministic values.
    let mut rng: u64 = 0xdeadbeef;
    let mut expected: Vec<u64> = Vec::new();
    for _ in 0..100 {
        rng ^= rng << 13;
        rng ^= rng >> 7;
        rng ^= rng << 17;
        expected.push(rng % 1_000_000);
    }

    // Send 100 PUTs.
    #[cfg(stageleft_runtime)]
    for i in 0..100usize {
        sender
            .send(format!("PUT:k{}:{}", i, expected[i]))
            .await
            .unwrap();
    }

    // Wait for 100 unique PUT commits (deduplicate by seq — 3 replica_dbs each respond).
    #[cfg(stageleft_runtime)]
    {
        let mut seen = std::collections::HashSet::<usize>::new();
        let t = tokio::time::timeout(std::time::Duration::from_secs(60), async {
            while let Some(resp) = receiver.next().await {
                if let Some(s) = resp.split("seq=").nth(1) {
                    if let Ok(seq) = s.split_whitespace().next().unwrap_or("0").parse::<usize>() {
                        seen.insert(seq);
                        if seen.len() >= 100 {
                            break;
                        }
                    }
                }
            }
        })
        .await;
        assert!(t.is_ok(), "Timed out waiting for PUTs");
        println!("100 PUTs committed.");
    }

    // Send 5 GETs.
    let verify_keys = [0usize, 20, 40, 60, 80];
    #[cfg(stageleft_runtime)]
    for &k in &verify_keys {
        sender.send(format!("GET:k{}", k)).await.unwrap();
    }

    // Collect 5 unique GET responses (deduplicate by seq).
    #[cfg(stageleft_runtime)]
    {
        let mut responses: std::collections::HashMap<usize, String> =
            std::collections::HashMap::new();
        let t = tokio::time::timeout(std::time::Duration::from_secs(30), async {
            while let Some(resp) = receiver.next().await {
                if resp.contains("GET") {
                    if let Some(s) = resp.split("seq=").nth(1) {
                        if let Ok(seq) = s.split_whitespace().next().unwrap_or("0").parse::<usize>()
                        {
                            responses.entry(seq).or_insert(resp);
                            if responses.len() >= 5 {
                                break;
                            }
                        }
                    }
                }
            }
        })
        .await;
        assert!(t.is_ok(), "Timed out waiting for GETs");

        // Sort by seq and verify.
        let mut sorted: Vec<(usize, String)> = responses.into_iter().collect();
        sorted.sort_by_key(|(seq, _)| *seq);

        println!("GET responses:");
        for (i, (seq, resp)) in sorted.iter().enumerate() {
            println!("  seq={} resp={}", seq, resp);
            let key = verify_keys[i];
            let expected_val = expected[key];
            assert!(
                resp.contains(&format!("GET k{}={}", key, expected_val)),
                "Wrong value for k{}: expected {}, got: {}",
                key,
                expected_val,
                resp
            );
        }
        println!("PASS: all GET responses correct");
    }
}
