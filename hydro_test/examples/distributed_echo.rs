use std::path::PathBuf;

use clap::Parser;
use futures::{SinkExt, StreamExt};
use hydro_deploy::{AwsNetwork, Deployment};
use hydro_lang::deploy::TrybuildHost;
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

#[cfg(feature = "test_docker")]
async fn docker() {
    use hydro_lang::deploy::{DockerDeploy, DockerNetwork, LinuxCompileType};

    telemetry::initialize_tracing_with_filter(
        tracing_subscriber::EnvFilter::try_new("trace,hyper=off").unwrap(),
    );

    let network = DockerNetwork::new("distributed_echo_test".to_owned());
    let mut deployment = DockerDeploy::new(network);

    let mut builder = FlowBuilder::new();
    let external = builder.external();
    let p1 = builder.process();
    let c2 = builder.cluster();
    let c3 = builder.cluster();
    let p4 = builder.process();
    let bidi_port = distributed_echo(&external, &p1, &c2, &c3, &p4);

    let config = vec![r#"profile.dev.strip="symbols""#.to_owned()];

    let nodes = builder
        .with_process(
            &p1,
            deployment
                .add_localhost_docker(None, config.clone())
                .features(["tokio"]),
        )
        .with_cluster(
            &c2,
            deployment
                .add_localhost_docker_cluster(None, config.clone(), CLUSTER_SIZE)
                .features(["tokio"]),
        )
        .with_cluster(
            &c3,
            deployment
                .add_localhost_docker_cluster(None, config.clone(), CLUSTER_SIZE)
                .features(["tokio"]),
        )
        .with_process(
            &p4,
            deployment
                .add_localhost_docker(None, config.clone())
                .base_image("debian:bookworm-slim")
                .linux_compile_type(LinuxCompileType::Glibc)
                .features(["tokio"]),
        )
        .with_external(&external, deployment.add_external("external".to_owned()))
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
        .with_process(
            &p1,
            TrybuildHost::new(deployment.Localhost()).features(["tokio"]),
        )
        .with_cluster(
            &c2,
            (0..CLUSTER_SIZE)
                .map(|_| TrybuildHost::new(deployment.Localhost()).features(["tokio"])),
        )
        .with_cluster(
            &c3,
            (0..CLUSTER_SIZE)
                .map(|_| TrybuildHost::new(deployment.Localhost()).features(["tokio"])),
        )
        .with_process(
            &p4,
            TrybuildHost::new(deployment.Localhost()).features(["tokio"]),
        )
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

#[cfg(feature = "test_ecs")]
async fn export_manifest(output_path: PathBuf) {
    use hydro_lang::deploy::EcsDeploy;

    telemetry::initialize_tracing_with_filter(tracing_subscriber::EnvFilter::try_new(
        "info,hyper=warn,aws_smithy_runtime=info,aws_sdk_ecs=info,aws_sigv4=info,aws_config=info,aws_runtime=info,aws_smithy_http_client=info,aws_sdk_ec2=info,aws_sdk_ecr=info,h2=warn",
    ).unwrap());

    let mut deployment = EcsDeploy::new();

    let mut builder = FlowBuilder::new();
    let external = builder.external();
    let p1 = builder.process();
    let c2 = builder.cluster();
    let c3 = builder.cluster();
    let p4 = builder.process();
    let _bidi_port = distributed_echo(&external, &p1, &c2, &c3, &p4);

    let nodes = builder
        .with_process(&p1, deployment.add_ecs_process())
        .with_cluster(&c2, deployment.add_ecs_cluster(CLUSTER_SIZE))
        .with_cluster(&c3, deployment.add_ecs_cluster(CLUSTER_SIZE))
        .with_process(&p4, deployment.add_ecs_process())
        .with_external(&external, deployment.add_external("external".to_owned()))
        .deploy(&mut deployment);

    let manifest = deployment.export(&nodes);

    tokio::fs::create_dir_all(&output_path).await.unwrap();
    let manifest_path = output_path.join("hydro-manifest.json");
    let manifest_json = serde_json::to_string_pretty(&manifest).unwrap();
    tokio::fs::write(&manifest_path, &manifest_json)
        .await
        .unwrap();

    println!("Export complete!");
    println!("Manifest written to: {}", manifest_path.display());

    // Build each trybuild binary to verify the generated code compiles.
    for build in manifest
        .processes
        .values()
        .map(|p| &p.build)
        .chain(manifest.clusters.values().map(|c| &c.build))
    {
        println!("Building trybuild binary: {}", build.bin_name);
        let status = std::process::Command::new("cargo")
            .args(["build", "--locked", "--example", &build.bin_name])
            .args(["--target-dir", &build.target_dir])
            .args(["--features", &build.features.join(",")])
            .args([
                "--manifest-path",
                &format!("{}/Cargo.toml", build.project_dir),
            ])
            .env("STAGELEFT_TRYBUILD_BUILD_STAGED", "1")
            .status()
            .expect("failed to invoke cargo build");
        assert!(
            status.success(),
            "cargo build failed for {}",
            build.bin_name
        );
    }

    println!("All trybuild binaries compiled successfully.");
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum DeployMode {
    #[cfg(feature = "test_docker")]
    Docker,
    Localhost,
    Aws,
    #[cfg(feature = "test_ecs")]
    Export,
}

#[derive(Parser, Debug)]
struct Args {
    #[clap(long, value_enum)]
    mode: DeployMode,

    /// Output directory for export (only used with --mode export)
    #[clap(long, default_value = "./hydro-assets")]
    output: PathBuf,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    match args.mode {
        #[cfg(feature = "test_docker")]
        DeployMode::Docker => docker().await,
        DeployMode::Aws => aws().await,
        DeployMode::Localhost => localhost().await,
        #[cfg(feature = "test_ecs")]
        DeployMode::Export => export_manifest(args.output).await,
    }
}

#[cfg(target_os = "linux")]
#[test]
fn test_distributed_echo_example_localhost() {
    use example_test::run_current_example;

    let mut run = run_current_example!("--mode localhost");
    run.read_string("successfully deployed and cleaned up");
}

#[cfg(all(target_os = "linux", feature = "test_docker"))]
#[test]
fn test_distributed_echo_example_docker() {
    use example_test::run_current_example;

    let mut run = run_current_example!("--mode docker");
    run.read_string("successfully deployed and cleaned up");
}

#[cfg(all(target_os = "linux", feature = "test_ecs"))]
#[test]
fn test_distributed_echo_example_export() {
    use example_test::run_current_example;

    let mut run = run_current_example!("--mode export --output /tmp/hydro-test-export");
    run.read_string("All trybuild binaries compiled successfully.");
    let _ = std::fs::remove_dir_all("/tmp/hydro-test-export");
}
