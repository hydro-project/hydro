//! Minimal reproduction: DemuxMap crashes surviving nodes when a peer dies.
//!
//! Deploys a 3-node cluster where each node broadcasts a heartbeat counter
//! to all peers. After warmup, node 2 is killed. With upstream Hydro's DemuxMap,
//! nodes 0 and 1 panic with BrokenPipe. With the patched DemuxMap, they survive.
//!
//! Run: cargo test --release demux_crash_repro -- --nocapture

#[cfg(stageleft_runtime)]
use hydro_lang::location::cluster::CLUSTER_SELF_ID;
#[cfg(stageleft_runtime)]
use hydro_lang::prelude::*;

use lego_replicate::messages::TransparentReplica;

#[cfg(stageleft_runtime)]
fn build_heartbeat_cluster<'a>(
    nodes: &Cluster<'a, TransparentReplica>,
) {
    // Each node sends a counter every 200ms
    let heartbeats = nodes
        .source_interval(q!(std::time::Duration::from_millis(200)))
        .scan(
            q!(|| 0u64),
            q!(|counter: &mut u64, _| { *counter += 1; Some(*counter) }),
        );

    // Broadcast to all peers (uses DemuxMap internally)
    let received = heartbeats
        .broadcast(nodes, TCP.fail_stop().bincode(), nondet!(/** heartbeat broadcast */))
        .values();

    // Print what we receive — this lets us verify nodes are alive via stdout
    received
        .assume_ordering::<hydro_lang::live_collections::stream::TotalOrder>(nondet!(/** order doesn't matter for printing */))
        .for_each(q!(move |count: u64| {
            println!("[HEARTBEAT] node={} received count={}", CLUSTER_SELF_ID.get_raw_id(), count);
        }));
}

#[tokio::test]
async fn demux_crash_repro() {
    use hydro_deploy::{Deployment, Service};
    use hydro_lang::deploy::{DeployCrateWrapper, TrybuildHost};

    let mut deployment = Deployment::new();
    let mut builder = hydro_lang::compile::builder::FlowBuilder::new();

    let nodes = builder.cluster::<TransparentReplica>();

    #[cfg(stageleft_runtime)]
    build_heartbeat_cluster(&nodes);

    let node_list = builder
        .with_cluster(&nodes, (0..3).map(|_| TrybuildHost::new(deployment.Localhost())))
        .deploy(&mut deployment);

    deployment.deploy().await.unwrap();

    // Get stdout handles for nodes 0 and 1
    let members = node_list.get_cluster(&nodes).members();
    let mut stdout_0 = members[0].stdout_filter("[HEARTBEAT]");
    let mut stdout_1 = members[1].stdout_filter("[HEARTBEAT]");

    deployment.start().await.unwrap();

    // Phase 1: Wait for heartbeats from nodes 0 and 1 (proving they're alive)
    println!("Phase 1: Verifying nodes are alive...");
    let mut count_0 = 0u32;
    let mut count_1 = 0u32;
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
    while tokio::time::Instant::now() < deadline && (count_0 < 3 || count_1 < 3) {
        tokio::select! {
            Some(_) = stdout_0.recv() => { count_0 += 1; }
            Some(_) = stdout_1.recv() => { count_1 += 1; }
            _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => {}
        }
    }
    println!("  Node 0: {} heartbeats, Node 1: {} heartbeats", count_0, count_1);
    assert!(count_0 >= 2 && count_1 >= 2, "Nodes should produce heartbeats during warmup");

    // Phase 2: Kill node 2
    println!("\nPhase 2: Killing node 2...");
    members[2].underlying().stop().await.unwrap();
    println!("  Node 2 stopped.");

    // Phase 3: Verify nodes 0 and 1 keep producing heartbeats
    println!("\nPhase 3: Verifying survivors keep operating...");
    let mut post_kill_0 = 0u32;
    let mut post_kill_1 = 0u32;
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
    while tokio::time::Instant::now() < deadline {
        tokio::select! {
            Some(_) = stdout_0.recv() => { post_kill_0 += 1; }
            Some(_) = stdout_1.recv() => { post_kill_1 += 1; }
            _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => {}
        }
        if post_kill_0 >= 3 && post_kill_1 >= 3 { break; }
    }

    println!("  Post-kill: Node 0 = {} heartbeats, Node 1 = {} heartbeats", post_kill_0, post_kill_1);

    if post_kill_0 > 0 && post_kill_1 > 0 {
        println!("\nPASS: Surviving nodes continued after peer death.");
        println!("      (DemuxMap patch is working correctly)");
    } else {
        println!("\nFAIL: Nodes stopped producing output after peer death.");
        println!("      Surviving nodes likely panicked with BrokenPipe.");
        println!("      This confirms the upstream DemuxMap bug.");
    }

    assert!(post_kill_0 > 0 && post_kill_1 > 0,
        "Surviving nodes should continue after peer death");
}
