#[cfg(not(feature = "ilp"))]
#[tokio::main]
async fn main() {
    panic!("Run with the `ilp` feature enabled.");
}

#[cfg(feature = "ilp")]
#[tokio::main]
async fn main() {
    use std::collections::HashMap;
    use std::sync::Arc;

    use hydro_deploy::Deployment;
    use hydro_deploy::gcp::GcpNetwork;
    use hydro_lang::Location;
    use hydro_lang::ir::deep_clone;
    use hydro_optimize::decoupler;
    use hydro_optimize::deploy::ReusableHosts;
    use hydro_optimize::deploy_and_analyze::deploy_and_analyze;
    use hydro_test::cluster::paxos::{CorePaxos, PaxosConfig};
    use tokio::sync::RwLock;

    let mut deployment = Deployment::new();
    let host_arg = std::env::args().nth(1).unwrap_or_default();
    let project = if host_arg == "gcp" {
        std::env::args().nth(2).unwrap()
    } else {
        String::new()
    };
    let network = Arc::new(RwLock::new(GcpNetwork::new(&project, None)));

    let mut builder = hydro_lang::FlowBuilder::new();
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
        (proposers.id().raw_id(), proposers.typename(), f + 1),
        (acceptors.id().raw_id(), acceptors.typename(), 2 * f + 1),
        (clients.id().raw_id(), clients.typename(), num_clients),
        (replicas.id().raw_id(), replicas.typename(), f + 1),
    ];
    let processes = vec![(
        client_aggregator.id().raw_id(),
        client_aggregator.typename(),
    )];

    // Deploy
    let mut reusable_hosts = ReusableHosts {
        hosts: HashMap::new(),
        host_arg,
        project: project.clone(),
        network: network.clone(),
    };

    let num_times_to_optimize = 2;

    for _ in 0..num_times_to_optimize {
        let (rewritten_ir_builder, ir, mut decoupler, bottleneck_num_nodes) = deploy_and_analyze(
            &mut reusable_hosts,
            &mut deployment,
            builder,
            &clusters,
            &processes,
        )
        .await;

        // Apply decoupling
        let mut decoupled_cluster = None;
        builder = rewritten_ir_builder.build_with(|builder| {
            let mut ir = deep_clone(&ir); // TODO: Not sure if this line is necessary anymore?

            let new_cluster = builder.cluster::<()>();
            decoupler.decoupled_location = new_cluster.id().clone();
            decoupler::decouple(&mut ir, &decoupler);
            decoupled_cluster = Some(new_cluster);

            ir
        });
        if let Some(new_cluster) = decoupled_cluster {
            clusters.push((
                new_cluster.id().raw_id(),
                new_cluster.typename(), // TODO: Need unique typename to prevent name collisions after multiple rewrites
                bottleneck_num_nodes,
            ));
        }
    }
}
