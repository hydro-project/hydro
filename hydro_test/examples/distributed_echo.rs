use clap::Parser;
use futures::{SinkExt, StreamExt};
use hydro_deploy::{AwsNetwork, Deployment};
use hydro_lang::deploy::{DockerDeploy, DockerNetwork};
use hydro_lang::prelude::FlowBuilder;
use hydro_lang::telemetry;
use hydro_test::distributed::distributed_echo::distributed_echo;

const CLUSTER_SIZE: usize = 2;

async fn docker() {
    telemetry::initialize_tracing_with_filter(
        tracing_subscriber::EnvFilter::try_new("trace,hyper=off").unwrap(),
    );

    let network = DockerNetwork::new("distributed_echo_test".to_string());

    let mut deployment = DockerDeploy::new(network);

    let builder = FlowBuilder::new();
    let external = builder.external();
    let p1 = builder.process();
    let c2 = builder.cluster();
    let c3 = builder.cluster();
    let p4 = builder.process();
    let p5 = builder.process();
    let (external_sink, external_stream, ems_c2, ems_c3) =
        distributed_echo(&external, &p1, &c2, &c3, &p4, &p5);

    let config = vec![r#"profile.dev.strip="symbols""#.to_string()];

    let nodes = builder
        .with_process(&p1, deployment.add_localhost_docker(None, config.clone()))
        .with_cluster(
            &c2,
            deployment.add_localhost_docker_cluster(None, config.clone(), CLUSTER_SIZE),
        )
        .with_cluster(
            &c3,
            deployment.add_localhost_docker_cluster(None, config.clone(), CLUSTER_SIZE),
        )
        .with_process(&p4, deployment.add_localhost_docker(None, config.clone()))
        .with_process(&p5, deployment.add_localhost_docker(None, config.clone()))
        .with_external(&external, deployment.add_external("external".to_string()))
        .deploy(&mut deployment);

    deployment.provision(&nodes).await.unwrap();
    deployment.start(&nodes).await.unwrap();

    let mut external_sink = nodes.connect(external_sink).await;
    let mut external_stream = nodes.connect(external_stream).await;

    let mut ems_c2_stream = nodes.connect(ems_c2).await;
    let mut ems_c3_stream = nodes.connect(ems_c3).await;

    println!("waiting for c2 events");
    {
        let mut events = Vec::new();
        events.push(ems_c2_stream.next().await.unwrap());
        println!("got 1 c2 events");
        events.push(ems_c2_stream.next().await.unwrap());
        println!("got 2 c2 events");
    }

    println!("waiting for c3 events");
    {
        let mut events = Vec::new();
        events.push(ems_c3_stream.next().await.unwrap());
        println!("got 1 c3 events");
        events.push(ems_c3_stream.next().await.unwrap());
        println!("got 2 c3 events");
    }

    tokio::time::sleep(std::time::Duration::from_secs(1)).await; // TODO: hack to get around a timing issue.

    println!("sending 0");
    external_sink.send(0).await.unwrap();
    assert_eq!(external_stream.next().await.unwrap(), 5);

    println!("sending 1");

    external_sink.send(1).await.unwrap();
    assert_eq!(external_stream.next().await.unwrap(), 6);

    external_sink.send(2).await.unwrap();
    assert_eq!(external_stream.next().await.unwrap(), 7);

    external_sink.send(3).await.unwrap();
    assert_eq!(external_stream.next().await.unwrap(), 8);

    deployment.stop(&nodes).await.unwrap();

    deployment.cleanup(&nodes).await.unwrap();

    println!("successfully deployed and cleaned up");
}

async fn localhost() {
    let mut deployment = Deployment::new();

    let builder = FlowBuilder::new();
    let external = builder.external();
    let p1 = builder.process();
    let c2 = builder.cluster();
    let c3 = builder.cluster();
    let p4 = builder.process();
    let p5 = builder.process();
    let (external_sink, external_stream, ems_c2, ems_c3) =
        distributed_echo(&external, &p1, &c2, &c3, &p4, &p5);

    let nodes = builder
        .with_process(&p1, deployment.Localhost())
        .with_cluster(&c2, (0..CLUSTER_SIZE).map(|_| deployment.Localhost()))
        .with_cluster(&c3, (0..CLUSTER_SIZE).map(|_| deployment.Localhost()))
        .with_process(&p4, deployment.Localhost())
        .with_process(&p5, deployment.Localhost())
        .with_external(&external, deployment.Localhost())
        .deploy(&mut deployment);

    deployment.deploy().await.unwrap();
    deployment.start().await.unwrap();

    let mut external_sink = nodes.connect(external_sink).await;
    let mut external_stream = nodes.connect(external_stream).await;

    let mut ems_c2_stream = nodes.connect(ems_c2).await;
    let mut ems_c3_stream = nodes.connect(ems_c3).await;

    println!("waiting for c2 events");
    {
        let mut events = Vec::new();
        events.push(ems_c2_stream.next().await.unwrap());
        println!("got 1 c2 events");
        events.push(ems_c2_stream.next().await.unwrap());
        println!("got 2 c2 events");
    }

    println!("waiting for c3 events");
    {
        let mut events = Vec::new();
        events.push(ems_c3_stream.next().await.unwrap());
        println!("got 1 c3 events");
        events.push(ems_c3_stream.next().await.unwrap());
        println!("got 2 c3 events");
    }

    println!("sending 0");
    external_sink.send(0).await.unwrap();
    assert_eq!(external_stream.next().await.unwrap(), 5);

    println!("sending 1");

    external_sink.send(1).await.unwrap();
    assert_eq!(external_stream.next().await.unwrap(), 6);

    external_sink.send(2).await.unwrap();
    assert_eq!(external_stream.next().await.unwrap(), 7);

    external_sink.send(3).await.unwrap();
    assert_eq!(external_stream.next().await.unwrap(), 8);

    deployment.stop().await.unwrap();

    println!("successfully deployed and cleaned up");
}

async fn aws() {
    let mut deployment: Deployment = Deployment::new();

    let builder = FlowBuilder::new();
    let external = builder.external();
    let p1 = builder.process();
    let c2 = builder.cluster();
    let c3 = builder.cluster();
    let p4 = builder.process();
    let p5 = builder.process();
    let (external_sink, external_stream, ems_c2, ems_c3) =
        distributed_echo(&external, &p1, &c2, &c3, &p4, &p5);

    let network = AwsNetwork::new("us-east-1", None);

    let nodes = builder
        .with_process(
            &p1,
            deployment.AwsEc2Host()
                        .region("us-east-1")
                        .instance_type("t3.micro")
                        .ami("ami-0e95a5e2743ec9ec9") // Amazon Linux 2
                        .network(network.clone())
                        .add(),
        )
        .with_cluster(
            &c2,
            (0..CLUSTER_SIZE).map(|_| {
                deployment.AwsEc2Host()
                        .region("us-east-1")
                        .instance_type("t3.micro")
                        .ami("ami-0e95a5e2743ec9ec9") // Amazon Linux 2
                        .network(network.clone())
                        .add()
            }),
        )
        .with_cluster(
            &c3,
            (0..CLUSTER_SIZE).map(|_| {
                deployment.AwsEc2Host()
                        .region("us-east-1")
                        .instance_type("t3.micro")
                        .ami("ami-0e95a5e2743ec9ec9") // Amazon Linux 2
                        .network(network.clone())
                        .add()
            }),
        )
        .with_process(
            &p4,
            deployment.AwsEc2Host()
                        .region("us-east-1")
                        .instance_type("t3.micro")
                        .ami("ami-0e95a5e2743ec9ec9") // Amazon Linux 2
                        .network(network.clone())
                        .add(),
        )
        .with_process(
            &p5,
            deployment.AwsEc2Host()
                        .region("us-east-1")
                        .instance_type("t3.micro")
                        .ami("ami-0e95a5e2743ec9ec9") // Amazon Linux 2
                        .network(network.clone())
                        .add(),
        )
        .with_external(&external, deployment.Localhost())
        .deploy(&mut deployment);

    deployment.deploy().await.unwrap();

    let mut external_sink = nodes.connect(external_sink).await;
    let mut external_stream = nodes.connect(external_stream).await;

    let mut ems_c2_stream = nodes.connect(ems_c2).await;
    let mut ems_c3_stream = nodes.connect(ems_c3).await;

    println!("waiting for c2 events");
    {
        let mut events = Vec::new();
        events.push(ems_c2_stream.next().await.unwrap());
        println!("got 1 c2 events");
        events.push(ems_c2_stream.next().await.unwrap());
        println!("got 2 c2 events");
    }

    println!("waiting for c3 events");
    {
        let mut events = Vec::new();
        events.push(ems_c3_stream.next().await.unwrap());
        println!("got 1 c3 events");
        events.push(ems_c3_stream.next().await.unwrap());
        println!("got 2 c3 events");
    }

    println!("sending 0");
    external_sink.send(0).await.unwrap();
    assert_eq!(external_stream.next().await.unwrap(), 5);

    println!("sending 1");

    external_sink.send(1).await.unwrap();
    assert_eq!(external_stream.next().await.unwrap(), 6);

    external_sink.send(2).await.unwrap();
    assert_eq!(external_stream.next().await.unwrap(), 7);

    external_sink.send(3).await.unwrap();
    assert_eq!(external_stream.next().await.unwrap(), 8);

    deployment.stop().await.unwrap();

    println!("successfully deployed and cleaned up");
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum DeployMode {
    Docker,
    Localhost,
    Aws,
}

#[derive(Parser, Debug)]
struct Args {
    #[clap(long, value_enum)]
    mode: DeployMode,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    match args.mode {
        DeployMode::Docker => docker().await,
        DeployMode::Aws => aws().await,
        DeployMode::Localhost => localhost().await,
    }
}

// only run this test on linux
// macos in github actions does not have docker installed.
// windows currently has a stack overflow issue that is being debugged in parallel.
#[cfg(target_os = "linux")]
#[test]
fn test_distributed_echo_example() {
    use example_test::run_current_example;

    let mut run = run_current_example!("--mode localhost");
    run.read_string("successfully deployed and cleaned up");

    let mut run = run_current_example!("--mode docker");
    run.read_string("successfully deployed and cleaned up");
}
