use std::sync::Arc;

use dfir_rs::tokio_stream::StreamExt;
use futures::SinkExt;
use hydro_deploy::{AwsNetwork, Deployment};
use hydro_lang::deploy::TrybuildHost;
use hydro_test::distributed::distributed_echo::distributed_echo;
use tokio::sync::RwLock;
// use hydro_lang::{compile::deploy_provider::Deploy, location::NetworkHint};

#[ignore]
#[tokio::main(flavor = "multi_thread")]
async fn main() {
    let mut deployment: Deployment = Deployment::new();

    let builder = hydro_lang::compile::builder::FlowBuilder::new();
    let external = builder.external();
    let p1 = builder.process();
    let p2 = builder.process();
    let p3 = builder.process();
    let (external_sink, external_stream) = distributed_echo(&external, &p1, &p2, &p3);

    let network = Arc::new(RwLock::new(AwsNetwork::new("us-east-1", None)));

    let nodes = builder
        .with_process(
            &p1,
            TrybuildHost::new(
                deployment.AwsEc2Host()
                    .region("us-east-1")
                    .instance_type("t3.micro")
                    .ami("ami-0e95a5e2743ec9ec9") // Amazon Linux 2
                    .network(network.clone())
                    .add(),
            )
            .rustflags(
                "-C opt-level=3 -C codegen-units=1 -C strip=debuginfo -C debuginfo=0 -C lto=off",
            ),
        )
        .with_process(
            &p2,
            TrybuildHost::new(
                deployment.AwsEc2Host()
                    .region("us-east-1")
                    .instance_type("t3.micro")
                    .ami("ami-0e95a5e2743ec9ec9") // Amazon Linux 2
                    .network(network.clone())
                    .add(),
            )
            .rustflags(
                "-C opt-level=3 -C codegen-units=1 -C strip=debuginfo -C debuginfo=0 -C lto=off",
            ),
        )
        .with_process(
            &p3,
            TrybuildHost::new(
                deployment.AwsEc2Host()
                    .region("us-east-1")
                    .instance_type("t3.micro")
                    .ami("ami-0e95a5e2743ec9ec9") // Amazon Linux 2
                    .network(network.clone())
                    .add(),
            )
            .rustflags(
                "-C opt-level=3 -C codegen-units=1 -C strip=debuginfo -C debuginfo=0 -C lto=off",
            ),
        )
        .with_external(&external, deployment.Localhost())
        .deploy(&mut deployment);

    deployment.deploy().await.unwrap();

    let mut external_sink = nodes.connect(external_sink).await;
    let mut external_stream = nodes.connect(external_stream).await;

    deployment.start().await.unwrap();

    external_sink.send(7).await.unwrap();
    assert_eq!(external_stream.next().await.unwrap(), 13);

    deployment.stop().await.unwrap();
}
