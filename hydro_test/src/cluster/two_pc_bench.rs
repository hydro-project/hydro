use hydro_lang::*;
use hydro_std::bench_client::{bench_client, print_bench_results};

use super::two_pc::{Coordinator, Participant};
use crate::cluster::paxos_bench::inc_u32_workload_generator;
use crate::cluster::two_pc::two_pc;

pub struct Client;
pub struct Aggregator;

pub fn two_pc_bench<'a>(
    num_clients_per_node: usize,
    coordinator: &Process<'a, Coordinator>,
    participants: &Cluster<'a, Participant>,
    num_participants: usize,
    clients: &Cluster<'a, Client>,
    client_aggregator: &Process<'a, Aggregator>,
) {
    let bench_results = bench_client(
        clients,
        inc_u32_workload_generator,
        |payloads| {
            // Send committed requests back to the original client
            two_pc(
                coordinator,
                participants,
                num_participants,
                payloads.send_bincode(coordinator).entries(),
            )
            .demux_bincode(clients)
        },
        num_clients_per_node,
        nondet!(/** bench */),
    );

    print_bench_results(bench_results, client_aggregator, clients);
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, HashMap};

    use dfir_lang::graph::WriteConfig;
    use hydro_deploy::Deployment;
    use hydro_lang::Location;
    use hydro_lang::deploy::{DeployCrateWrapper, HydroDeploy, TrybuildHost};
    use hydro_lang::ir::deep_clone;
    use hydro_lang::rewrites::persist_pullup::persist_pullup;
    #[cfg(stageleft_runtime)]
    use hydro_lang::{Cluster, Process};
    use hydro_optimize::debug::name_to_id_map;
    use hydro_optimize::partition_node_analysis::{nodes_to_partition, partitioning_analysis};
    use hydro_optimize::partitioner::{Partitioner, partition};
    use hydro_optimize::repair::{cycle_source_to_sink_input, inject_id, inject_location};

    #[cfg(stageleft_runtime)]
    use crate::cluster::{
        two_pc::{Coordinator, Participant},
        two_pc_bench::{Aggregator, Client},
    };

    const NUM_PARTICIPANTS: usize = 3;

    #[cfg(stageleft_runtime)]
    fn create_two_pc<'a>(
        coordinator: &Process<'a, Coordinator>,
        participants: &Cluster<'a, Participant>,
        clients: &Cluster<'a, Client>,
        client_aggregator: &Process<'a, Aggregator>,
    ) {
        super::two_pc_bench(
            100,
            coordinator,
            participants,
            NUM_PARTICIPANTS,
            clients,
            client_aggregator,
        );
    }

    #[test]
    fn two_pc_ir() {
        let builder = hydro_lang::FlowBuilder::new();
        let coordinator = builder.process();
        let participants = builder.cluster();
        let clients = builder.cluster();
        let client_aggregator = builder.process();

        create_two_pc(&coordinator, &participants, &clients, &client_aggregator);
        let built = builder.with_default_optimize::<HydroDeploy>();

        hydro_lang::ir::dbg_dedup_tee(|| {
            insta::assert_debug_snapshot!(built.ir());
        });

        let preview = built.preview_compile();
        insta::with_settings!({snapshot_suffix => "coordinator_mermaid"}, {
            insta::assert_snapshot!(
                preview.dfir_for(&coordinator).to_mermaid(&WriteConfig {
                    no_subgraphs: true,
                    no_pull_push: true,
                    no_handoffs: true,
                    op_text_no_imports: true,
                    ..WriteConfig::default()
                })
            );
        });

        let preview = built.preview_compile();
        insta::with_settings!({snapshot_suffix => "participants_mermaid"}, {
            insta::assert_snapshot!(
                preview.dfir_for(&participants).to_mermaid(&WriteConfig {
                    no_subgraphs: true,
                    no_pull_push: true,
                    no_handoffs: true,
                    op_text_no_imports: true,
                    ..WriteConfig::default()
                })
            );
        });
    }

    #[tokio::test]
    async fn two_pc_some_throughput() {
        let builder = hydro_lang::FlowBuilder::new();
        let coordinator = builder.process();
        let participants = builder.cluster();
        let clients = builder.cluster();
        let client_aggregator = builder.process();

        create_two_pc(&coordinator, &participants, &clients, &client_aggregator);
        let mut deployment = Deployment::new();

        let nodes = builder
            .with_process(&coordinator, TrybuildHost::new(deployment.Localhost()))
            .with_cluster(
                &participants,
                (0..NUM_PARTICIPANTS).map(|_| TrybuildHost::new(deployment.Localhost())),
            )
            .with_cluster(&clients, vec![TrybuildHost::new(deployment.Localhost())])
            .with_process(
                &client_aggregator,
                TrybuildHost::new(deployment.Localhost()),
            )
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let client_node = &nodes.get_process(&client_aggregator);
        let client_out = client_node.stdout_filter("Throughput:").await;

        deployment.start().await.unwrap();

        use std::str::FromStr;

        use regex::Regex;

        let re = Regex::new(r"Throughput: ([^ ]+) - ([^ ]+) - ([^ ]+) requests/s").unwrap();
        let mut found = 0;
        let mut client_out = client_out;
        while let Some(line) = client_out.recv().await {
            if let Some(caps) = re.captures(&line)
                && let Ok(lower) = f64::from_str(&caps[1])
                && 0.0 < lower
            {
                println!("Found throughput lower-bound: {}", lower);
                found += 1;
                if found == 2 {
                    break;
                }
            }
        }
    }

    #[test]
    fn two_pc_partition_coordinator() {
        let builder = hydro_lang::FlowBuilder::new();
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
        let coordinator_partitioning =
            partitioning_analysis(&mut ir, &coordinator.id(), &cycle_data);
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

        insta::assert_debug_snapshot!(&ir);
    }

    #[test]
    fn two_pc_partition_participant() {
        let builder = hydro_lang::FlowBuilder::new();
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

        let participant_partitioning =
            partitioning_analysis(&mut ir, &participants.id(), &cycle_data);
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

        insta::assert_debug_snapshot!(&ir);
    }
}
