use dfir_rs::tokio_stream::StreamExt;
use futures::SinkExt;
use hydro_lang::deploy::{DockerDeploy, DockerNetwork};
use hydro_test::distributed::rendezvous_hashing::{Command, distributed_rendezvous_partitioning};

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
        println!("received: from: {}, data: {:?}", member_id, data);
    }

    deployment.stop(&nodes).await.unwrap();

    println!("success");
}

#[tokio::test]
async fn docker() {
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

#[tokio::test]
async fn localhost() {
    use hydro_deploy::Deployment;
    use hydro_lang::deploy::TrybuildHost;

    let mut deployment = Deployment::new();

    let builder = hydro_lang::compile::builder::FlowBuilder::new();
    let external = builder.external();
    let p1 = builder.process();
    let c2 = builder.cluster();
    let p3 = builder.process();
    let (external_sink, external_stream) =
        distributed_rendezvous_partitioning(&external, &p1, &c2, &p3);

    let nodes = builder
        .with_process(&p1, TrybuildHost::new(deployment.Localhost()))
        .with_cluster(
            &c2,
            (0..2).map(|_| TrybuildHost::new(deployment.Localhost())),
        )
        .with_process(&p3, TrybuildHost::new(deployment.Localhost()))
        .with_external(&external, deployment.Localhost())
        .deploy(&mut deployment);

    deployment.deploy().await.unwrap();

    let mut external_stream = nodes.connect(external_stream).await;

    let mut external_sink = nodes.connect(external_sink).await;

    deployment.start().await.unwrap();

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
        println!("received: from: {}, data: {:?}", member_id, data);
    }

    deployment.stop().await.unwrap();

    println!("success");
}
