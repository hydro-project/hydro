use hydro_lang::location::MemberId;
use hydro_lang::location::external_process::{ExternalBincodeSink, ExternalBincodeStream};
use hydro_lang::prelude::*;

pub struct P1 {}
pub struct P2 {}
pub struct P3 {}

pub fn distributed_echo<'a>(
    external: &External<'a, ()>,
    p1: &Process<'a, P1>,
    p2: &Process<'a, P2>,
    p3: &Process<'a, P3>,
) -> (ExternalBincodeSink<u32>, ExternalBincodeStream<u32>) {
    let (tx, rx) = p1.source_external_bincode(external);

    let z = rx
        .map(q!(|n| n + 1))
        .send_bincode(p2)
        .map(q!(|n| n + 2))
        .send_bincode(p3)
        .map(q!(|n| n + 3))
        .send_bincode_external(external);

    (tx, z)
}

pub struct C2 {}

pub fn distributed_clustered_echo<'a>(
    external: &External<'a, ()>,
    p1: &Process<'a, P1>,
    c2: &Cluster<'a, C2>,
    p3: &Process<'a, P3>,
) -> (
    ExternalBincodeSink<String>,
    ExternalBincodeStream<(MemberId<C2>, String)>,
) {
    let (tx, rx) = p1.source_external_bincode(external);

    let z = rx
        .map(q!(|n| {
            println!("processed element: {n}");
            format!("{n} - p1")
        }))
        .broadcast_bincode(c2, nondet!(/** test */))
        .map(q!(|n| {
            println!("processed element: {n}");
            format!("{n} - c2")
        }))
        .send_bincode(p3)
        .entries()
        .assume_ordering(nondet!(/** test */))
        .map(q!(|(from, v)| {
            println!("processed element: from: {}, v: {v}", from);
            (from, format!("{v} - p3"))
        }))
        .send_bincode_external(external);
    (tx, z)
}

// #[cfg(test)]
// mod tests {
//     use std::sync::Arc;

//     use dfir_rs::tokio_stream::StreamExt;
//     use futures::SinkExt;
//     use hydro_deploy::{AwsNetwork, Deployment};
//     use hydro_lang::deploy::{DockerDeploy, DockerNetwork};
//     // use hydro_lang::{compile::deploy_provider::Deploy, location::NetworkHint};
//     use tokio::sync::RwLock;

//     #[tokio::test]
//     async fn distributed_echo_localhost() {
//         let mut deployment = Deployment::new();

//         let builder = hydro_lang::compile::builder::FlowBuilder::new();
//         let external = builder.external();
//         let p1 = builder.process();
//         let p2 = builder.process();
//         let p3 = builder.process();
//         let (external_sink, external_stream) = super::distributed_echo(&external, &p1, &p2, &p3);

//         let nodes = builder
//             .with_process(&p1, deployment.Localhost())
//             .with_process(&p2, deployment.Localhost())
//             .with_process(&p3, deployment.Localhost())
//             .with_external(&external, deployment.Localhost())
//             .deploy(&mut deployment);

//         deployment.deploy().await.unwrap();

//         let mut external_sink = nodes.connect(external_sink).await;
//         let mut external_stream = nodes.connect(external_stream).await;

//         deployment.start().await.unwrap();

//         external_sink.send(7).await.unwrap();
//         assert_eq!(external_stream.next().await.unwrap(), 13);

//         deployment.stop().await.unwrap();
//     }

//     #[ignore]
//     #[tokio::test(flavor = "multi_thread")]
//     async fn distributed_echo_aws() {
//         let mut deployment: Deployment = Deployment::new();

//         let builder = hydro_lang::compile::builder::FlowBuilder::new();
//         let external = builder.external();
//         let p1 = builder.process();
//         let p2 = builder.process();
//         let p3 = builder.process();
//         let (external_sink, external_stream) = super::distributed_echo(&external, &p1, &p2, &p3);

//         let network = Arc::new(RwLock::new(AwsNetwork::new("us-east-1", None)));

//         let nodes = builder
//             .with_process(
//                 &p1,
//                 deployment.AwsEc2Host()
//                     .region("us-east-1")
//                     .instance_type("t3.micro")
//                     .ami("ami-0e95a5e2743ec9ec9") // Amazon Linux 2
//                     .network(network.clone())
//                     .add(),
//             )
//             .with_process(
//                 &p2,
//                 deployment.AwsEc2Host()
//                     .region("us-east-1")
//                     .instance_type("t3.micro")
//                     .ami("ami-0e95a5e2743ec9ec9") // Amazon Linux 2
//                     .network(network.clone())
//                     .add(),
//             )
//             .with_process(
//                 &p3,
//                 deployment.AwsEc2Host()
//                     .region("us-east-1")
//                     .instance_type("t3.micro")
//                     .ami("ami-0e95a5e2743ec9ec9") // Amazon Linux 2
//                     .network(network.clone())
//                     .add(),
//             )
//             .with_external(&external, deployment.Localhost())
//             .deploy(&mut deployment);

//         deployment.deploy().await.unwrap();

//         let mut external_sink = nodes.connect(external_sink).await;
//         let mut external_stream = nodes.connect(external_stream).await;

//         deployment.start().await.unwrap();

//         external_sink.send(7).await.unwrap();
//         assert_eq!(external_stream.next().await.unwrap(), 13);

//         deployment.stop().await.unwrap();
//     }

//     #[tokio::test]
//     async fn distributed_echo_containerized() {
//         let mut deployment = DockerDeploy::new();

//         let builder = hydro_lang::compile::builder::FlowBuilder::new();
//         let external = builder.external();
//         let p1 = builder.process();
//         let p2 = builder.process();
//         let p3 = builder.process();
//         let (external_sink, external_stream) = super::distributed_echo(&external, &p1, &p2, &p3);

//         let network = DockerNetwork::new("distributed_echo_test".to_string());

//         let nodes = builder
//             .with_process(
//                 &p1,
//                 deployment.add_docker("p1".to_string(), network.clone()),
//             )
//             .with_process(
//                 &p2,
//                 deployment.add_docker("p2".to_string(), network.clone()),
//             )
//             .with_process(
//                 &p3,
//                 deployment.add_docker("p3".to_string(), network.clone()),
//             )
//             .with_external(&external, deployment.add_external("external".to_string()))
//             .deploy(&mut deployment);

//         deployment.provision().await.unwrap();
//         deployment.start().await.unwrap();

//         let mut external_stream = nodes.connect(external_stream).await;

//         tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

//         let mut external_sink = nodes.connect(external_sink).await;

//         external_sink.send(7).await.unwrap();
//         assert_eq!(external_stream.next().await.unwrap(), 13);

//         deployment.stop().await.unwrap();
//     }
// }
