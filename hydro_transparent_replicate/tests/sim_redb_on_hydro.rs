//! Integration test: redb running inside the Hydro dataflow.
//!
//! The applier process uses `RedbApplierState` from our crate — a concrete type
//! that wraps redb::Database and is accessible in the trybuild context via
//! `hydro_transparent_replicate::applier::RedbApplierState`.

#[cfg(stageleft_runtime)]
use hydro_lang::prelude::*;
#[cfg(stageleft_runtime)]
use hydro_transparent_replicate::messages::TransparentReplica;
#[cfg(stageleft_runtime)]
use hydro_transparent_replicate::protocol::core_replication_module;

struct RedbApplier;

#[cfg(stageleft_runtime)]
fn build_redb_pipeline<'a>(
    replicas: &Cluster<'a, TransparentReplica>,
    applier: &Process<'a, RedbApplier>,
) {
    let tick = replicas.tick();

    let commands: Stream<String, _, Unbounded> = replicas
        .source_iter(q!(vec![
            "PUT:x:1".to_string(),
            "PUT:y:7".to_string(),
            "PUT:z:42".to_string(),
            "GET:y".to_string(),
            "GET:x".to_string(),
            "GET:z".to_string(),
        ]))
        .into();

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

    let core = core_replication_module(
        replicas,
        &tick,
        commands,
        read_only,
        current_view,
        reconciled_seq,
    );

    let committed_at_applier = core.replicated
        .send(applier, TCP.fail_stop().bincode())
        .values();

    // Applier with REAL redb via RedbApplierState (a concrete type in our crate).
    let applier_tick = applier.tick();
    committed_at_applier
        .batch(&applier_tick, nondet!(/** applier batch */))
        .sort()
        .across_ticks(|s| s.fold(
            q!(|| hydro_transparent_replicate::applier::RedbApplierState::new()),
            q!(|state: &mut hydro_transparent_replicate::applier::RedbApplierState, (seq, cmd): (usize, String)| {
                let response = state.apply_command(seq, &cmd);
                println!("[REDB] {}", response);
            }),
        ))
        .all_ticks()
        .for_each(q!(|_| {}));
}

#[tokio::test]
async fn redb_applier_on_hydro() {
    use hydro_deploy::Deployment;
    use hydro_lang::deploy::{TrybuildHost, DeployCrateWrapper};

    let mut builder = FlowBuilder::new();
    let replicas = builder.cluster::<TransparentReplica>();
    let applier = builder.process::<RedbApplier>();

    #[cfg(stageleft_runtime)]
    build_redb_pipeline(&replicas, &applier);

    let mut deployment = Deployment::new();

    let nodes = builder
        .with_cluster(&replicas, (0..3).map(|_| TrybuildHost::new(deployment.Localhost())))
        .with_process(&applier, TrybuildHost::new(deployment.Localhost()))
        .deploy(&mut deployment);

    deployment.deploy().await.unwrap();

    let applier_node = nodes.get_process(&applier);
    let mut redb_out = applier_node.stdout_filter("[REDB]");

    deployment.start().await.unwrap();

    let mut responses: Vec<String> = Vec::new();
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        async {
            while let Some(line) = redb_out.recv().await {
                println!("  {}", line);
                responses.push(line);
                if responses.len() >= 6 { break; }
            }
        },
    ).await;

    assert!(result.is_ok(), "Timed out. Got {} responses: {:?}", responses.len(), responses);
    assert!(responses.len() >= 6, "Expected 6 responses, got {}: {:?}", responses.len(), responses);

    // Verify GETs return correct values from redb.
    let get_y = responses.iter().any(|r: &String| r.contains("GET y=7"));
    let get_x = responses.iter().any(|r: &String| r.contains("GET x=1"));
    let get_z = responses.iter().any(|r: &String| r.contains("GET z=42"));
    assert!(get_y, "GET y should return 7: {:?}", responses);
    assert!(get_x, "GET x should return 1: {:?}", responses);
    assert!(get_z, "GET z should return 42: {:?}", responses);

    println!("PASS: redb running inside Hydro dataflow - writes replicated, reads correct");
}
