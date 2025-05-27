use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use hydro_deploy::gcp::GcpNetwork;
use hydro_deploy::{Deployment, Host};
use hydro_lang::deploy::TrybuildHost;
use hydro_test::cluster::paxos::{CorePaxos, PaxosConfig};
use tokio::sync::RwLock;
use tokio::time::sleep;

struct ReusableHosts {
    hosts: HashMap<String, Arc<dyn Host>>, // Key = display_name
    next_available_host_index: usize,
    host_arg: String,
    project: String,
    network: Arc<RwLock<GcpNetwork>>,
}

impl ReusableHosts {
    // NOTE: Creating hosts with the same display_name in the same deployment will result in undefined behavior.
    fn get_host(&mut self, deployment: &mut Deployment, display_name: String) -> Arc<dyn Host> {
        self.hosts.entry(display_name.clone())
            .or_insert_with(|| {
                if self.host_arg == "gcp" {
                    deployment
                        .GcpComputeEngineHost()
                        .project(&self.project)
                        .machine_type("n2-standard-4")
                        .image("debian-cloud/debian-12")
                        .region("us-central1-c")
                        .network(self.network.clone())
                        .display_name(display_name)
                        .add()
                } else {
                    deployment.Localhost()
                }
            })
            .clone()
    }
}


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

    let mut reusable_hosts = ReusableHosts{ 
        hosts: HashMap::new(),
        next_available_host_index: 0,
        host_arg: host_arg,
        project: project.clone(),
        network: network.clone(),
    };

    let f = 1;
    let checkpoint_frequency = 1000; // Num log entries
    let i_am_leader_send_timeout = 5; // Sec
    let i_am_leader_check_timeout = 10; // Sec
    let i_am_leader_check_timeout_delay_multiplier = 15;

    // Benchmark parameters
    let num_clients = [1,2];
    let num_clients_per_node = vec![1,500,1000,2000,3000];
    let run_seconds = 60;

    let max_num_clients_per_node = num_clients_per_node.iter().max().unwrap();
    for (i, num_clients) in num_clients.iter().enumerate() {
        
        // For the 1st client, test a variable number of virtual clients. For the rest, use the max number.
        let virtual_clients = if i == 0 {
            &num_clients_per_node
        } else {
            &vec![max_num_clients_per_node.clone()]
        };

        for num_clients_per_node in virtual_clients {
            println!("Running Paxos with {} clients and {} virtual clients per node for {} seconds", num_clients, num_clients_per_node, run_seconds);

            let builder = hydro_lang::FlowBuilder::new();
            let proposers = builder.cluster();
            let acceptors = builder.cluster();
            let clients = builder.cluster();
            let client_aggregator = builder.process();
            let replicas = builder.cluster();

            hydro_test::cluster::paxos_bench::paxos_bench(
                *num_clients_per_node,
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

            let rustflags = "-C opt-level=3 -C codegen-units=1 -C strip=none -C debuginfo=2 -C lto=off";

            let nodes = builder
                .with_cluster(
                    &proposers,
                    (0..f + 1)
                        .map(|i| TrybuildHost::new(reusable_hosts.get_host(&mut deployment, format!("proposer{}", i))).rustflags(rustflags)),
                )
                .with_cluster(
                    &acceptors,
                    (0..2 * f + 1)
                        .map(|_| TrybuildHost::new(reusable_hosts.get_host(&mut deployment, format!("acceptor{}", i))).rustflags(rustflags)),
                )
                .with_cluster(
                    &clients,
                    (0..*num_clients)
                        .map(|_| TrybuildHost::new(reusable_hosts.get_host(&mut deployment, format!("client{}", i))).rustflags(rustflags)),
                )
                .with_process(
                    &client_aggregator,
                    TrybuildHost::new(reusable_hosts.get_host(&mut deployment, "client-aggregator".into())).rustflags(rustflags),
                )
                .with_cluster(
                    &replicas,
                    (0..f + 1)
                        .map(|_| TrybuildHost::new(reusable_hosts.get_host(&mut deployment, format!("replica{}", i))).rustflags(rustflags)),
                )
                .deploy(&mut deployment);

            deployment.deploy().await.unwrap();
            deployment
                .start_until(sleep(Duration::from_secs(run_seconds)))
                .await
                .unwrap();

            drop(nodes);
        }
    }
}
