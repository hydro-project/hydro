use std::collections::HashMap;
use std::sync::Arc;

use hydro_deploy::Deployment;
use hydro_deploy::gcp::GcpNetwork;
use hydro_lang::Location;
use hydro_optimize::deploy::{ReusableHosts, deploy_and_analyze};
use tokio::sync::RwLock;

/// Run with no args for localhost, with `gcp <GCP PROJECT>` for GCP
///
/// ```bash
/// cargo run -p hydro_test --example perf_compute_pi -- gcp my-gcp-project
/// ```
///
/// Once the program is running, you can **press enter** to stop the program and see the results.
/// (Pressing Ctrl+C will stop the program **without cleaning up cloud resources** nor generating the
/// flamegraphs).
#[tokio::main]
async fn main() {
    let mut deployment = Deployment::new();
    let host_arg = std::env::args().nth(1).unwrap_or_default();
    let project = if host_arg == "gcp" {
        std::env::args().nth(2).unwrap()
    } else {
        String::new()
    };
    let network = Arc::new(RwLock::new(GcpNetwork::new(&project, None)));

    let mut reusable_hosts = ReusableHosts {
        hosts: HashMap::new(),
        host_arg,
        project: project.clone(),
        network: network.clone(),
    };

    let builder = hydro_lang::FlowBuilder::new();
    let (cluster, leader) = hydro_test::cluster::compute_pi::compute_pi(&builder, 8192);

    let clusters = vec![(cluster.id().raw_id(), cluster.typename(), 8)];
    let processes = vec![(leader.id().raw_id(), leader.typename())];

    let _ = deploy_and_analyze(
        &mut reusable_hosts,
        &mut deployment,
        builder,
        &clusters,
        &processes,
    )
    .await;
}
