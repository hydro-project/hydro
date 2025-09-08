#[tokio::main]
async fn main() {
    use std::collections::HashMap;
    use std::sync::Arc;

    use clap::Parser;
    use hydro_deploy::Deployment;
    use hydro_deploy::gcp::GcpNetwork;
    use hydro_lang::graph::config::GraphConfig;
    use hydro_lang::location::Location;
    use hydro_optimize::decoupler;
    use hydro_optimize::deploy::ReusableHosts;
    use hydro_optimize::deploy_and_analyze::deploy_and_analyze;
    use hydro_test::cluster::kv_replica::Replica;
    use hydro_test::cluster::paxos::{Acceptor, CorePaxos, PaxosConfig, Proposer};
    use hydro_test::cluster::paxos_bench::{Aggregator, Client};
    use tokio::sync::RwLock;

    #[derive(Parser, Debug)]
    #[command(author, version, about, long_about = None)]
    struct PerfPaxosArgs {
        #[command(flatten)]
        graph: GraphConfig,

        /// Use GCP for deployment (provide project name)
        #[arg(long)]
        gcp: Option<String>,
    }

    let args = PerfPaxosArgs::parse();

    let mut deployment = Deployment::new();
    let (host_arg, project) = if let Some(project) = args.gcp {
        ("gcp".to_string(), project)
    } else {
        ("localhost".to_string(), String::new())
    };
    let network = Arc::new(RwLock::new(GcpNetwork::new(&project, None)));

    let mut builder = hydro_lang::builder::FlowBuilder::new();
    let f = 1;
    let num_clients = 3;
    let num_clients_per_node = 500; // Change based on experiment between 1, 50, 100.
    let checkpoint_frequency = 1000; // Num log entries
    let i_am_leader_send_timeout = 5; // Sec
    let i_am_leader_check_timeout = 10; // Sec
    let i_am_leader_check_timeout_delay_multiplier = 15;

    let proposers = builder.cluster();
    let acceptors = builder.cluster();
    let clients = builder.cluster();
    let client_aggregator = builder.process();
    let replicas = builder.cluster();

    hydro_test::cluster::paxos_bench::paxos_bench(
        num_clients_per_node,
        checkpoint_frequency,
        f,
        f + 1,
        CorePaxos {
            proposers: proposers.clone(),
            acceptors: acceptors.clone(),
            paxos_config: PaxosConfig {
                f,
                i_am_leader_send_timeout,
                i_am_leader_check_timeout,
                i_am_leader_check_timeout_delay_multiplier,
            },
        },
        &clients,
        &client_aggregator,
        &replicas,
    );

    let mut clusters = vec![
        (
            proposers.id().raw_id(),
            std::any::type_name::<Proposer>().to_string(),
            f + 1,
        ),
        (
            acceptors.id().raw_id(),
            std::any::type_name::<Acceptor>().to_string(),
            2 * f + 1,
        ),
        (
            clients.id().raw_id(),
            std::any::type_name::<Client>().to_string(),
            num_clients,
        ),
        (
            replicas.id().raw_id(),
            std::any::type_name::<Replica>().to_string(),
            f + 1,
        ),
    ];
    let processes = vec![(
        client_aggregator.id().raw_id(),
        std::any::type_name::<Aggregator>().to_string(),
    )];

    // Deploy
    let mut reusable_hosts = ReusableHosts {
        hosts: HashMap::new(),
        host_arg,
        project: project.clone(),
        network: network.clone(),
    };

    let num_times_to_optimize = 2;

    for i in 0..num_times_to_optimize {
        let (rewritten_ir_builder, mut ir, mut decoupler, bottleneck_name, bottleneck_num_nodes) =
            deploy_and_analyze(
                &mut reusable_hosts,
                &mut deployment,
                builder,
                &clusters,
                &processes,
                vec![
                    std::any::type_name::<Client>().to_string(),
                    std::any::type_name::<Aggregator>().to_string(),
                ],
                None,
            )
            .await;

        // Apply decoupling
        let mut decoupled_cluster = None;
        builder = rewritten_ir_builder.build_with(|builder| {
            let new_cluster = builder.cluster::<()>();
            decoupler.decoupled_location = new_cluster.id().clone();
            decoupler::decouple(&mut ir, &decoupler);
            decoupled_cluster = Some(new_cluster);

            ir
        });
        if let Some(new_cluster) = decoupled_cluster {
            clusters.push((
                new_cluster.id().raw_id(),
                format!("{}-decouple-{}", bottleneck_name, i),
                bottleneck_num_nodes,
            ));
        }
    }

    let built = builder.finalize();

    // Generate graphs if requested
    _ = built.generate_graph_with_config(&args.graph, None);
}
