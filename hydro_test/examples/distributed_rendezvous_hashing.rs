use dfir_rs::tokio_stream::StreamExt;
use futures::SinkExt;
use hydro_lang::deploy::{DockerDeploy, DockerNetwork};
use hydro_test::distributed::rendezvous_hashing::{Command, distributed_rendezvous_partitioning};
// use hydro_lang::{compile::deploy_provider::Deploy, location::NetworkHint};

#[tokio::main]
async fn main() {
    let network = DockerNetwork::new("distributed_rendezvous_hashing".to_string());

    let mut deployment = DockerDeploy::new(network);

    let builder = hydro_lang::compile::builder::FlowBuilder::new();
    let external = builder.external();
    let p1 = builder.process();
    let c2 = builder.cluster();
    let p3 = builder.process();
    let (external_sink, external_stream) =
        distributed_rendezvous_partitioning(&external, &p1, &c2, &p3);

    let config = vec![
        // r#"profile.dev.lto="fat""#.to_string(),
        // r#"profile.dev.strip="symbols""#.to_string(),
        // r#"profile.dev.opt-level=3"#.to_string(),
        // r#"profile.dev.panic="abort""#.to_string(),
        // r#"profile.dev.codegen-units=1"#.to_string(),
    ];

    let nodes = builder
        .with_process(
            &p1,
            deployment.add_localhost_docker("queryer".to_string(), None, config.clone()),
        )
        .with_cluster(
            &c2,
            deployment.add_localhost_docker_cluster(
                "MyCluster".to_string(),
                None,
                config.clone(),
                3,
            ),
        )
        .with_process(
            &p3,
            deployment.add_localhost_docker("echoer".to_string(), None, config.clone()),
        )
        .with_external(&external, deployment.add_external("external".to_string()))
        .deploy(&mut deployment);

    deployment.provision(&nodes).await.unwrap();
    deployment.start(&nodes).await.unwrap();

    let mut external_stream = nodes.connect(external_stream).await;

    let mut external_sink = nodes.connect(external_sink).await;

    for i in 0..10 {
        external_sink
            .send(Command::Put(format!("hello{i}"), format!("{i}")))
            .await
            .unwrap();
    }

    for i in 0..10 {
        external_sink
            .send(Command::Get(format!("hello{i}")))
            .await
            .unwrap();
    }

    for _ in 0..10 {
        let (member_id, data) = external_stream.next().await.unwrap();
        eprintln!("received: from: {}, data: {:?}", member_id, data);
    }

    deployment.stop(&nodes).await.unwrap();

    eprintln!("success");
}

// #[tokio::test]
// async fn distributed_echo_localhost() {
//     let mut deployment = Deployment::new();

//     let builder = hydro_lang::compile::builder::FlowBuilder::new();
//     let external = builder.external();
//     let p1 = builder.process();
//     let p2 = builder.process();
//     let p3 = builder.process();
//     let (external_sink, external_stream) = super::distributed_echo(&external, &p1, &p2, &p3);

//     let nodes = builder
//         .with_process(&p1, deployment.Localhost())
//         .with_process(&p2, deployment.Localhost())
//         .with_process(&p3, deployment.Localhost())
//         .with_external(&external, deployment.Localhost())
//         .deploy(&mut deployment);

//     deployment.deploy().await.unwrap();

//     let mut external_sink = nodes.connect(external_sink).await;
//     let mut external_stream = nodes.connect(external_stream).await;

//     deployment.start().await.unwrap();

//     external_sink.send(7).await.unwrap();
//     assert_eq!(external_stream.next().await.unwrap(), 13);

//     deployment.stop().await.unwrap();
// }

// #[ignore]
// #[tokio::test(flavor = "multi_thread")]
// async fn distributed_echo_aws() {
//     let mut deployment: Deployment = Deployment::new();

//     let builder = hydro_lang::compile::builder::FlowBuilder::new();
//     let external = builder.external();
//     let p1 = builder.process();
//     let p2 = builder.process();
//     let p3 = builder.process();
//     let (external_sink, external_stream) = super::distributed_echo(&external, &p1, &p2, &p3);

//     let network = Arc::new(RwLock::new(AwsNetwork::new("us-east-1", None)));

//     let nodes = builder
//         .with_process(
//             &p1,
//             deployment.AwsEc2Host()
//                     .region("us-east-1")
//                     .instance_type("t3.micro")
//                     .ami("ami-0e95a5e2743ec9ec9") // Amazon Linux 2
//                     .network(network.clone())
//                     .add(),
//         )
//         .with_process(
//             &p2,
//             deployment.AwsEc2Host()
//                     .region("us-east-1")
//                     .instance_type("t3.micro")
//                     .ami("ami-0e95a5e2743ec9ec9") // Amazon Linux 2
//                     .network(network.clone())
//                     .add(),
//         )
//         .with_process(
//             &p3,
//             deployment.AwsEc2Host()
//                     .region("us-east-1")
//                     .instance_type("t3.micro")
//                     .ami("ami-0e95a5e2743ec9ec9") // Amazon Linux 2
//                     .network(network.clone())
//                     .add(),
//         )
//         .with_external(&external, deployment.Localhost())
//         .deploy(&mut deployment);

//     deployment.deploy().await.unwrap();

//     let mut external_sink = nodes.connect(external_sink).await;
//     let mut external_stream = nodes.connect(external_stream).await;

//     deployment.start().await.unwrap();

//     external_sink.send(7).await.unwrap();
//     assert_eq!(external_stream.next().await.unwrap(), 13);

//     deployment.stop().await.unwrap();
// }

// #[tokio::test]
// async fn distributed_echo_containerized() {
//     let mut deployment = DockerDeploy::new();

//     let builder = hydro_lang::compile::builder::FlowBuilder::new();
//     let external = builder.external();
//     let p1 = builder.process();
//     let p2 = builder.process();
//     let p3 = builder.process();
//     let (external_sink, external_stream) = super::distributed_echo(&external, &p1, &p2, &p3);

//     let network = DockerNetwork::new("distributed_echo_test".to_string());

//     let nodes = builder
//         .with_process(
//             &p1,
//             deployment.add_docker("p1".to_string(), network.clone()),
//         )
//         .with_process(
//             &p2,
//             deployment.add_docker("p2".to_string(), network.clone()),
//         )
//         .with_process(
//             &p3,
//             deployment.add_docker("p3".to_string(), network.clone()),
//         )
//         .with_external(&external, deployment.add_external("external".to_string()))
//         .deploy(&mut deployment);

//     deployment.provision().await.unwrap();
//     deployment.start().await.unwrap();

//     let mut external_stream = nodes.connect(external_stream).await;

//     tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

//     let mut external_sink = nodes.connect(external_sink).await;

//     external_sink.send(7).await.unwrap();
//     assert_eq!(external_stream.next().await.unwrap(), 13);

//     deployment.stop().await.unwrap();
// }
