use std::sync::Arc;

use clap::Parser;
use hydro_deploy::gcp::GcpNetwork;
use hydro_deploy::{Deployment, Host};
use hydro_lang::deploy::TrybuildHost;
use hydro_lang::graph_util::GraphConfig;
use tokio::sync::RwLock;

type HostCreator = Box<dyn Fn(&mut Deployment) -> Arc<dyn Host>>;

#[derive(Parser, Debug)]
struct Args {
    /// Use GCP instead of localhost (requires project name)
    #[clap(long)]
    gcp: Option<String>,

    #[clap(flatten)]
    graph: GraphConfig,
}

// run with no args for localhost, with `--gcp <GCP PROJECT>` for GCP
#[tokio::main]
async fn main() {
    let args = Args::parse();
    let mut deployment = Deployment::new();

    let (create_host, rustflags): (HostCreator, &'static str) = if let Some(project) = args.gcp {
        let network = Arc::new(RwLock::new(GcpNetwork::new(&project, None)));

        (
            Box::new(move |deployment| -> Arc<dyn Host> {
                deployment
                    .GcpComputeEngineHost()
                    .project(&project)
                    .machine_type("e2-micro")
                    .image("debian-cloud/debian-11")
                    .region("us-west1-a")
                    .network(network.clone())
                    .add()
            }),
            "-C opt-level=3 -C codegen-units=1 -C strip=none -C debuginfo=2 -C lto=off",
        )
    } else {
        let localhost = deployment.Localhost();
        (
            Box::new(move |_| -> Arc<dyn Host> { localhost.clone() }),
            "",
        )
    };

    let builder = hydro_lang::FlowBuilder::new();
    let (cluster, leader) = hydro_test::cluster::compute_pi::compute_pi(&builder, 8192);

    // Extract the IR for graph visualization
    let built = builder.finalize();

    // Generate graph visualizations based on command line arguments
    if let Err(e) = built.generate_graph_with_config(&args.graph, None) {
        eprintln!("Error generating graph: {}", e);
    }

    let _nodes = built
        .with_default_optimize()
        .with_process(
            &leader,
            TrybuildHost::new(create_host(&mut deployment)).rustflags(rustflags),
        )
        .with_cluster(
            &cluster,
            (0..8).map(|_| TrybuildHost::new(create_host(&mut deployment)).rustflags(rustflags)),
        )
        .deploy(&mut deployment);

    deployment.run_ctrl_c().await.unwrap();
}

// Graph visualization functionality
#[cfg(feature = "debugging")]
mod graph_viz {
    use hydro_lang::graph::debug::{open_mermaid, save_dot, save_mermaid};
    use hydro_lang::graph::render::{
        HydroWriteConfig, render_hydro_ir_dot, render_hydro_ir_mermaid,
    };
    use hydro_lang::ir::HydroLeaf;

    pub fn visualize_hydro_ir() {
        println!("=== 🎨 Hydro IR Graph Visualization ===");

        // Get the finalized Hydro IR by finalizing a temporary builder
        let built_flow = {
            let temp_builder = hydro_lang::FlowBuilder::new();
            let (_temp_cluster, _temp_leader) =
                hydro_test::cluster::compute_pi::compute_pi(&temp_builder, 8192);
            temp_builder.finalize()
        };

        let hydro_ir_leaves = built_flow.ir();

        // Generate visualizations
        let config = HydroWriteConfig {
            show_metadata: true,
            show_location_groups: true,
            include_tee_ids: true,
            use_short_labels: false,
            process_id_name: built_flow.process_id_name().clone(),
            cluster_id_name: built_flow.cluster_id_name().clone(),
            external_id_name: built_flow.external_id_name().clone(),
        };

        generate_and_save_graphs(hydro_ir_leaves, &config);
        open_in_browser(hydro_ir_leaves, &config);
        print_statistics(hydro_ir_leaves, &config);
    }

    fn generate_and_save_graphs(leaves: &Vec<HydroLeaf>, config: &HydroWriteConfig) {
        let mermaid_output = render_hydro_ir_mermaid(leaves, config);
        let dot_output = render_hydro_ir_dot(leaves, config);

        std::fs::write("compute_pi_graph.mermaid", &mermaid_output).unwrap();
        std::fs::write("compute_pi_graph.dot", &dot_output).unwrap();

        println!("📄 Saved graphs:");
        println!("  - compute_pi_graph.mermaid (interactive Mermaid diagram)");
        println!("  - compute_pi_graph.dot (Graphviz DOT format)");
    }

    fn open_in_browser(leaves: &Vec<HydroLeaf>, config: &HydroWriteConfig) {
        println!("\n🚀 Opening in browser...");

        match save_mermaid(
            leaves,
            Some("compute_pi_interactive.mermaid"),
            Some(config.clone()),
        ) {
            Ok(_) => println!("✅ Saved Mermaid graph to file"),
            Err(e) => println!("⚠️  Could not save Mermaid graph: {}", e),
        }

        match open_mermaid(leaves, Some(config.clone())) {
            Ok(_) => println!("✅ Opened graph in browser"),
            Err(e) => println!("⚠️  Could not open in browser: {}", e),
        }
    }

    fn print_statistics(leaves: &Vec<HydroLeaf>, config: &HydroWriteConfig) {
        let mermaid_output = render_hydro_ir_mermaid(leaves, config);
        let dot_output = render_hydro_ir_dot(leaves, config);

        println!("\n📊 Graph Statistics:");
        println!("  - Total IR leaves: {}", leaves.len());
        println!(
            "  - Mermaid output: {} lines",
            mermaid_output.lines().count()
        );
        println!("  - DOT output: {} lines", dot_output.lines().count());

        println!("\n🌐 View online:");
        println!("  - Mermaid: Copy content to https://mermaid.live");
        println!("  - DOT: Copy content to https://dreampuf.github.io/GraphvizOnline/");
    }
}

#[test]
fn test() {
    use example_test::run_current_example;

    let mut run = run_current_example!();
    run.read_regex(r"\[hydro_test::cluster::compute_pi::Leader \(process 1\)\] pi: 3\.141\d+ \(\d{8,} trials\)");
}
