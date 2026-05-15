//! Demonstrates `sidecar_bidi` on both a Process and a Cluster.
//!
//! Each sidecar accepts a TCP connection, reads length-delimited string frames,
//! forwards them into the dataflow via the mpsc channel, and writes back
//! transformed strings received from the dataflow.
//!
//! The Docker mode additionally demonstrates connecting to multiple cluster
//! instances and verifying each one responds independently.

use clap::Parser;
use futures::{SinkExt, StreamExt};
use hydro_lang::prelude::*;
use hydro_lang::telemetry;
use tokio_util::codec::LengthDelimitedCodec;

struct SidecarProcess;
struct SidecarCluster;

#[derive(clap::ValueEnum, Clone, Debug)]
enum DeployMode {
    Localhost,
    #[cfg(feature = "test_docker")]
    Docker,
}

#[derive(Parser, Debug)]
struct Args {
    #[clap(long, value_enum)]
    mode: DeployMode,
}

/// Connect to a sidecar, send a message, and verify the response.
async fn send_and_verify(
    host: &str,
    port: u16,
    msg: &str,
    expected: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let stream = tokio::net::TcpStream::connect(format!("{}:{}", host, port)).await?;
    let (reader, writer) = stream.into_split();
    let mut framed_write = tokio_util::codec::FramedWrite::new(writer, LengthDelimitedCodec::new());
    let mut framed_read = tokio_util::codec::FramedRead::new(reader, LengthDelimitedCodec::new());

    framed_write
        .send(msg.to_owned().into_bytes().into())
        .await?;

    if let Some(Ok(frame)) = framed_read.next().await {
        let resp = String::from_utf8(frame.to_vec())?;
        assert_eq!(
            resp, expected,
            "sent '{}', expected '{}', got '{}'",
            msg, expected, resp
        );
        println!("  sent '{}' -> got '{}' ✓", msg, resp);
    } else {
        panic!("no response received for '{}'", msg);
    }

    Ok(())
}

async fn localhost() {
    use hydro_deploy::Deployment;

    telemetry::initialize_tracing_with_filter(
        tracing_subscriber::EnvFilter::try_new("info").unwrap(),
    );

    let mut deployment = Deployment::new();

    let mut flow = FlowBuilder::new();
    let process = flow.process::<SidecarProcess>();
    let cluster = flow.cluster::<SidecarCluster>();

    // Process sidecar on port 9000: echoes with "process: " prefix
    let (proc_inbound, proc_response_handle) =
        process.sidecar_bidi::<String, String, _>(q!(|| {
            hydro_test::external_client::sidecar_demo::create(9000)
        }));
    let proc_responses = proc_inbound.map(q!(|msg: String| format!("process: {}", msg)));
    proc_response_handle.complete(proc_responses);

    // Cluster sidecar on port 9001: echoes with "cluster: " prefix
    let (cluster_inbound, cluster_response_handle) =
        cluster.sidecar_bidi::<String, String, _>(q!(|| {
            hydro_test::external_client::sidecar_demo::create(9001)
        }));
    let cluster_responses = cluster_inbound.map(q!(|msg: String| format!("cluster: {}", msg)));
    cluster_response_handle.complete(cluster_responses);

    let built = flow.finalize();

    let _nodes = built
        .with_default_optimize()
        .with_process(&process, deployment.Localhost())
        .with_cluster(&cluster, (0..1).map(|_| deployment.Localhost()))
        .deploy(&mut deployment);

    deployment.deploy().await.unwrap();
    deployment.start().await.unwrap();

    // Give sidecars time to start listening
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // Test process sidecar
    println!("Testing process sidecar (port 9000):");
    send_and_verify("127.0.0.1", 9000, "hello", "process: hello")
        .await
        .unwrap();

    // Test cluster sidecar
    println!("Testing cluster sidecar (port 9001):");
    send_and_verify("127.0.0.1", 9001, "world", "cluster: world")
        .await
        .unwrap();

    deployment.stop().await.unwrap();

    println!("SUCCESS: round-trip through both sidecars works");
}

#[cfg(feature = "test_docker")]
async fn docker() {
    use hydro_lang::deploy::{DockerDeploy, DockerNetwork};

    telemetry::initialize_tracing_with_filter(
        tracing_subscriber::EnvFilter::try_new("info").unwrap(),
    );

    let network = DockerNetwork::new("sidecar_bidi_test".to_owned());
    let mut deployment = DockerDeploy::new(network);

    let mut flow = FlowBuilder::new();
    let process = flow.process::<SidecarProcess>();
    let cluster = flow.cluster::<SidecarCluster>();

    // Process sidecar on port 9000: echoes with "process: " prefix
    let (proc_inbound, proc_response_handle) =
        process.sidecar_bidi::<String, String, _>(q!(|| {
            hydro_test::external_client::sidecar_demo::create(9000)
        }));
    let proc_responses = proc_inbound.map(q!(|msg: String| format!("process: {}", msg)));
    proc_response_handle.complete(proc_responses);

    // Cluster sidecar on port 9000: echoes with "cluster: " prefix
    // Each cluster instance is a separate container, so port 9000 doesn't conflict.
    let (cluster_inbound, cluster_response_handle) =
        cluster.sidecar_bidi::<String, String, _>(q!(|| {
            hydro_test::external_client::sidecar_demo::create(9000)
        }));
    let cluster_responses = cluster_inbound.map(q!(|msg: String| format!("cluster: {}", msg)));
    cluster_response_handle.complete(cluster_responses);

    let built = flow.finalize();

    let config = vec![r#"profile.dev.strip="symbols""#.to_owned()];

    let nodes = built
        .with_default_optimize()
        .with_process(
            &process,
            deployment.add_localhost_docker(None, config.clone()),
        )
        .with_cluster(
            &cluster,
            deployment.add_localhost_docker_cluster(None, config, 2),
        )
        .deploy(&mut deployment);

    // Expose port 9000 on both the process and the cluster
    let process_node = nodes.get_process(&process);
    process_node.expose_port(9000);

    let cluster_node = nodes.get_cluster(&cluster);
    cluster_node.expose_port(9000);

    deployment.provision(&nodes).await.unwrap();
    deployment.start(&nodes).await.unwrap();

    // Give sidecars time to start listening
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // Test process sidecar
    let (proc_host, proc_port) = process_node.get_tcp_endpoint(9000).await;
    println!("Process sidecar listening on: {}:{}", proc_host, proc_port);
    println!("Testing process sidecar:");
    send_and_verify(&proc_host, proc_port, "hello", "process: hello")
        .await
        .unwrap();

    // Test cluster sidecar — connect to each instance independently
    let cluster_endpoints = cluster_node.get_all_tcp_endpoints(9000).await;
    println!("Cluster sidecar endpoints: {:?}", cluster_endpoints);

    for (i, (host, port)) in cluster_endpoints.iter().enumerate() {
        println!("Testing cluster sidecar (instance {}):", i);
        let msg = format!("instance-{}", i);
        let expected = format!("cluster: instance-{}", i);
        send_and_verify(host, *port, &msg, &expected).await.unwrap();
    }

    deployment.stop(&nodes).await.unwrap();
    deployment.cleanup(&nodes).await.unwrap();

    println!("SUCCESS: round-trip through both sidecars works");
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    match args.mode {
        DeployMode::Localhost => localhost().await,
        #[cfg(feature = "test_docker")]
        DeployMode::Docker => docker().await,
    }
}

#[cfg(target_os = "linux")]
#[test]
fn test_sidecar_bidi_localhost() {
    use example_test::run_current_example;

    let mut run = run_current_example!("--mode localhost");
    run.read_string("SUCCESS: round-trip through both sidecars works");
}

#[cfg(all(target_os = "linux", feature = "test_docker"))]
#[test]
fn test_sidecar_bidi_docker() {
    use example_test::run_current_example;

    let mut run = run_current_example!("--mode docker");
    run.read_string("SUCCESS: round-trip through both sidecars works");
}
