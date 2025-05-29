use hydro_deploy::Deployment;
use hydro_lang::deploy::TrybuildHost;

#[tokio::main]
async fn main() {
    let mut deployment = Deployment::new();
    let builder = hydro_lang::FlowBuilder::new();
    let num_clients: u32 = 3;

    let (server, clients) = hydro_test::cluster::echo_server::echo_server(&builder);

    let _nodes = builder
        .with_process(&server, TrybuildHost::new(deployment.Localhost()))
        .with_cluster(
            &clients,
            (0..num_clients).map(|_| TrybuildHost::new(deployment.Localhost())),
        )
        .deploy(&mut deployment);

    deployment.deploy().await.unwrap();

    deployment.start().await.unwrap();

    tokio::signal::ctrl_c().await.unwrap();
}
