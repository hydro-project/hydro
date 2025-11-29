use dfir_rs::tokio_stream::StreamExt;
use futures::SinkExt;
use hydro_lang::deploy::{DockerDeploy, DockerNetwork};
use hydro_test::distributed::distributed_echo::distributed_echo;

#[tokio::main]
async fn main() {
    let network = DockerNetwork::new("distributed_echo_test".to_string());

    let mut deployment = DockerDeploy::new(network);

    let builder = hydro_lang::compile::builder::FlowBuilder::new();
    let external = builder.external();
    let p1 = builder.process();
    let c2 = builder.cluster();
    let c3 = builder.cluster();
    let p4 = builder.process();
    let p5 = builder.process();
    let (external_sink, external_stream) = distributed_echo(&external, &p1, &c2, &c3, &p4, &p5);

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
            deployment.add_localhost_docker("MyBroadcaster".to_string(), None, config.clone()),
        )
        .with_cluster(
            &c2,
            deployment.add_localhost_docker_cluster(
                "MyCluster1".to_string(),
                None,
                config.clone(),
                2,
            ),
        )
        .with_cluster(
            &c3,
            deployment.add_localhost_docker_cluster(
                "MyCluster2".to_string(),
                None,
                config.clone(),
                2,
            ),
        )
        .with_process(
            &p4,
            deployment.add_localhost_docker("MyReceiver".to_string(), None, config.clone()),
        )
        .with_process(
            &p5,
            deployment.add_localhost_docker("MyReceiver".to_string(), None, config.clone()),
        )
        .with_external(&external, deployment.add_external("external".to_string()))
        .deploy(&deployment.get_deployment_instance(), &mut deployment);

    deployment.provision(&nodes).await.unwrap();
    deployment.start(&nodes).await.unwrap();

    let mut external_stream = nodes.connect(external_stream).await;

    let mut external_sink = nodes.connect(external_sink).await;

    external_sink.send(0).await.unwrap();

    assert_eq!(external_stream.next().await.unwrap(), 5);

    deployment.stop(&nodes).await.unwrap();

    eprintln!("success");
}

#[tokio::test]
async fn docker() {
    let network = DockerNetwork::new("distributed_echo_test".to_string());

    let mut deployment = DockerDeploy::new(network);

    let builder = hydro_lang::compile::builder::FlowBuilder::new();
    let external = builder.external();
    let p1 = builder.process();
    let c2 = builder.cluster();
    let c3 = builder.cluster();
    let p4 = builder.process();
    let p5 = builder.process();
    let (external_sink, external_stream) = distributed_echo(&external, &p1, &c2, &c3, &p4, &p5);

    let config = vec![
        // r#"profile.dev.lto="fat""#.to_string(),
        r#"profile.dev.strip="symbols""#.to_string(),
        // r#"profile.dev.opt-level=3"#.to_string(),
        // r#"profile.dev.panic="abort""#.to_string(),
        // r#"profile.dev.codegen-units=1"#.to_string(),
    ];

    let nodes = builder
        .with_process(
            &p1,
            deployment.add_localhost_docker("MyBroadcaster".to_string(), None, config.clone()),
        )
        .with_cluster(
            &c2,
            deployment.add_localhost_docker_cluster(
                "MyCluster1".to_string(),
                None,
                config.clone(),
                2,
            ),
        )
        .with_cluster(
            &c3,
            deployment.add_localhost_docker_cluster(
                "MyCluster2".to_string(),
                None,
                config.clone(),
                2,
            ),
        )
        .with_process(
            &p4,
            deployment.add_localhost_docker("MyReceiver".to_string(), None, config.clone()),
        )
        .with_process(
            &p5,
            deployment.add_localhost_docker("MyReceiver".to_string(), None, config.clone()),
        )
        .with_external(&external, deployment.add_external("external".to_string()))
        .deploy(&deployment.get_deployment_instance(), &mut deployment);

    deployment.provision(&nodes).await.unwrap();
    deployment.start(&nodes).await.unwrap();

    tokio::time::sleep(std::time::Duration::from_secs(9)).await;

    let mut external_stream = nodes.connect(external_stream).await;

    let mut external_sink = nodes.connect(external_sink).await;

    tokio::time::sleep(std::time::Duration::from_secs(9)).await;

    external_sink.send(0).await.unwrap();
    assert_eq!(external_stream.next().await.unwrap(), 5);

    external_sink.send(1).await.unwrap();
    assert_eq!(external_stream.next().await.unwrap(), 6);

    external_sink.send(2).await.unwrap();
    assert_eq!(external_stream.next().await.unwrap(), 7);

    external_sink.send(3).await.unwrap();
    assert_eq!(external_stream.next().await.unwrap(), 8);

    deployment.stop(&nodes).await.unwrap();

    eprintln!("success");
}

#[tokio::test]
async fn docker2() {
    use hydro_test::distributed::distributed_echo::distributed_echo2;

    let network = DockerNetwork::new("distributed_echo_test".to_string());

    let mut deployment = DockerDeploy::new(network);

    let builder = hydro_lang::compile::builder::FlowBuilder::new();
    let external = builder.external();
    let p1 = builder.process();
    let c2 = builder.cluster();
    let c3 = builder.cluster();
    let p4 = builder.process();
    let p5 = builder.process();
    let (external_sink, external_stream, ems_c2, ems_c3) =
        distributed_echo2(&external, &p1, &c2, &c3, &p4, &p5);

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
            deployment.add_localhost_docker("MyBroadcaster".to_string(), None, config.clone()),
        )
        .with_cluster(
            &c2,
            deployment.add_localhost_docker_cluster(
                "MyCluster1".to_string(),
                None,
                config.clone(),
                2,
            ),
        )
        .with_cluster(
            &c3,
            deployment.add_localhost_docker_cluster(
                "MyCluster2".to_string(),
                None,
                config.clone(),
                2,
            ),
        )
        .with_process(
            &p4,
            deployment.add_localhost_docker("MyReceiver".to_string(), None, config.clone()),
        )
        .with_process(
            &p5,
            deployment.add_localhost_docker("MyReceiver".to_string(), None, config.clone()),
        )
        .with_external(&external, deployment.add_external("external".to_string()))
        .deploy(&deployment.get_deployment_instance(), &mut deployment);

    deployment.provision(&nodes).await.unwrap();
    deployment.start(&nodes).await.unwrap();

    tokio::time::sleep(std::time::Duration::from_secs(9)).await;

    let mut external_sink = nodes.connect(external_sink).await;
    let mut external_stream = nodes.connect(external_stream).await;

    let mut ems_c2_stream = nodes.connect(ems_c2).await;
    let mut ems_c3_stream = nodes.connect(ems_c3).await;

    eprintln!("waiting for c2 events");
    {
        let mut events = Vec::new();
        events.push(ems_c2_stream.next().await.unwrap());
        eprintln!("got 1 c2 events");
        events.push(ems_c2_stream.next().await.unwrap());
        eprintln!("got 2 c2 events");
    }

    eprintln!("waiting for c3 events");
    {
        let mut events = Vec::new();
        events.push(ems_c3_stream.next().await.unwrap());
        eprintln!("got 1 c3 events");
        events.push(ems_c3_stream.next().await.unwrap());
        eprintln!("got 2 c3 events");
    }

    tokio::time::sleep(std::time::Duration::from_secs(9)).await;

    eprintln!("sending 0");
    external_sink.send(0).await.unwrap();
    assert_eq!(external_stream.next().await.unwrap(), 5);

    eprintln!("sending 1");

    external_sink.send(1).await.unwrap();
    assert_eq!(external_stream.next().await.unwrap(), 6);

    external_sink.send(2).await.unwrap();
    assert_eq!(external_stream.next().await.unwrap(), 7);

    external_sink.send(3).await.unwrap();
    assert_eq!(external_stream.next().await.unwrap(), 8);

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
//         .deploy(&(), &mut deployment);

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
//         .deploy(&(), &mut deployment);

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
//         .deploy(&(), &mut deployment);

//     deployment.provision().await.unwrap();
//     deployment.start().await.unwrap();

//     let mut external_stream = nodes.connect(external_stream).await;

//     tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

//     let mut external_sink = nodes.connect(external_sink).await;

//     external_sink.send(7).await.unwrap();
//     assert_eq!(external_stream.next().await.unwrap(), 13);

//     deployment.stop().await.unwrap();
// }
