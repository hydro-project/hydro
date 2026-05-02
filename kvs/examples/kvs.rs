#![allow(clippy::type_complexity)]

use std::path::PathBuf;

use clap::Parser;
use hydro_lang::deploy::{DockerDeploy, DockerNetwork, EcsDeploy};
use hydro_lang::location::external_process::{ExternalBytesPort, Many};
use hydro_lang::prelude::*;
use kvs::testing::{run_kvs_test, send_and_check_get, send_and_check_put, send_recv};
use kvs::{KvsCommand, KvsNode, KvsResponse, KvsRouter, REPLICATION_FACTOR};

const CLUSTER_SIZE: usize = 6;
const ROUTER_COUNT: usize = 3;

/// Set up the distributed KVS on the router cluster. Each router binary
/// hosts two ingresses via `bidi_external_sidecar` — one tonic gRPC
/// server (first returned port) and one tokio-tungstenite WebSocket
/// server (second returned port) — both feeding the same `distributed_kvs`
/// pipeline. The returned `ExternalBytesPort`s let the caller discover
/// each member's host-visible TCP endpoint per protocol via
/// `DeployResult::get_all_tcp_endpoints`.
fn setup_kvs<'a>(
    external: &External<'a, ()>,
    routers: &Cluster<'a, KvsRouter>,
    nodes: &Cluster<'a, KvsNode>,
) -> (ExternalBytesPort<Many>, ExternalBytesPort<Many>) {
    kvs::complete_distributed_kvs::<REPLICATION_FACTOR, ()>(external, routers, nodes)
}

/// Remove all docker containers on networks matching the given prefix, then remove those networks.
#[cfg(test)]
fn docker_cleanup(network_prefix: &str) {
    use std::process::Command;
    let output = Command::new("docker")
        .args([
            "network",
            "ls",
            "--filter",
            &format!("name={network_prefix}"),
            "--format",
            "{{.Name}}",
        ])
        .output();
    if let Ok(output) = output {
        let networks = String::from_utf8_lossy(&output.stdout);
        for net in networks.lines().filter(|l| !l.is_empty()) {
            // Remove containers still connected to the network
            let containers = Command::new("docker")
                .args([
                    "network",
                    "inspect",
                    net,
                    "--format",
                    "{{range .Containers}}{{.Name}} {{end}}",
                ])
                .output();
            if let Ok(containers) = containers {
                let names: Vec<String> = String::from_utf8_lossy(&containers.stdout)
                    .split_whitespace()
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
                    .collect();
                if !names.is_empty() {
                    let mut args = vec!["rm".to_string(), "-f".to_string()];
                    args.extend(names);
                    let _ = Command::new("docker").args(&args).output();
                }
            }
            let _ = Command::new("docker").args(["network", "rm", net]).output();
        }
    }
    // Also remove any exited containers with the hy- prefix (e.g. nodes killed during rebalance tests)
    let output = Command::new("docker")
        .args([
            "ps",
            "-a",
            "--filter",
            "status=exited",
            "--filter",
            "name=hy-",
            "--format",
            "{{.Names}}",
        ])
        .output();
    if let Ok(output) = output {
        let names: Vec<String> = String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();
        if !names.is_empty() {
            let mut args = vec!["rm".to_string(), "-f".to_string()];
            args.extend(names);
            let _ = Command::new("docker").args(&args).output();
        }
    }
}

async fn docker_deploy(
    network_name: &str,
) -> (
    DockerDeploy,
    hydro_lang::compile::deploy::DeployResult<'_, DockerDeploy>,
    Vec<(String, u16)>,
    Vec<(String, u16)>,
) {
    let network = DockerNetwork::new(network_name.to_owned());
    let mut deployment = DockerDeploy::new(network)
        .with_env(vec!["RUST_LOG=info".to_owned(), "NO_COLOR=1".to_owned()]);
    let config = vec![r#"profile.dev.strip="symbols""#.to_owned()];
    let mut builder = FlowBuilder::new();
    let external = builder.external::<()>();
    let routers = builder.cluster();
    let nodes = builder.cluster();
    let (grpc_port, ws_port) = setup_kvs(&external, &routers, &nodes);
    let nodes_deploy = builder
        .with_cluster(
            &routers,
            deployment.add_localhost_docker_cluster(None, config.clone(), ROUTER_COUNT),
        )
        .with_cluster(
            &nodes,
            deployment.add_localhost_docker_cluster(None, config.clone(), CLUSTER_SIZE),
        )
        .with_external(&external, deployment.add_external("kvs-clients".to_owned()))
        .deploy(&mut deployment);
    println!("Provisioning...");
    deployment.provision(&nodes_deploy).await.unwrap();
    println!("Starting...");
    deployment.start(&nodes_deploy).await.unwrap();
    println!("Waiting for cluster to initialize...");
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    let grpc_endpoints = nodes_deploy.get_all_tcp_endpoints(grpc_port).await;
    let ws_endpoints = nodes_deploy.get_all_tcp_endpoints(ws_port).await;
    println!("Router gRPC endpoints: {grpc_endpoints:?}");
    println!("Router WebSocket endpoints: {ws_endpoints:?}");
    assert_eq!(
        grpc_endpoints.len(),
        ROUTER_COUNT,
        "expected {ROUTER_COUNT} router gRPC endpoints"
    );
    assert_eq!(
        ws_endpoints.len(),
        ROUTER_COUNT,
        "expected {ROUTER_COUNT} router WebSocket endpoints"
    );

    (deployment, nodes_deploy, grpc_endpoints, ws_endpoints)
}

async fn docker_run() {
    let (_deployment, _nodes_deploy, grpc_endpoints, ws_endpoints) = docker_deploy("kvs_run").await;
    println!("\nKVS cluster is running. Endpoints:");
    for (i, (h, p)) in grpc_endpoints.iter().enumerate() {
        println!("  Router {i} (gRPC):      http://{h}:{p}");
    }
    for (i, (h, p)) in ws_endpoints.iter().enumerate() {
        println!("  Router {i} (WebSocket): ws://{h}:{p}");
    }
    let endpoint_strs: Vec<String> = grpc_endpoints
        .iter()
        .map(|(h, p)| format!("http://{h}:{p}"))
        .collect();
    println!("\nFor load testing:");
    println!(
        "  cargo run --example loadtest -- {}",
        endpoint_strs.join(" ")
    );
    println!("\nTo stop: docker rm -f $(docker ps -aq --filter name=hy-)");
}

async fn docker_e2e_test() {
    let (deployment, nodes_deploy, grpc_endpoints, ws_endpoints) = docker_deploy("kvs_basic").await;

    let (host, port) = &grpc_endpoints[0];
    let addr = format!("{host}:{port}");
    println!("Running gRPC test suite on {addr}");
    run_kvs_test(&addr, CLUSTER_SIZE).await;

    for (i, (host, port)) in grpc_endpoints.iter().enumerate() {
        let addr = format!("{host}:{port}");
        println!("Testing router {i} via gRPC at {addr}");
        send_and_check_put(&addr, "router_test", &format!("from_router_{i}")).await;
        send_and_check_get(&addr, "router_test").await;
        println!("Router {i} gRPC OK");
    }
    println!("All {ROUTER_COUNT} routers verified on gRPC ingress");

    // Exercise the WebSocket ingress on every router. Writes go via WS,
    // reads go back via WS *and* gRPC — values written through WS
    // must be visible through both the WS and gRPC ingresses since
    // both feed the same dataflow.
    use kvs::testing::{new_trace_id, send_recv_ws};
    use kvs::{KvsCommand, KvsResponse};
    for (i, (host, port)) in ws_endpoints.iter().enumerate() {
        let ws_addr = format!("{host}:{port}");
        let grpc_addr = format!("{}:{}", grpc_endpoints[i].0, grpc_endpoints[i].1);
        let key = format!("ws_test_{i}");
        let value = format!("from_ws_{i}");
        println!("Testing router {i} via WebSocket at {ws_addr}");

        // Write via WS.
        let put_resp = send_recv_ws(
            &ws_addr,
            &KvsCommand::Put {
                trace_id: new_trace_id(),
                key: key.clone(),
                value: value.clone(),
            },
        )
        .await;
        assert!(
            matches!(&put_resp, KvsResponse::PutOk { key: k, .. } if k == &key),
            "expected PutOk, got {put_resp:?}"
        );

        // Read back via WS.
        let get_resp = send_recv_ws(
            &ws_addr,
            &KvsCommand::Get {
                trace_id: new_trace_id(),
                key: key.clone(),
            },
        )
        .await;
        match &get_resp {
            KvsResponse::GetResult {
                value: Some(vs), ..
            } if vs.contains(&value) => {}
            other => panic!("WS read-back failed for {key}: {other:?}"),
        }

        // Cross-protocol read: value written via WS must also be visible
        // via the gRPC ingress.
        send_and_check_get(&grpc_addr, &key).await;
        println!("Router {i} WebSocket OK (read back via both WS and gRPC)");
    }
    println!("All {ROUTER_COUNT} routers verified on WebSocket ingress");

    // Final sentinel consumed by `test_kvs_docker` to know the full
    // gRPC+WS suite finished.
    println!("docker e2e test completed successfully");

    // nodes_deploy is type-erased, need to stop/cleanup via deployment
    // The deployment owns the lifecycle
    drop(nodes_deploy);
    drop(deployment);
}

async fn export_manifest(output_path: PathBuf) {
    let mut deployment = EcsDeploy::new();
    let mut builder = FlowBuilder::new();
    let external = builder.external::<()>();
    let routers = builder.cluster();
    let nodes = builder.cluster();
    let _ports = setup_kvs(&external, &routers, &nodes);
    let nodes_deploy = builder
        .with_cluster(&routers, deployment.add_ecs_cluster(ROUTER_COUNT))
        .with_cluster(&nodes, deployment.add_ecs_cluster(CLUSTER_SIZE))
        .with_external(
            &external,
            deployment.add_external("kvs-clients".to_string()),
        )
        .deploy(&mut deployment);
    let manifest = deployment.export(&nodes_deploy);
    tokio::fs::create_dir_all(&output_path).await.unwrap();
    let manifest_path = output_path.join("hydro-manifest.json");
    tokio::fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .await
    .unwrap();
    println!("CDK export complete!");
}

async fn docker_rebalance() {
    let network = DockerNetwork::new("kvs_rebal".to_owned());
    let mut deployment = DockerDeploy::new(network)
        .with_env(vec!["RUST_LOG=info".to_owned(), "NO_COLOR=1".to_owned()]);
    let config = vec![r#"profile.dev.strip="symbols""#.to_owned()];
    let mut builder = FlowBuilder::new();
    let external = builder.external::<()>();
    let routers = builder.cluster();
    let nodes = builder.cluster();
    let (grpc_port, _ws_port) = setup_kvs(&external, &routers, &nodes);
    let nodes_deploy = builder
        .with_cluster(
            &routers,
            deployment.add_localhost_docker_cluster(None, config.clone(), ROUTER_COUNT),
        )
        .with_cluster(
            &nodes,
            deployment.add_localhost_docker_cluster(None, config.clone(), CLUSTER_SIZE),
        )
        .with_external(&external, deployment.add_external("kvs-clients".to_owned()))
        .deploy(&mut deployment);
    deployment.provision(&nodes_deploy).await.unwrap();
    deployment.start(&nodes_deploy).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    let endpoints = nodes_deploy.get_all_tcp_endpoints(grpc_port).await;
    let (host, port) = endpoints
        .first()
        .cloned()
        .unwrap_or(("127.0.0.1".to_string(), 10001));

    // Phase 1: Put keys with all nodes alive
    println!("=== Phase 1: Putting keys with all {CLUSTER_SIZE} nodes ===");
    let num_keys = 50;
    let addr = format!("{host}:{port}");
    for i in 0..num_keys {
        send_and_check_put(&addr, &format!("rebal_key_{i}"), &format!("val_{i}")).await;
    }
    println!("Put {num_keys} keys successfully");

    // Discover node container names
    let node_containers: Vec<String> = {
        let output = std::process::Command::new("docker")
            .args(["ps", "--format", "{{.Names}}", "--filter", "status=running"])
            .output()
            .unwrap();
        let all: Vec<String> = String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .filter(|n| !n.is_empty())
            .map(String::from)
            .collect();
        let mut groups: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        for name in &all {
            if let Some(prefix) = name.rsplit_once('-').map(|(p, _)| p.to_string()) {
                groups.entry(prefix).or_default().push(name.clone());
            }
        }
        #[allow(clippy::disallowed_methods)] // searching groups; order irrelevant
        let (_, names) = groups
            .iter()
            .find(|(_, v)| v.len() == CLUSTER_SIZE)
            .expect("could not find node container group");
        names.clone()
    };
    println!("Node containers: {node_containers:?}");

    // Kill nodes one at a time, keeping at least REPLICATION_FACTOR alive so
    // every key still has a quorum of replicas for the Get verification.
    let nodes_to_kill = CLUSTER_SIZE - REPLICATION_FACTOR;
    #[allow(clippy::needless_range_loop)] // kill_round used for both indexing and display
    for kill_round in 0..nodes_to_kill {
        let alive = CLUSTER_SIZE - kill_round;
        let target = &node_containers[kill_round];
        println!(
            "\n=== Round {}: Killing {target} ({alive} -> {} nodes) ===",
            kill_round + 1,
            alive - 1
        );
        let status = std::process::Command::new("docker")
            .args(["kill", target])
            .status()
            .unwrap();
        assert!(status.success(), "failed to kill {target}");

        println!("Waiting for membership change and rebalancing...");
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

        // Verify every key returns the correct value
        let remaining = alive - 1;
        println!("Verifying all {num_keys} keys with {remaining} nodes alive...");
        for i in 0..num_keys {
            let key = format!("rebal_key_{i}");
            let resp = send_recv(
                &addr,
                &KvsCommand::Get {
                    trace_id: String::new(),
                    key: key.clone(),
                },
            )
            .await;
            match &resp {
                KvsResponse::GetResult {
                    key: k,
                    value: Some(v),
                    node_ids,
                    ..
                } => {
                    assert_eq!(k, &key);
                    assert!(
                        v.contains(&format!("val_{i}")),
                        "round {}: key {key}: expected val_{i} in {v:?}",
                        kill_round + 1
                    );
                    assert_eq!(
                        node_ids.len(),
                        REPLICATION_FACTOR,
                        "round {}: key {key}: expected {REPLICATION_FACTOR} node_ids, got {node_ids:?}",
                        kill_round + 1
                    );
                }
                other => panic!(
                    "round {}: key {key}: expected GetResult with value, got {other:?}",
                    kill_round + 1
                ),
            }
        }
        println!("All {num_keys} keys correct with {remaining} nodes alive ✓");
    }

    println!("\nrebalance test completed successfully");

    // Clean up: force-remove all containers (some may already be stopped)
    let _ = std::process::Command::new("sh")
        .args(["-c", "docker rm -f $(docker ps -aq) 2>/dev/null"])
        .output();
    let _ = std::process::Command::new("sh")
        .args(["-c", "docker network prune -f 2>/dev/null"])
        .output();
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum DeployMode {
    /// Start the KVS cluster locally in Docker and keep it running
    Docker,
    /// Start the cluster, run the E2E test suite, then tear down
    DockerE2eTest,
    /// Start the cluster, run the rebalance test, then tear down
    DockerRebalance,
    /// Export deployment manifest (binary names, ports, service naming)
    Export,
}

#[derive(Parser, Debug)]
struct Args {
    #[clap(long, value_enum)]
    mode: DeployMode,
    #[clap(long, default_value = "./hydro-assets")]
    output: PathBuf,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    match args.mode {
        DeployMode::Docker => docker_run().await,
        DeployMode::DockerE2eTest => docker_e2e_test().await,
        DeployMode::DockerRebalance => docker_rebalance().await,
        DeployMode::Export => export_manifest(args.output).await,
    }
}

#[test]
fn test_kvs_docker() {
    use std::io::Read;
    use std::process::{Command, Stdio};
    docker_cleanup("kvs_basic");

    let mut child = Command::new("cargo")
        .args([
            "run",
            "--frozen",
            "-p",
            env!("CARGO_PKG_NAME"),
            "--example",
            "kvs",
            "--",
            "--mode",
            "docker-e2e-test",
        ])
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to spawn");
    let stdout = child.stdout.as_mut().unwrap();
    let mut output = Vec::new();
    loop {
        let mut buf = [0u8; 1024];
        let n = stdout.read(&mut buf).expect("read failed");
        if n == 0 {
            panic!("exited early.\n{}", String::from_utf8_lossy(&output));
        }
        output.extend_from_slice(&buf[..n]);
        if String::from_utf8_lossy(&output).contains("docker e2e test completed successfully") {
            break;
        }
    }
    child.kill().ok();
    child.wait().ok();
    docker_cleanup("kvs_basic");
}

#[test]
fn test_kvs_rebalance_docker() {
    use std::io::Read;
    use std::process::{Command, Stdio};
    docker_cleanup("kvs_rebal");

    let mut child = Command::new("cargo")
        .args([
            "run",
            "--frozen",
            "-p",
            env!("CARGO_PKG_NAME"),
            "--example",
            "kvs",
            "--",
            "--mode",
            "docker-rebalance",
        ])
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to spawn");
    let stdout = child.stdout.as_mut().unwrap();
    let mut output = Vec::new();
    loop {
        let mut buf = [0u8; 1024];
        let n = stdout.read(&mut buf).expect("read failed");
        if n == 0 {
            panic!("exited early.\n{}", String::from_utf8_lossy(&output));
        }
        output.extend_from_slice(&buf[..n]);
        let text = String::from_utf8_lossy(&output);
        if text.contains("rebalance test completed successfully") {
            break;
        }
    }
    child.kill().ok();
    child.wait().ok();
    docker_cleanup("kvs_rebal");
}
