use clap::Parser;
use futures::{SinkExt, StreamExt};
use hydro_deploy::{AwsNetwork, Deployment};
use hydro_lang::prelude::FlowBuilder;
use hydro_lang::telemetry;
use hydro_test::distributed::distributed_echo::distributed_echo;

const CLUSTER_SIZE: usize = 2;

/// Common test logic: send test messages, verify responses
/// Now works with raw bytes streams and handles JSON serialization/deserialization
async fn run_echo_test<S, R>(mut external: (R, S))
where
    S: futures::Sink<bytes::Bytes> + Unpin,
    S::Error: std::fmt::Debug,
    R: StreamExt<Item = Result<bytes::BytesMut, std::io::Error>> + Unpin,
{
    let (ref mut stream, ref mut sink) = external;

    // Helper to send a JSON-encoded u32
    async fn send_json<S>(sink: &mut S, value: u32)
    where
        S: futures::Sink<bytes::Bytes> + Unpin,
        S::Error: std::fmt::Debug,
    {
        let json = serde_json::to_string(&value).unwrap();
        sink.send(bytes::Bytes::from(json)).await.unwrap();
    }

    // Helper to receive and parse JSON response (now just a u32)
    async fn recv_response<R>(stream: &mut R) -> u32
    where
        R: StreamExt<Item = Result<bytes::BytesMut, std::io::Error>> + Unpin,
    {
        let raw_bytes = stream.next().await.unwrap().unwrap();
        let json_str = String::from_utf8_lossy(&raw_bytes);
        serde_json::from_str(&json_str).unwrap()
    }

    // Send test messages and verify echo responses
    // The echo chain adds 4 to each value:
    // external -> P1 (+1) -> C2 (+1) -> C3 (+1) -> P4 (+1) -> back to P1 -> external

    println!("sending 0");
    send_json(sink, 0).await;
    let response = recv_response(stream).await;
    assert_eq!(response, 4);

    println!("sending 1");
    send_json(sink, 1).await;
    let response = recv_response(stream).await;
    assert_eq!(response, 5);

    send_json(sink, 2).await;
    let response = recv_response(stream).await;
    assert_eq!(response, 6);

    send_json(sink, 3).await;
    let response = recv_response(stream).await;
    assert_eq!(response, 7);
}

#[cfg(feature = "docker")]
async fn docker() {
    use hydro_lang::deploy::{DockerDeploy, DockerNetwork};

    telemetry::initialize_tracing_with_filter(
        tracing_subscriber::EnvFilter::try_new("trace,hyper=off").unwrap(),
    );

    let network = DockerNetwork::new("distributed_echo_test".to_string());
    let mut deployment = DockerDeploy::new(network);

    let mut builder = FlowBuilder::new();
    let external = builder.external();
    let p1 = builder.process();
    let c2 = builder.cluster();
    let c3 = builder.cluster();
    let p4 = builder.process();
    let bidi_port = distributed_echo(&external, &p1, &c2, &c3, &p4);

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
        .with_external(&external, deployment.add_external("external".to_string()))
        .deploy(&mut deployment);

    deployment.provision(&nodes).await.unwrap();
    deployment.start(&nodes).await.unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(2000)).await; // TOOD: hack to get around some timing issues, will fix this shortly.

    let external_conn = nodes.connect(bidi_port).await;

    run_echo_test(external_conn).await;

    deployment.stop(&nodes).await.unwrap();
    deployment.cleanup(&nodes).await.unwrap();

    println!("successfully deployed and cleaned up");
}

async fn localhost() {
    telemetry::initialize_tracing_with_filter(
        tracing_subscriber::EnvFilter::try_new("trace,hyper=off").unwrap(),
    );

    let mut deployment = Deployment::new();

    let mut builder = FlowBuilder::new();
    let external = builder.external();
    let p1 = builder.process();
    let c2 = builder.cluster();
    let c3 = builder.cluster();
    let p4 = builder.process();
    let bidi_port = distributed_echo(&external, &p1, &c2, &c3, &p4);

    let nodes = builder
        .with_process(&p1, deployment.Localhost())
        .with_cluster(&c2, (0..CLUSTER_SIZE).map(|_| deployment.Localhost()))
        .with_cluster(&c3, (0..CLUSTER_SIZE).map(|_| deployment.Localhost()))
        .with_process(&p4, deployment.Localhost())
        .with_external(&external, deployment.Localhost())
        .deploy(&mut deployment);

    deployment.deploy().await.unwrap();
    deployment.start().await.unwrap();

    let external_conn = nodes.connect(bidi_port).await;

    run_echo_test(external_conn).await;

    deployment.stop().await.unwrap();

    println!("successfully deployed and cleaned up");
}

async fn aws() {
    let mut deployment: Deployment = Deployment::new();

    let mut builder = FlowBuilder::new();
    let external = builder.external();
    let p1 = builder.process();
    let c2 = builder.cluster();
    let c3 = builder.cluster();
    let p4 = builder.process();
    let bidi_port = distributed_echo(&external, &p1, &c2, &c3, &p4);

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
        .with_external(&external, deployment.Localhost())
        .deploy(&mut deployment);

    deployment.deploy().await.unwrap();

    let external_conn = nodes.connect(bidi_port).await;

    run_echo_test(external_conn).await;

    deployment.stop().await.unwrap();

    println!("successfully deployed and cleaned up");
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum DeployMode {
    #[cfg(feature = "docker")]
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
        #[cfg(feature = "docker")]
        DeployMode::Docker => docker().await,
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
