use std::time::{Duration, SystemTime};

use hydro_lang::live_collections::stream::NoOrder;
use hydro_lang::prelude::*;
use hydro_std::bench_client::{
    KeyedProtocol, WorkloadGenerator, bench_client, compute_throughput_latency, print_bench_results,
};

use super::two_pc::{Coordinator, Participant};
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
    let latencies = bench_client(
        clients,
        num_clients_per_node,
        IncU32WorkloadGenerator {},
        TwoPCProtocol {
            coordinator,
            participants,
            num_participants,
            clients,
        },
    );

    // Create throughput/latency graphs
    let bench_results = compute_throughput_latency(clients, latencies, nondet!(/** bench */));
    print_bench_results(bench_results, client_aggregator, clients);
}

struct IncU32WorkloadGenerator {}
impl<'a> WorkloadGenerator<'a, Client, (i32, SystemTime)> for IncU32WorkloadGenerator {
    fn generate_workload(
        self,
        ids_and_prev_payloads: KeyedStream<
            u32,
            Option<(i32, SystemTime)>,
            Cluster<'a, Client>,
            Unbounded,
            NoOrder,
        >,
    ) -> KeyedStream<u32, (i32, SystemTime), Cluster<'a, Client>, Unbounded, NoOrder> {
        ids_and_prev_payloads.map(q!(move |payload| {
            let value = if let Some((counter, _time)) = payload {
                counter + 1
            } else {
                0
            };
            // Record current time for latency
            (value, SystemTime::now())
        }))
    }
}

struct TwoPCProtocol<'a> {
    coordinator: &'a Process<'a, Coordinator>,
    participants: &'a Cluster<'a, Participant>,
    num_participants: usize,
    clients: &'a Cluster<'a, Client>,
}
impl<'a> KeyedProtocol<'a, Client, (i32, SystemTime), Stream<Duration, Cluster<'a, Client>, Unbounded, NoOrder>> for TwoPCProtocol<'a> {
    fn protocol(
        self,
        input: KeyedStream<u32, (i32, SystemTime), Cluster<'a, Client>, Unbounded, NoOrder>,
    ) -> (
        KeyedStream<u32, (i32, SystemTime), Cluster<'a, Client>, Unbounded, NoOrder>,
        Stream<Duration, Cluster<'a, Client>, Unbounded, NoOrder>,
    ) {
        let completed_payloads = two_pc(
            self.coordinator,
            self.participants,
            self.num_participants,
            input.entries().send(self.coordinator, TCP.bincode()).entries(),
        )
        .demux(self.clients, TCP.bincode())
        .into_keyed();

        // Calculate latencies
        let latencies = completed_payloads.clone().values().map(q!(move |(_counter, time)| {
            SystemTime::now().duration_since(time).unwrap()
        }));
        (completed_payloads, latencies)
    }
}

#[cfg(test)]
mod tests {
    use dfir_lang::graph::WriteConfig;
    use hydro_deploy::Deployment;
    use hydro_lang::deploy::{DeployCrateWrapper, HydroDeploy, TrybuildHost};
    #[cfg(stageleft_runtime)]
    use hydro_lang::location::{Cluster, Process};

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
        let builder = hydro_lang::compile::builder::FlowBuilder::new();
        let coordinator = builder.process();
        let participants = builder.cluster();
        let clients = builder.cluster();
        let client_aggregator = builder.process();

        create_two_pc(&coordinator, &participants, &clients, &client_aggregator);
        let mut built = builder.with_default_optimize::<HydroDeploy>();

        hydro_lang::compile::ir::dbg_dedup_tee(|| {
            hydro_build_utils::assert_debug_snapshot!(built.ir());
        });

        let preview = built.preview_compile();
        hydro_build_utils::insta::with_settings!({
            snapshot_suffix => "coordinator_mermaid"
        }, {
            hydro_build_utils::assert_snapshot!(
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
        hydro_build_utils::insta::with_settings!({
            snapshot_suffix => "participants_mermaid"
        }, {
            hydro_build_utils::assert_snapshot!(
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
        let builder = hydro_lang::compile::builder::FlowBuilder::new();
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
        let client_out = client_node.stdout_filter("Throughput:");

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
}
