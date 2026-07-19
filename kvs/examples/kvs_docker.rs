//! Local Docker deployment of the quorum key-value store.
//!
//! Launches the whole KVS — a cluster of 3 router nodes and a cluster of 9
//! storage nodes — as separate Docker containers on the local machine. Each
//! router runs a `sidecar_bidi` gRPC server (see `kvs::sidecar`), so an external
//! client connects **directly to one router** (no gateway process, no
//! external↔cluster networking support needed from the backend). The example
//! then drives a small Put/Put/Get workload through that connection over gRPC.
//!
//! Requires a running Docker daemon:
//!
//! ```bash
//! cargo run -p kvs --example kvs_docker
//! ```

use hydro_lang::deploy::{DockerDeploy, DockerNetwork};
use hydro_lang::prelude::*;
use kvs::sidecar::pb::kv_store_client::KvStoreClient;
use kvs::sidecar::pb::{GetRequest, PutRequest};
use kvs::{RouterNode, StorageNode, QUORUM, REPLICATION_FACTOR, STORAGE_MEMBERS};

/// Standard KVS topology: 3 routers, 9 storage nodes.
const NUM_ROUTERS: usize = 3;

/// TCP port each router's gRPC sidecar listens on inside its container.
const CLIENT_PORT: u16 = 9000;

/// Return the names of this deployment's containers whose name contains
/// `name_part` (e.g. `"routernode"` or `"storagenode"`), scoped to this run via
/// the deployment instance id so containers left over from earlier runs are
/// ignored. Container names follow `hy-{node}-{deployment_instance}-{loc}-{num}`.
fn container_names(deployment_instance: &str, name_part: &str) -> Vec<String> {
    let output = std::process::Command::new("docker")
        .args(["ps", "-a", "--format", "{{.Names}}"])
        .output()
        .expect("run `docker ps`");
    String::from_utf8(output.stdout)
        .expect("docker ps output is utf-8")
        .lines()
        .filter(|name| name.contains(deployment_instance) && name.contains(name_part))
        .map(str::to_owned)
        .collect()
}

/// Fetch a container's logs (both stdout and stderr — the dataflow's `tracing`
/// output goes to stderr) as a single string.
fn container_logs(name: &str) -> String {
    let output = std::process::Command::new("docker")
        .args(["logs", name])
        .output()
        .unwrap_or_else(|e| panic!("run `docker logs {name}`: {e}"));
    let mut logs = String::from_utf8_lossy(&output.stdout).into_owned();
    logs.push_str(&String::from_utf8_lossy(&output.stderr));
    logs
}

/// Count how many of the given containers logged a line containing `needle`.
fn count_containers_logging(names: &[String], needle: &str) -> usize {
    names
        .iter()
        .filter(|name| container_logs(name).contains(needle))
        .count()
}

#[tokio::main]
async fn main() {
    hydro_lang::telemetry::initialize_tracing_with_filter(
        tracing_subscriber::EnvFilter::try_new("info,hyper=off,bollard=off").unwrap(),
    );

    let network = DockerNetwork::new("kvs_local".to_owned());
    let mut deployment = DockerDeploy::new(network);

    let mut builder = FlowBuilder::new();
    let routers = builder.cluster::<RouterNode>();
    let storage = builder.cluster::<StorageNode>();

    kvs::kvs_deploy(&routers, &storage, CLIENT_PORT);

    let built = builder.finalize();

    // Strip symbols to keep the container images small.
    let config = vec![r#"profile.dev.strip="symbols""#.to_owned()];

    let nodes = built
        .with_default_optimize()
        .with_cluster(
            &routers,
            deployment.add_localhost_docker_cluster(None, config.clone(), NUM_ROUTERS),
        )
        .with_cluster(
            &storage,
            deployment.add_localhost_docker_cluster(None, config.clone(), STORAGE_MEMBERS),
        )
        .deploy(&mut deployment);

    // Expose the sidecar port on every router container.
    let router_node = nodes.get_cluster(&routers);
    router_node.expose_port(CLIENT_PORT);

    deployment.provision(&nodes).await.unwrap();
    deployment.start(&nodes).await.unwrap();

    // Give the router sidecars time to bind their gRPC listeners.
    tokio::time::sleep(std::time::Duration::from_millis(2000)).await;

    // Connect a gRPC client directly to router 0's sidecar.
    let endpoints = router_node.get_all_tcp_endpoints(CLIENT_PORT).await;
    let (host, port) = &endpoints[0];
    let url = format!("http://{host}:{port}");
    println!("connecting client to router 0 at {url}");
    let mut client = KvStoreClient::connect(url).await.unwrap();

    // Put hydro=rocks.
    println!("--> Put hydro=rocks");
    client
        .put(PutRequest {
            key: "hydro".to_owned(),
            value: "rocks".to_owned(),
        })
        .await
        .unwrap();
    println!("<-- PutReply");

    // Put a second value under the same key; the store is a grow-only set.
    println!("--> Put hydro=streams");
    client
        .put(PutRequest {
            key: "hydro".to_owned(),
            value: "streams".to_owned(),
        })
        .await
        .unwrap();
    println!("<-- PutReply");

    // Get hydro; should return the union of both values.
    println!("--> Get hydro");
    let reply = client
        .get(GetRequest {
            key: "hydro".to_owned(),
        })
        .await
        .unwrap()
        .into_inner();
    let values: std::collections::HashSet<String> = reply.values.into_iter().collect();
    println!("<-- GetReply = {values:?}");
    assert_eq!(
        values,
        std::collections::HashSet::from(["rocks".to_owned(), "streams".to_owned()])
    );

    println!("workload succeeded");

    // Demonstrate that the dataflow's `tracing` output is captured per container:
    // query each router/storage container's logs and assert the KVS logged the
    // request as it flowed through the system. The `kvs` dataflow emits these via
    // `tracing::info!` inside `.inspect(...)` operators (see `kvs::kvs`), and the
    // deployed container runtime routes that output to the container's logs.
    println!("--- verifying container logs ---");
    let instance = deployment.get_deployment_instance();
    let routers = container_names(&instance, "routernode");
    let storage = container_names(&instance, "storagenode");
    assert_eq!(routers.len(), NUM_ROUTERS, "found routers: {routers:?}");
    assert_eq!(storage.len(), STORAGE_MEMBERS, "found storage: {storage:?}");

    // The sidecar mints internal `req_id`s per RPC starting at 0 (the client
    // never sees them), so our three calls are: put rocks = req_id 0, put streams
    // = req_id 1, get = req_id 2.
    const PUT_ROCKS_ID: u64 = 0;
    const GET_ID: u64 = 2;

    // The client connected to exactly one router (its sidecar owns the only live
    // client connection), so exactly one router logged the request arrivals and
    // the quorum decisions.
    let routers_saw_put = count_containers_logging(
        &routers,
        &format!("kvs router received request: req_id={PUT_ROCKS_ID}"),
    );
    assert_eq!(
        routers_saw_put, 1,
        "exactly one router should have received the put (req_id={PUT_ROCKS_ID})"
    );
    let routers_saw_put_quorum = count_containers_logging(
        &routers,
        &format!("kvs router put quorum reached: req_id={PUT_ROCKS_ID}"),
    );
    assert_eq!(
        routers_saw_put_quorum, 1,
        "exactly one router should have reached put quorum for req_id={PUT_ROCKS_ID}"
    );
    let routers_saw_get_quorum = count_containers_logging(
        &routers,
        &format!("kvs router get quorum reached: req_id={GET_ID}"),
    );
    assert_eq!(
        routers_saw_get_quorum, 1,
        "exactly one router should have reached get quorum for req_id={GET_ID}"
    );
    println!("router logs: request arrival + put/get quorum decisions present");

    // The put of `hydro=rocks` is replicated to REPLICATION_FACTOR storage nodes;
    // a matching number should have logged applying it. The get is served by the
    // same replica set. We assert at least a quorum logged each, since that is
    // what the protocol guarantees observably.
    let storage_applied_put = count_containers_logging(
        &storage,
        // Match the stable tail of the line; the `from_router=...` prefix is a
        // container name we can't predict here.
        &format!("req_id={PUT_ROCKS_ID} key=hydro value=rocks"),
    );
    assert!(
        storage_applied_put >= QUORUM,
        "at least a quorum ({QUORUM}) of storage nodes should have applied the put, \
         got {storage_applied_put}"
    );
    assert!(
        storage_applied_put <= REPLICATION_FACTOR,
        "no more than REPLICATION_FACTOR ({REPLICATION_FACTOR}) storage nodes should have \
         applied the put, got {storage_applied_put}"
    );
    let storage_served_get = count_containers_logging(
        &storage,
        // Match the stable tail; the `from_router=...` prefix is unpredictable.
        &format!("req_id={GET_ID} key=hydro"),
    );
    assert!(
        storage_served_get >= QUORUM,
        "at least a quorum ({QUORUM}) of storage nodes should have served the get, \
         got {storage_served_get}"
    );
    println!(
        "storage logs: {storage_applied_put} nodes applied the put, \
         {storage_served_get} served the get (quorum={QUORUM}, replication={REPLICATION_FACTOR})"
    );

    println!("container log verification succeeded");

    deployment.stop(&nodes).await.unwrap();
    // deployment.cleanup(&nodes).await.unwrap();

    println!("successfully deployed and cleaned up");
}
