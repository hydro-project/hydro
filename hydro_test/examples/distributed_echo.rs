use clap::Parser;
use futures::{SinkExt, StreamExt};
use hydro_deploy::{AwsNetwork, Deployment};
use hydro_lang::prelude::FlowBuilder;
use hydro_lang::telemetry;
use hydro_test::distributed::distributed_echo::distributed_echo;

const CLUSTER_SIZE: usize = 2;

/// Common test logic: wait for cluster events, send test messages, verify responses
async fn run_echo_test<S, R, C2, C3>(
    mut external_sink: S,
    mut external_stream: R,
    mut ems_c2_stream: C2,
    mut ems_c3_stream: C3,
) where
    S: SinkExt<u32> + Unpin,
    S::Error: std::fmt::Debug,
    R: StreamExt<Item = u32> + Unpin,
    C2: StreamExt + Unpin,
    C3: StreamExt + Unpin,
{
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
}

#[cfg(feature = "docker")]
async fn docker() {
    use hydro_lang::deploy::{DockerDeploy, DockerNetwork};

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

    let external_sink = nodes.connect(external_sink).await;
    let external_stream = nodes.connect(external_stream).await;
    let ems_c2_stream = nodes.connect(ems_c2).await;
    let ems_c3_stream = nodes.connect(ems_c3).await;

    tokio::time::sleep(std::time::Duration::from_secs(1)).await; // TODO: hack to get around a timing issue.

    run_echo_test(external_sink, external_stream, ems_c2_stream, ems_c3_stream).await;

    deployment.stop(&nodes).await.unwrap();
    deployment.cleanup(&nodes).await.unwrap();

    println!("successfully deployed and cleaned up");
}

#[cfg(feature = "ecs")]
async fn ecs() {
    use hydro_lang::deploy::{DockerDeployEcs, DockerNetworkEcs};

    telemetry::initialize_tracing_with_filter(tracing_subscriber::EnvFilter::try_new(
        "trace,hyper=warn,aws_smithy_runtime=info,aws_sdk_ecs=info,aws_sigv4=info,aws_config=info,aws_runtime=info,aws_smithy_http_client=info,aws_sdk_ec2=info,aws_sdk_ecr=info,h2=warn,aws_sdk_iam=info,aws_sdk_servicediscovery=info,aws_sdk_cloudformation=info",
    ).unwrap());

    let network = DockerNetworkEcs::new("distributed_echo_test".to_string());
    let mut deployment = DockerDeployEcs::new(network);

    let builder = FlowBuilder::new();
    let external = builder.external();
    let p1 = builder.process();
    let c2 = builder.cluster();
    let c3 = builder.cluster();
    let p4 = builder.process();
    let p5 = builder.process();
    let (external_sink, external_stream, ems_c2, ems_c3) =
        distributed_echo(&external, &p1, &c2, &c3, &p4, &p5);

    let config = vec![
        // r#"profile.dev.lto="fat""#.to_string(),
        r#"profile.dev.strip="symbols""#.to_string(),
        // r#"profile.dev.opt-level=3"#.to_string(),
        // r#"profile.dev.panic="abort""#.to_string(),
        // r#"profile.dev.codegen-units=1"#.to_string(),
    ];

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

    let external_sink = nodes.connect(external_sink).await;
    let external_stream = nodes.connect(external_stream).await;
    let ems_c2_stream = nodes.connect(ems_c2).await;
    let ems_c3_stream = nodes.connect(ems_c3).await;

    run_echo_test(external_sink, external_stream, ems_c2_stream, ems_c3_stream).await;

    deployment.stop(&nodes).await.unwrap();
    deployment.cleanup(&nodes).await.unwrap();

    println!("successfully deployed and cleaned up");
}

async fn localhost() {
    telemetry::initialize_tracing_with_filter(
        tracing_subscriber::EnvFilter::try_new("trace,hyper=off").unwrap(),
    );

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

    let external_sink = nodes.connect(external_sink).await;
    let external_stream = nodes.connect(external_stream).await;
    let ems_c2_stream = nodes.connect(ems_c2).await;
    let ems_c3_stream = nodes.connect(ems_c3).await;

    run_echo_test(external_sink, external_stream, ems_c2_stream, ems_c3_stream).await;

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

    let external_sink = nodes.connect(external_sink).await;
    let external_stream = nodes.connect(external_stream).await;
    let ems_c2_stream = nodes.connect(ems_c2).await;
    let ems_c3_stream = nodes.connect(ems_c3).await;

    run_echo_test(external_sink, external_stream, ems_c2_stream, ems_c3_stream).await;

    deployment.stop().await.unwrap();

    println!("successfully deployed and cleaned up");
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum DeployMode {
    #[cfg(feature = "docker")]
    Docker,
    #[cfg(feature = "ecs")]
    Ecs,
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
        #[cfg(feature = "docker")]
        DeployMode::Docker => docker().await,
        #[cfg(feature = "ecs")]
        DeployMode::Ecs => ecs().await,
        DeployMode::Aws => aws().await,
        DeployMode::Localhost => localhost().await,
    }
}

#[test]
fn test_distributed_echo_example_localhost() {
    use example_test::run_current_example;

    let mut run = run_current_example!("--mode localhost");
    run.read_string("successfully deployed and cleaned up");
}

#[cfg(feature = "docker")]
#[test]
fn test_distributed_echo_example_docker() {
    use example_test::run_current_example;

    let mut run = run_current_example!("--mode docker");
    run.read_string("successfully deployed and cleaned up");
}

#[cfg(feature = "ecs")]
#[test]
fn test_distributed_echo_example_ecs() {
    use example_test::run_current_example;

    // Because of no credentials this test is expected to fail, however we can still get a decent amount of coverage by asserting it fails with that error specifically.
    let mut run = run_current_example!("--mode ecs");
    run.read_string("provision: provision: close"); // this is printed while trying to upload the image to ecr, even if that attempt then fails.
}
