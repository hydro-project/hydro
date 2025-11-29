use hydro_deploy::Deployment;
use hydro_lang::location::{Location, NetworkHint};
use tokio_util::codec::LinesCodec;

#[tokio::main]
async fn main() {
    let mut deployment = Deployment::new();

    let flow = hydro_lang::compile::builder::FlowBuilder::new();
    let process = flow.process();
    let external = flow.external::<()>();

    let (_port, input, output) =
        process.bind_single_client::<_, _, LinesCodec>(&external, NetworkHint::TcpPort(Some(4000)));
    output.complete(hydro_template::echo_capitalize(input));

    let _nodes = flow
        .with_process(&process, deployment.Localhost())
        .with_external(&external, deployment.Localhost())
        .deploy(&(), &mut deployment);

    deployment.deploy().await.unwrap();

    println!("Launched Echo Server! Run `nc localhost 4000` to connect.");

    deployment.start_ctrl_c().await.unwrap();
}
