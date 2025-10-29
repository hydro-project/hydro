use hydro_lang::prelude::*;
use hydro_deploy::Deployment;

#[tokio::main]
async fn main() {
    let mut deployment = Deployment::new();

    let flow = hydro_lang::compile::builder::FlowBuilder::new();
    let process = flow.process();
    let external = flow.external::<()>();

    let (in_port, input) = process.source_external_bincode(&external);
    hydro_template::echo_capitalize(input)
        .for_each(q!(|s| {
            println!("Echoed capitalized: {}", s);
        }));

    let nodes = flow
        .with_process(&process, deployment.Localhost())
        .with_external(&external, deployment.Localhost())
        .deploy(&mut deployment);

    deployment.deploy().await.unwrap();

    let mut external_in = nodes.connect(in_port).await;

    deployment.start().await.unwrap();

    for line in std::io::stdin().lines() {
        let msg = msg.unwrap();
        if msg.is_empty() {
            break;
        }
        external_in.send(msg).await.unwrap();
    }
    external_in.send("hello".to_string()).await.unwrap();
    external_in.send("world".to_string()).await.unwrap();
}
