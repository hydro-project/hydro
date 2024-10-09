use hydro_deploy::Deployment;
use hydroflow_plus::deploy::TrybuildHost;

#[tokio::main]
async fn main() {
    let mut deployment = Deployment::new();
    let _localhost = deployment.Localhost();

    let builder = hydroflow_plus::FlowBuilder::new();
    let num_participants: u32 = 3;

    let (coordinator, participants, client) =
        hydroflow_plus_test::cluster::two_pc::two_pc(&builder, num_participants);

    let _rustflags = "-C opt-level=3 -C codegen-units=1 -C strip=none -C debuginfo=2 -C lto=off";

    let _nodes = builder
        .with_default_optimize()
        .with_process(&coordinator, TrybuildHost::new(deployment.Localhost()))
        .with_cluster(
            &participants,
            (0..num_participants)
                .map(|_| TrybuildHost::new(deployment.Localhost()))
                .collect::<Vec<_>>(),
        )
        .with_process(&client, TrybuildHost::new(deployment.Localhost()))
        .deploy(&mut deployment);

    deployment.deploy().await.unwrap();

    deployment.start().await.unwrap();

    tokio::signal::ctrl_c().await.unwrap();
}
