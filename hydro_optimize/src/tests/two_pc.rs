use std::collections::{BTreeMap, HashMap};

use hydro_lang::compile::ir::deep_clone;
use hydro_lang::compile::rewrites::persist_pullup::persist_pullup;
use hydro_lang::deploy::HydroDeploy;
use hydro_lang::location::Location;
use hydro_lang::prelude::*;
use hydro_test::cluster::two_pc::{Coordinator, Participant};
use hydro_test::cluster::two_pc_bench::{Aggregator, Client};

use crate::debug::name_to_id_map;
use crate::partition_node_analysis::{nodes_to_partition, partitioning_analysis};
use crate::partitioner::{Partitioner, partition};
use crate::repair::{cycle_source_to_sink_input, inject_id, inject_location};

const NUM_PARTICIPANTS: usize = 3;

fn create_two_pc<'a>(
    coordinator: &Process<'a, Coordinator>,
    participants: &Cluster<'a, Participant>,
    clients: &Cluster<'a, Client>,
    client_aggregator: &Process<'a, Aggregator>,
) {
    hydro_test::cluster::two_pc_bench::two_pc_bench(
        100,
        coordinator,
        participants,
        NUM_PARTICIPANTS,
        clients,
        client_aggregator,
    );
}

#[test]
fn two_pc_partition_coordinator() {
    let builder = FlowBuilder::new();
    let coordinator = builder.process();
    let partitioned_coordinator = builder.cluster::<()>();
    let participants = builder.cluster();
    let clients = builder.cluster();
    let client_aggregator = builder.process();

    create_two_pc(&coordinator, &participants, &clients, &client_aggregator);

    let mut cycle_data = HashMap::new();
    let built = builder
        .optimize_with(persist_pullup)
        .optimize_with(inject_id)
        .optimize_with(|ir| {
            cycle_data = cycle_source_to_sink_input(ir);
            inject_location(ir, &cycle_data);
        })
        .into_deploy::<HydroDeploy>();
    let mut ir = deep_clone(built.ir());

    // Coordinator
    let coordinator_partitioning = partitioning_analysis(&mut ir, &coordinator.id(), &cycle_data);
    let name_to_id = name_to_id_map(&mut ir);
    let c_prepare_id = *name_to_id.get("c_prepare").unwrap();
    let c_votes_id = *name_to_id.get("c_votes").unwrap();
    let c_commits_id = *name_to_id.get("c_commits").unwrap();
    // 1 is the partitioning index of those inputs. Specifically, given the client sends (sender_id, payload) to the coordinator, we can partition on the entire payload
    let expected_coordinator_partitioning = vec![BTreeMap::from([
        (c_votes_id, vec!["1".to_string()]),
        (c_commits_id, vec!["1".to_string()]),
    ])];
    let expected_coordinator_input_parents = BTreeMap::from([
        (c_prepare_id, c_prepare_id - 1),
        (c_votes_id, c_votes_id - 1),
        (c_commits_id, c_commits_id - 1),
    ]);
    assert_eq!(
        coordinator_partitioning,
        Some((
            expected_coordinator_partitioning,
            expected_coordinator_input_parents
        ))
    );
    let coordinator_nodes_to_partition = nodes_to_partition(coordinator_partitioning).unwrap();
    let coordinator_partitioner = Partitioner {
        nodes_to_partition: coordinator_nodes_to_partition,
        num_partitions: 3,
        location_id: coordinator.id().raw_id(),
        new_cluster_id: Some(partitioned_coordinator.id().raw_id()),
    };
    partition(&mut ir, &coordinator_partitioner);

    hydro_build_utils::assert_debug_snapshot!(&ir);
}

#[test]
fn two_pc_partition_participant() {
    let builder = FlowBuilder::new();
    let coordinator = builder.process();
    let participants = builder.cluster();
    let clients = builder.cluster();
    let client_aggregator = builder.process();

    create_two_pc(&coordinator, &participants, &clients, &client_aggregator);

    let mut cycle_data = HashMap::new();
    let built = builder
        .optimize_with(persist_pullup)
        .optimize_with(inject_id)
        .optimize_with(|ir| {
            cycle_data = cycle_source_to_sink_input(ir);
            inject_location(ir, &cycle_data);
        })
        .into_deploy::<HydroDeploy>();
    let mut ir = deep_clone(built.ir());

    let participant_partitioning = partitioning_analysis(&mut ir, &participants.id(), &cycle_data);
    // Recalculate node IDs since they've changed as well
    let name_to_id = name_to_id_map(&mut ir);
    let p_prepare_id = *name_to_id.get("p_prepare").unwrap();
    let p_commits_id = *name_to_id.get("p_commits").unwrap();
    // Participants can partition on ANYTHING, since they only execute maps
    let expected_participant_partitionings = vec![];
    let expected_participant_input_parents = BTreeMap::from([
        (p_prepare_id, p_prepare_id - 1),
        (p_commits_id, p_commits_id - 1),
    ]);
    assert_eq!(
        participant_partitioning,
        Some((
            expected_participant_partitionings,
            expected_participant_input_parents
        ))
    );
    let participant_nodes_to_partition = nodes_to_partition(participant_partitioning).unwrap();
    let participant_partitioner = Partitioner {
        nodes_to_partition: participant_nodes_to_partition,
        num_partitions: 3,
        location_id: participants.id().raw_id(),
        new_cluster_id: None,
    };
    partition(&mut ir, &participant_partitioner);

    hydro_build_utils::assert_debug_snapshot!(&ir);
}
