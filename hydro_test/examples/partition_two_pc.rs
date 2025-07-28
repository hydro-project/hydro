use std::collections::HashMap;
use std::sync::Arc;

use hydro_deploy::gcp::GcpNetwork;
use hydro_deploy::{Deployment, Host};
use hydro_lang::Location;
use hydro_lang::deploy::TrybuildHost;
use hydro_lang::rewrites::persist_pullup::persist_pullup;
use hydro_optimize::partition_node_analysis::{nodes_to_partition, partitioning_analysis};
use hydro_optimize::partitioner::{Partitioner, partition};
use hydro_optimize::repair::{cycle_source_to_sink_input, inject_id, inject_location};
use tokio::sync::RwLock;

type HostCreator = Box<dyn Fn(&mut Deployment) -> Arc<dyn Host>>;

#[tokio::main]
async fn main() {
    let mut deployment = Deployment::new();
    let host_arg = std::env::args().nth(1).unwrap_or_default();

    let create_host: HostCreator = if host_arg == *"gcp" {
        let project = std::env::args().nth(2).unwrap();
        let network = Arc::new(RwLock::new(GcpNetwork::new(&project, None)));

        Box::new(move |deployment| -> Arc<dyn Host> {
            deployment
                .GcpComputeEngineHost()
                .project(&project)
                .machine_type("n2-standard-4")
                .image("debian-cloud/debian-11")
                .region("us-central1-c")
                .network(network.clone())
                .add()
        })
    } else {
        let localhost = deployment.Localhost();
        Box::new(move |_| -> Arc<dyn Host> { localhost.clone() })
    };

    let builder = hydro_lang::FlowBuilder::new();
    let num_participants = 3;
    let num_partitions = 3;
    let num_clients = 3;
    let num_clients_per_node = 100; // Change based on experiment between 1, 50, 100.

    let coordinator = builder.process();
    let partitioned_coordinator = builder.cluster::<()>();
    let participants = builder.cluster();
    let clients = builder.cluster();
    let client_aggregator = builder.process();

    hydro_test::cluster::two_pc_bench::two_pc_bench(
        num_clients_per_node,
        &coordinator,
        &participants,
        num_participants,
        &clients,
        &client_aggregator,
    );

    let mut cycle_data = HashMap::new();
    let deployable = builder
        .optimize_with(persist_pullup)
        .optimize_with(inject_id)
        .optimize_with(|ir| {
            cycle_data = cycle_source_to_sink_input(ir);
            inject_location(ir, &cycle_data);

            // Partition coordinator
            let coordinator_partitioning =
                partitioning_analysis(ir, &coordinator.id(), &cycle_data);
            let coordinator_nodes_to_partition =
                nodes_to_partition(coordinator_partitioning).unwrap();
            let coordinator_partitioner = Partitioner {
                nodes_to_partition: coordinator_nodes_to_partition,
                num_partitions,
                location_id: coordinator.id().raw_id(),
                new_cluster_id: Some(partitioned_coordinator.id().raw_id()),
            };
            partition(ir, &coordinator_partitioner);

            // Partition participants
            cycle_data = cycle_source_to_sink_input(ir); // Recompute since IDs have changed
            let participant_partitioning =
                partitioning_analysis(ir, &participants.id(), &cycle_data);
            let participant_nodes_to_partition =
                nodes_to_partition(participant_partitioning).unwrap();
            let participant_partitioner = Partitioner {
                nodes_to_partition: participant_nodes_to_partition,
                num_partitions,
                location_id: participants.id().raw_id(),
                new_cluster_id: None,
            };
            partition(ir, &participant_partitioner);
        })
        .into_deploy();

    let rustflags = "-C opt-level=3 -C codegen-units=1 -C strip=none -C debuginfo=2 -C lto=off";

    let _nodes = deployable
        .with_cluster(
            &partitioned_coordinator,
            (0..num_partitions)
                .map(|_| TrybuildHost::new(create_host(&mut deployment)).rustflags(rustflags)),
        )
        .with_cluster(
            &participants,
            (0..num_participants * num_partitions)
                .map(|_| TrybuildHost::new(create_host(&mut deployment)).rustflags(rustflags)),
        )
        .with_cluster(
            &clients,
            (0..num_clients)
                .map(|_| TrybuildHost::new(create_host(&mut deployment)).rustflags(rustflags)),
        )
        .with_process(
            &client_aggregator,
            TrybuildHost::new(create_host(&mut deployment)).rustflags(rustflags),
        )
        .deploy(&mut deployment);

    deployment.deploy().await.unwrap();

    deployment.start().await.unwrap();

    tokio::signal::ctrl_c().await.unwrap();
}
