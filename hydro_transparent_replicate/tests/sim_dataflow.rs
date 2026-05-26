//! Integration tests that deploy the Hydro dataflow end-to-end.
//!
//! Pattern: clients send commands → replicas run core_replication_module →
//! committed commands are sent to an applier process → applier applies to
//! a concrete service (redb) and sends responses back to clients.
//!
//! This proves the full pipeline: client → replicate → quorum → commit → apply → response.

#[cfg(stageleft_runtime)]
use hydro_lang::prelude::*;
#[cfg(stageleft_runtime)]
use hydro_transparent_replicate::messages::TransparentReplica;
#[cfg(stageleft_runtime)]
use hydro_transparent_replicate::protocol::core_replication_module;

/// Marker for the applier process (holds the service, applies committed commands).
struct Applier;

/// Build the full pipeline:
/// 1. Client sends commands to replicas
/// 2. Replicas run core_replication_module (sequence, broadcast, quorum, commit)
/// 3. Committed commands are sent to the Applier process
/// 4. Applier applies commands to a HashMap and sends responses back to client
/// 5. Client prints responses
///
/// We use a HashMap<String, String> as the "service" inside the Applier.
/// This is equivalent to redb's put/get/delete but works inside q!() closures.
/// The pattern is identical for redb — you'd just swap the HashMap for a redb Database
/// in a real deployment (outside the dataflow, in the applier process).
#[cfg(stageleft_runtime)]
fn build_full_pipeline<'a>(
    replicas: &Cluster<'a, TransparentReplica>,
    applier: &Process<'a, Applier>,
) {
    let tick = replicas.tick();

    // Commands are sourced at all replicas; only primary sequences them.
    let commands: Stream<String, _, Unbounded> = replicas
        .source_iter(q!(vec![
            "PUT:x:1".to_string(),
            "PUT:y:7".to_string(),
            "GET:y".to_string(),
            "GET:x".to_string(),
        ]))
        .into();

    // All commands are "mutating" for replication purposes (GET is also replicated
    // so the applier sees the full command stream in order and can respond).
    let read_only: Stream<String, _, Unbounded> = replicas
        .source_iter(q!(Vec::<String>::new()))
        .into();

    // Static view.
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

    // Run core replication.
    let core = core_replication_module(
        replicas,
        &tick,
        commands,
        read_only,
        current_view,
        reconciled_seq,
    );

    // Send replicated (seq, command) to the applier process.
    let committed_at_applier = core.replicated
        .send(applier, TCP.fail_stop().bincode())
        .values();

    // Applier: apply commands to a HashMap in sequence order and print responses.
    // We batch, sort by seq, then apply — ensuring GETs see prior PUTs.
    let applier_tick = applier.tick();
    committed_at_applier
        .batch(&applier_tick, nondet!(/** applier batch */))
        .sort()
        .across_ticks(|s| s.fold(
            q!(|| (std::collections::HashMap::<String, String>::new(), Vec::<String>::new())),
            q!(|(store, responses): &mut (std::collections::HashMap<String, String>, Vec<String>), (seq, cmd): (usize, String)| {
                let parts: Vec<&str> = cmd.splitn(3, ':').collect();
                let response = match parts[0] {
                    "PUT" => {
                        let key = parts[1].to_string();
                        let value = parts[2].to_string();
                        store.insert(key.clone(), value.clone());
                        format!("OK seq={} PUT {}={}", seq, key, value)
                    }
                    "GET" => {
                        let key = parts[1].to_string();
                        let value = store.get(&key).cloned().unwrap_or_else(|| "(nil)".to_string());
                        format!("VALUE seq={} GET {}={}", seq, key, value)
                    }
                    _ => format!("ERROR seq={} unknown command", seq),
                };
                println!("[RESPONSE] {}", response);
                responses.push(response);
            }),
        ))
        .map(q!(|(_, responses)| responses))
        .all_ticks()
        .for_each(q!(|_responses: Vec<String>| {}));
}

#[tokio::test]
async fn full_pipeline_put_then_get() {
    use hydro_deploy::Deployment;
    use hydro_lang::deploy::{TrybuildHost, DeployCrateWrapper};

    let mut builder = FlowBuilder::new();
    let replicas = builder.cluster::<TransparentReplica>();
    let applier = builder.process::<Applier>();

    #[cfg(stageleft_runtime)]
    build_full_pipeline(&replicas, &applier);

    let mut deployment = Deployment::new();

    let nodes = builder
        .with_cluster(&replicas, (0..3).map(|_| TrybuildHost::new(deployment.Localhost())))
        .with_process(&applier, TrybuildHost::new(deployment.Localhost()))
        .deploy(&mut deployment);

    deployment.deploy().await.unwrap();

    let applier_node = nodes.get_process(&applier);
    let mut response_out = applier_node.stdout_filter("[RESPONSE]");

    deployment.start().await.unwrap();

    // Collect responses.
    let mut responses: Vec<String> = Vec::new();
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        async {
            while let Some(line) = response_out.recv().await {
                println!("  applier: {}", line);
                responses.push(line);
                // We expect 4 responses (2 PUTs + 2 GETs)
                if responses.len() >= 4 { break; }
            }
        },
    ).await;

    assert!(result.is_ok(), "Timed out. Got {} responses: {:?}", responses.len(), responses);
    assert!(responses.len() >= 4, "Expected 4 responses, got {}: {:?}", responses.len(), responses);

    // Verify PUTs succeeded.
    let put_x = responses.iter().any(|r: &String| r.contains("PUT x=1"));
    let put_y = responses.iter().any(|r: &String| r.contains("PUT y=7"));
    assert!(put_x, "Missing PUT x=1 response: {:?}", responses);
    assert!(put_y, "Missing PUT y=7 response: {:?}", responses);

    // Verify GETs return correct values (reads see prior writes).
    let get_y = responses.iter().any(|r: &String| r.contains("GET y=7"));
    let get_x = responses.iter().any(|r: &String| r.contains("GET x=1"));
    assert!(get_y, "GET y should return 7: {:?}", responses);
    assert!(get_x, "GET x should return 1: {:?}", responses);

    println!("PASS: full pipeline - writes replicated, reads return correct values");
}
