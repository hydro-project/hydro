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
    use std::collections::HashMap;
    use std::sync::Arc;

    use clap::Parser;
    use hydro_deploy::Deployment;
    use hydro_deploy::gcp::GcpNetwork;
    use hydro_lang::graph::config::GraphConfig;
    use hydro_lang::location::Location;
    use hydro_optimize::deploy::ReusableHosts;
    use hydro_optimize::deploy_and_analyze::deploy_and_analyze;
    use hydro_test::cluster::compute_pi::{Leader, Worker, compute_pi};
    use tokio::sync::RwLock;

    #[derive(Parser, Debug)]
    #[command(author, version, about, long_about = None)]
    struct PerfArgs {
        #[command(flatten)]
        graph: GraphConfig,

        /// Use GCP for deployment (provide project name)
        #[arg(long)]
        gcp: Option<String>,
    }

    let args = PerfArgs::parse();

    let mut deployment = Deployment::new();
    let (host_arg, project) = if let Some(project) = args.gcp {
        ("gcp".to_string(), project)
    } else {
        ("localhost".to_string(), String::new())
    };
    let network = Arc::new(RwLock::new(GcpNetwork::new(&project, None)));

    let mut reusable_hosts = ReusableHosts {
        hosts: HashMap::new(),
        host_arg,
        project: project.clone(),
        network: network.clone(),
    };

    let builder = hydro_lang::compile::builder::FlowBuilder::new();
    let (cluster, leader) = compute_pi(&builder, 8192);

    let clusters = vec![(
        cluster.id().raw_id(),
        std::any::type_name::<Worker>().to_string(),
        8,
    )];
    let processes = vec![(
        leader.id().raw_id(),
        std::any::type_name::<Leader>().to_string(),
    )];

    let (rewritten_ir_builder, ir, _, _, _) = deploy_and_analyze(
        &mut reusable_hosts,
        &mut deployment,
        builder,
        &clusters,
        &processes,
        vec![],
        None,
    )
    .await;

    // Cleanup and generate graphs if requested
    let built = rewritten_ir_builder.build_with(|_| ir).finalize();
    _ = built.generate_graph_with_config(&args.graph, None);
}
