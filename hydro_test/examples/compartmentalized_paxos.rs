use std::sync::Arc;

use hydro_deploy::gcp::GcpNetwork;
use hydro_deploy::{Deployment, Host};
use hydro_lang::deploy::TrybuildHost;
use hydro_test::cluster::compartmentalized_paxos::{
    CompartmentalizedPaxosConfig, CoreCompartmentalizedPaxos,
};
use hydro_test::cluster::paxos::PaxosConfig;
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
                .machine_type("n2-highcpu-2")
                .image("debian-cloud/debian-11")
                .region("us-west1-a")
                .network(network.clone())
                .add()
        })
    } else {
        let localhost = deployment.Localhost();
        Box::new(move |_| -> Arc<dyn Host> { localhost.clone() })
    };

    let builder = hydro_lang::FlowBuilder::new();
    let f = 1;
    let num_clients = 1;
    let num_clients_per_node = 100; // Change based on experiment between 1, 50, 100.
    let checkpoint_frequency = 1000; // Num log entries
    let i_am_leader_send_timeout = 5; // Sec
    let i_am_leader_check_timeout = 10; // Sec
    let i_am_leader_check_timeout_delay_multiplier = 15;

    let num_proxy_leaders = 10;
    let acceptor_grid_rows = 2;
    let acceptor_grid_cols = 2;
    let num_replicas = 4;
    let acceptor_retry_timeout = 10; // Sec

    let proposers = builder.cluster();
    let proxy_leaders = builder.cluster();
    let acceptors = builder.cluster();
    let clients = builder.cluster();
    let client_aggregator = builder.process();
    let replicas = builder.cluster();

    hydro_test::cluster::paxos_bench::paxos_bench(
        num_clients_per_node,
        checkpoint_frequency,
        f,
        num_replicas,
        CoreCompartmentalizedPaxos {
            proposers: proposers.clone(),
            proxy_leaders: proxy_leaders.clone(),
            acceptors: acceptors.clone(),
            config: CompartmentalizedPaxosConfig {
                paxos_config: PaxosConfig {
                    f,
                    i_am_leader_send_timeout,
                    i_am_leader_check_timeout,
                    i_am_leader_check_timeout_delay_multiplier,
                },
                num_proxy_leaders,
                acceptor_grid_rows,
                acceptor_grid_cols,
                num_replicas,
                acceptor_retry_timeout,
            },
        },
        &clients,
        &client_aggregator,
        &replicas,
    );

    let rustflags = "-C opt-level=3 -C codegen-units=1 -C strip=none -C debuginfo=2 -C lto=off";

    let _nodes = builder
        .with_cluster(
            &proposers,
            (0..f + 1)
                .map(|_| TrybuildHost::new(create_host(&mut deployment)).rustflags(rustflags)),
        )
        .with_cluster(
            &proxy_leaders,
            (0..num_proxy_leaders)
                .map(|_| TrybuildHost::new(create_host(&mut deployment)).rustflags(rustflags)),
        )
        .with_cluster(
            &acceptors,
            (0..acceptor_grid_rows * acceptor_grid_cols)
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
        .with_cluster(
            &replicas,
            (0..num_replicas)
                .map(|_| TrybuildHost::new(create_host(&mut deployment)).rustflags(rustflags)),
        )
        .deploy(&mut deployment);

    deployment.deploy().await.unwrap();

    deployment.start().await.unwrap();

    tokio::signal::ctrl_c().await.unwrap();
}
