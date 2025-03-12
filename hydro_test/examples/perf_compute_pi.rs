use std::collections::HashMap;
use std::sync::Arc;

use hydro_deploy::gcp::GcpNetwork;
use hydro_deploy::rust_crate::tracing_options::TracingOptions;
use hydro_deploy::{Deployment, Host};
use hydro_lang::deploy::{DeployCrateWrapper, TrybuildHost};
use hydro_lang::ir::deep_clone;
use hydro_lang::q;
use hydro_lang::rewrites::analyze_counter::COUNTER_PREFIX;
use hydro_lang::rewrites::analyze_perf::CPU_USAGE_PREFIX;
use hydro_lang::rewrites::analyze_perf_and_counters::analyze_results;
use hydro_lang::rewrites::{insert_counter, persist_pullup};
use tokio::sync::RwLock;

type HostCreator = Box<dyn Fn(&mut Deployment) -> Arc<dyn Host>>;

// run with no args for localhost, with `gcp <GCP PROJECT>` for GCP
#[tokio::main]
async fn main() {
    let mut deployment = Deployment::new();
    let host_arg = std::env::args().nth(1).unwrap_or_default();

    let project = if host_arg == "gcp" {
        std::env::args().nth(2)
    } else {
        None
    };

    let network = project
        .as_ref()
        .map(|project| Arc::new(RwLock::new(GcpNetwork::new(project, None))));

    let create_host: HostCreator = if host_arg == "gcp" {
        Box::new(move |deployment| -> Arc<dyn Host> {
            let startup_script = "sudo sh -c 'apt update && apt install -y linux-perf binutils && echo -1 > /proc/sys/kernel/perf_event_paranoid && echo 0 > /proc/sys/kernel/kptr_restrict'";
            deployment
                .GcpComputeEngineHost()
                .project(project.as_ref().unwrap())
                .machine_type("e2-micro")
                .image("debian-cloud/debian-11")
                .region("us-west1-a")
                .network(network.as_ref().unwrap().clone())
                .startup_script(startup_script)
                .add()
        })
    } else {
        let localhost = deployment.Localhost();
        Box::new(move |_| -> Arc<dyn Host> { localhost.clone() })
    };

    let rustflags = if host_arg == "gcp" {
        "-C opt-level=3 -C codegen-units=1 -C strip=none -C debuginfo=2 -C lto=off -C link-args=--no-rosegment"
    } else {
        "-C opt-level=3 -C codegen-units=1 -C strip=none -C debuginfo=2 -C lto=off"
    };

    let builder = hydro_lang::FlowBuilder::new();
    let (cluster, leader) = hydro_test::cluster::compute_pi::compute_pi(&builder, 8192);

    let frequency = 128;
    let counter_output_duration = q!(std::time::Duration::from_secs(1));

    let optimized = builder
        .optimize_with(persist_pullup::persist_pullup)
        .optimize_with(|ir| insert_counter::insert_counter(ir, counter_output_duration));
    let mut ir = deep_clone(optimized.ir());
    let nodes = optimized
        .with_process(
            &leader,
            TrybuildHost::new(create_host(&mut deployment))
                .rustflags(rustflags)
                .additional_hydro_features(vec!["runtime_measure".to_string()])
                .tracing(
                    TracingOptions::builder()
                        .perf_raw_outfile("leader.perf.data")
                        .dtrace_outfile("leader.stacks")
                        .fold_outfile("leader.data.folded")
                        .flamegraph_outfile("leader.svg")
                        .frequency(frequency)
                        .build(),
                ),
        )
        .with_cluster(
            &cluster,
            (0..8).map(|idx| {
                TrybuildHost::new(create_host(&mut deployment))
                    .rustflags(rustflags)
                    .additional_hydro_features(vec!["runtime_measure".to_string()])
                    .tracing(
                        TracingOptions::builder()
                            .perf_raw_outfile(format!("cluster{}.perf.data", idx))
                            .dtrace_outfile(format!("cluster{}.leader.stacks", idx))
                            .fold_outfile(format!("cluster{}.data.folded", idx))
                            .flamegraph_outfile(format!("cluster{}.svg", idx))
                            .frequency(frequency)
                            .build(),
                    )
            }),
        )
        .deploy(&mut deployment);

    deployment.deploy().await.unwrap();

    let mut leader_usage_out = nodes
        .get_process(&leader)
        .stdout_filter(CPU_USAGE_PREFIX)
        .await;
    let mut usage_out = HashMap::new();
    let mut cardinality_out = HashMap::new();
    for (id, name, cluster) in nodes.get_all_clusters() {
        for (idx, node) in cluster.members().iter().enumerate() {
            let out = node.stdout_filter(CPU_USAGE_PREFIX).await;
            usage_out.insert((id.clone(), name.clone(), idx), out);

            let out = node.stdout_filter(COUNTER_PREFIX).await;
            cardinality_out.insert((id.clone(), name.clone(), idx), out);
        }
    }

    deployment
        .start_until(async {
            std::io::stdin().read_line(&mut String::new()).unwrap();
        })
        .await
        .unwrap();

    println!("Leader {}", leader_usage_out.recv().await.unwrap());
    analyze_results(nodes, &mut ir, &mut usage_out, &mut cardinality_out).await;

    hydro_lang::ir::dbg_dedup_tee(|| {
        println!("{:#?}", ir);
    });
}
