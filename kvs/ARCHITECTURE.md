# Architecture

This document describes the architecture of the distributed key-value store (KVS) built on the Hydro framework.

## Overview

The KVS is a two-tier distributed system: a **router cluster** that accepts client connections and a **storage node cluster** that persists data. Values are replicated across multiple storage nodes using rendezvous hashing, and consistency is maintained through vector-clocked CRDT registers.

## Package Layout

The project is split across several Brazil packages so the Smithy contract and the Rust implementation can evolve independently:

| Package | Role |
|---|---|
| `HydroProjectDemoAppModel` | Smithy IDL (`model/main.smithy`) — the single source of truth for the client-facing API |
| `HydroProjectDemoAppServerSdk` | Smithy→Rust server codegen; emits the `amzn-hydro-kvs-protocol` crate (output builders used for HTTP response JSON) |
| `HydroProjectDemoAppClientSdk` | Smithy→Rust client codegen; emits the `amzn-hydro-kvs-client` crate (fluent client used by all tests and examples) |
| `HydroProjectDemoApp` | The Hydro dataflow, the Smithy HTTP sidecar, and the wire codec glue. `distributed_kvs` is the pure dataflow function; `complete_distributed_kvs` is the production wrapper that hooks it up to the Smithy sidecar via Hydro's `bidi_external_sidecar` primitive |
| `HydroProjectDemoAppCDK` | CDK synth; reads `hydro-manifest.json` and provisions ECS/NLB/VPC |
| `HydroProjectDemoAppImageBuild` | Packages the release binary into a Docker image |
| `HydroProjectDemoAppTests` | Hydra integration test harness (runs post-deploy against the live NLB) |

The three Smithy packages (`Model`, `ServerSdk`, `ClientSdk`) follow the standard Amazon Smithy-Rust decomposition: one package for the language-neutral IDL, and one package per emitted Rust crate. This mirrors the layout used by AWS service teams and is the recommended pattern for new Hydro services that expose a Smithy-defined API.

```
                    ┌──────────────────────────────────────────────────┐
                    │                  AWS VPC                         │
                    │                                                  │
  Clients ──TCP──▶  NLB:80  ──▶  ┌─────────┐  ChannelMux:10000       │
                    │             │ Router 0 │ ◀──────────────────┐    │
                    │             │ Router 1 │ ◀────────────┐     │    │
                    │             │ Router 2 │ ◀──────┐     │     │    │
                    │             └─────────┘         │     │     │    │
                    │                  │               │     │     │    │
                    │          rendezvous hash         │     │     │    │
                    │            to N nodes            │     │     │    │
                    │                  │               │     │     │    │
                    │                  ▼               │     │     │    │
                    │             ┌──────────┐         │     │     │    │
                    │             │  Node 0  │ ────────┘     │     │    │
                    │             │  Node 1  │ ──────────────┘     │    │
                    │             │  Node 2  │ ────────────────────┘    │
                    │             │  Node 3  │                         │
                    │             │  Node 4  │                         │
                    │             │  Node 5  │                         │
                    │             │  Node 6  │                         │
                    │             └──────────┘                         │
                    └──────────────────────────────────────────────────┘
```

**Default configuration:** 3 routers, 7 storage nodes, replication factor 3.

## Data Model

Each key maps to a **vector-clocked multi-value register** — a CRDT that supports both sequential overwrites and concurrent writes:

```
type VectorClock = MapUnion<HashMap<String, Max<u64>>>
type ClockedSet  = DomPair<VectorClock, SetUnionHashSet<String>>
```

- **Sequential writes** (dominating vector clock): the new value fully replaces the old one.
- **Concurrent writes** (incomparable vector clocks): both values are preserved via set union. A subsequent read returns all concurrent values.
- **Duplicate/replayed writes**: idempotent via lattice merge — no effect.

This is implemented using the `lattices` crate's `DomPair` type, which pairs a `VectorClock` (the "domain" that determines ordering) with a `SetUnionHashSet<String>` (the payload). When two `ClockedSet` values are merged, the dominating clock's payload wins; concurrent clocks' payloads are unioned.

## Wire Protocol

The client-facing API is defined by the Smithy model in `HydroProjectDemoAppModel/model/main.smithy` and uses the AWS `restJson1` protocol over HTTP/1.1. **The only supported way to talk to the KVS is via the generated Smithy client** (`amzn-hydro-kvs-client`, emitted by `HydroProjectDemoAppClientSdk`) — all tests (Docker, Hydra, and local examples) go through it. Hand-rolled HTTP clients are not a supported surface.

The server side binds to the same contract: `src/smithy_http_port.rs`'s sidecar handlers construct responses using `PutKeyOutput::builder()` / `GetKeyOutput::builder()` from the generated `amzn-hydro-kvs-protocol` crate (emitted by `HydroProjectDemoAppServerSdk`). smithy-rs's `IntoResponse` impl renders those typed outputs as restJson1 responses. Because both sides are generated from the same Smithy model, the wire shape cannot drift from the contract.

Operations:

| Operation | HTTP | Purpose |
|---|---|---|
| `PutKey`  | `PUT /{key}`  with plain-text body | Store a value for a key |
| `GetKey`  | `GET /{key}`                       | Retrieve the value(s) for a key |

Responses are JSON matching the Smithy output shapes:

```json
// PutKey response
{"traceId": "...", "key": "hello", "existingVc": {"router_0": 1}, "nodeIds": ["node_0", "node_1", "node_2"]}

// GetKey response
{"traceId": "...", "key": "hello", "value": ["world"], "existingVc": {"router_0": 1}, "nodeIds": ["node_0", "node_1", "node_2"]}
```

For a `GetKey` on a missing key, both `value` and `existingVc` are absent. The `nodeIds` field in every response lists which storage nodes participated, allowing clients to verify replication factor compliance.

Inter-service traffic (router↔node, node↔node rebalance) is **not** HTTP/Smithy — it uses prost/protobuf (see `proto/kvs_internal.proto` and `src/proto_codec.rs`) over the Hydro `ChannelMux`.

## Request Flow

### Get

```
Client ──HTTP GET /{key}──▶ Router ──rendezvous hash──▶ N storage nodes
                                                         │
                           ◀──KvsResponse (per node)─────┘
                           │
                   merge N responses (union values, max VCs)
                           │
  ◀──HTTP 200 (GetKeyOutput JSON)──┘
```

1. Client (via the Smithy SDK) sends `GetKey { key }` as `GET /{key}` over HTTP.
2. Router parses the HTTP request, assigns a `client_id`.
3. Router computes `rendezvous_targets(key, members, REPLICATION_FACTOR)` to pick N target nodes.
4. Router sends `NodeCommand::Get { key }` to each target via ChannelMux (encoded via prost/protobuf).
5. Each node looks up the key in its local `HashMap<String, ClockedSet>` and returns a `KvsResponse::GetResult` with the value set, existing vector clock, and its own node ID.
6. Router accumulates responses in `merge_responses`. Once N `node_ids` are collected for the `(client_id, key)` pair, it emits the merged response (values unioned, VCs max-merged).
7. The Smithy sidecar handler — which was awaiting a `oneshot::Receiver` for this request — wakes up, converts the `KvsResponse` into a `GetKeyOutput` via the generated `GetKeyOutput::builder()`, returns it. smithy-rs's `IntoResponse` impl renders the restJson1 response body; hyper writes the HTTP 200 response back to the client.

### Put (Read-Before-Write)

Puts use a two-phase protocol to ensure sequential overwrites produce dominating vector clocks:

```
Client ──HTTP PUT /{key} (body=value)──▶ Router
                                           │
                           Phase 1: read existing VC
                                           │──Get(key, offset_client_id)──▶ N nodes
                                           │◀──GetResult(existing_vc)──────┘
                                           │
                           Phase 2: write with dominating VC
                                           │──ClockedPut(key, dominating_vc)──▶ N nodes
                                           │◀──PutOk────────────────────────────┘
                                           │
  ◀──HTTP 200 (PutKeyOutput JSON)──────────┘
```

1. `split_client_command` splits the Put into a read-phase Get (with `client_id + 1_000_000_000` offset) and a pending put record.
2. The read-phase Get is routed to N nodes and merged like a normal Get.
3. `classify_merged_response` identifies the merged response as a read-phase response (offset client_id ≥ 1B).
4. `build_dominating_clocked_put` constructs a `ClockedSet` whose vector clock dominates the existing one by taking the max of each entry and incrementing the router's own entry.
5. The `ClockedPut` is routed to the same N nodes. Each node lattice-merges it into storage — the dominating VC replaces the old value.
6. The `PutOk` response is returned to the client.

This two-phase approach ensures that sequential puts from different routers produce dominating vector clocks (overwrite semantics) rather than concurrent clocks (merge semantics).

## Component Reference

### `distributed_kvs`

The pure KVS dataflow, free of any ingress/egress concerns. Takes a `KeyedStream<u64, KvsCommand, Cluster<KvsRouter>>` of requests and returns a `Stream<(u64, KvsResponse), Cluster<KvsRouter>>` of responses. This shape is what makes the full pipeline exercisable by the sim harness (see `fuzz_distributed_kvs`) without spinning up network I/O.

- Computes `rendezvous_targets(key, members, REPLICATION_FACTOR)` for each command.
- Fans out to N storage nodes, merges responses at quorum.
- Runs the read-before-write protocol for puts.
- Handles node-to-node rebalancing on membership changes.

### `complete_distributed_kvs`

Production wrapper that hosts a Smithy HTTP server alongside the dataflow on every router. Signature: `complete_distributed_kvs<const REP: usize, Ext>(&External<Ext>, &Cluster<KvsRouter>, &Cluster<KvsNode>) -> ExternalBytesPort<Many>`.

Under the hood, it calls [`routers.bidi_external_sidecar`](#bidi_external_sidecar-the-hydro-primitive) with the user's `External` handle (which models the population of HTTP clients) and the `smithy_http_port::kvs_sidecar` closure. Hydro Deploy binds a TCP listener on each router member and exposes it via the container backend's port-mapping machinery; the sidecar accepts connections on that pre-bound listener. Requests are translated into `KvsCommand`s, pushed into `distributed_kvs`, and the returned `KvsResponse`s are routed back to the originating HTTP handler via a `oneshot` map keyed by request id. The caller uses the returned `ExternalBytesPort<Many>` with `DeployResult::get_all_tcp_endpoints` to discover each router member's host-visible address.

### `smithy_http_port::kvs_sidecar`

The sidecar future that runs on every router binary. Receives `(listener: tokio::net::TcpListener, cmds_tx: Sender<(u64, KvsCommand)>, resp_rx: Receiver<(u64, KvsResponse)>)` and:

1. Spawns a drain task that reads `(id, KvsResponse)` from `resp_rx` and routes each response to the matching `oneshot::Sender` stored in a shared `HashMap`.
2. Builds a tower service using `hydro_kvs_protocol::HydroKvs::builder()` with `put_key` and `get_key` handlers. Each handler allocates a request id, inserts a `(id, oneshot::Sender)` pair into the pending map, pushes `(id, KvsCommand)` into `cmds_tx`, and awaits the matching `oneshot::Receiver`.
3. Hands the pre-bound `TcpListener` to `hyper::server::conn::AddrIncoming::from_listener` and runs `hyper::Server::builder(incoming).serve(app.into_make_service())`.

Because the listener is pre-bound by Hydro Deploy, the sidecar never tries to bind a port itself — this avoids collisions and ensures the port is properly exposed in Docker / ECS.

### `bidi_external_sidecar` (the Hydro primitive)

New method on `Location` (lives in `hydro_lang`). Fuses two concepts:

- **External port exposure**: registers a TCP port with Hydro Deploy's `exposed_ports` map and binds a `TcpListener` at runtime, so Docker `--publish` / ECS port mapping make the port host-visible. The returned `ExternalBytesPort<Many>` is compatible with `DeployResult::get_all_tcp_endpoints`.
- **Sidecar-to-dataflow bridge**: creates a pair of bounded mpsc channels and registers a user-supplied async closure (`FnOnce(TcpListener, Sender<(u64, InT)>, Receiver<(u64, OutT)>) -> impl Future`) as a sidecar on the LocalSet alongside the DFIR scheduler.

Under the hood, this primitive adds a new IR root `HydroRoot::ExposeExternalListener` that emits no DFIR code but participates in `compile_network` to drive the deploy backend's new `Deploy::e2m_listener_bind` hook. The mpsc channel setup is emitted via `FlowState::push_extra_stmt` so the channels are in scope for both DFIR construction and the sidecar `spawn_local` call.

This is the primitive that correctly models "external process hosts arbitrary async server, bridged to dataflow via typed channels." `local_sidecar_bidi` (its predecessor, without port exposure) has been removed.

### Inter-node wire codec (`proto_codec.rs`)

Router↔node and node↔node traffic uses prost/protobuf, defined in `proto/kvs_internal.proto` and compiled at build time by `build.rs`. The codec encodes `NodeCommand` / `KvsResponse` / rebalance transfers into `Vec<u8>` just before each Hydro `.demux(...)` and decodes them just after `.entries()`. Hydro's networking layer handles framing of the resulting `Vec<u8>` via its default bincode configuration — no custom `SerKind` is required.

### `kvs_storage`

A `sliced!` block that maintains the per-node `HashMap<String, ClockedSet>`. Accepts two input streams (primary puts from routers, secondary puts from rebalancing) and lattice-merges them into a single state singleton.

### `route_commands`

A `sliced!` block on the router cluster that:

- Maintains a per-router sequence counter for vector clock stamping.
- Computes rendezvous hash targets for each command's key using the current node membership set.
- Converts `KvsCommand::Put` into `NodeCommand::ClockedPut` with a partial VC (just the router's own entry incremented).
- Fans out each command to `REPLICATION_FACTOR` target nodes.

### `node_respond` / `node_respond_with_id`

Processes commands on storage nodes. For each incoming `(router_id, client_id, NodeCommand)`:

- `ClockedPut`: lattice-merges the value into storage (handled by `kvs_storage`), returns `PutOk`.
- `Get`: looks up the key, returns `GetResult` with the value set, existing VC, and this node's ID.

`node_respond_with_id` is the generic version (takes node ID as a parameter, used in sim tests). `node_respond` is the cluster-specific wrapper that uses `CLUSTER_SELF_ID`.

### `merge_responses`

A `sliced!` block that accumulates partial responses from storage nodes. Keyed by `(client_id, key)`, it:

- Unions `node_ids` sets across responses.
- Unions value sets for GetResults.
- Max-merges existing vector clocks.
- Emits the merged response once `REPLICATION_FACTOR` node IDs are collected.

### Rebalancing

When node membership changes (detected by comparing current vs previous membership sets), each node:

1. Iterates over all keys in its local storage.
2. Computes `rendezvous_targets(key, current_members, REPLICATION_FACTOR)` for each key.
3. Sends the key's `ClockedSet` to any target node that isn't itself.
4. The receiving node lattice-merges the data into its storage (idempotent).

This ensures keys are redistributed to their correct owners after nodes join or leave. The lattice merge makes duplicate transfers harmless.

### Pure Helper Functions

| Function | Purpose |
|---|---|
| `rendezvous_targets` | Top-N rendezvous hash: deterministically picks N nodes for a key |
| `build_node_response` | Builds a `KvsResponse` from a `NodeCommand` and the local store |
| `classify_merged_response` | Separates client responses from read-phase responses by client_id offset |
| `split_client_command` | Splits a Put into a read-phase Get + pending put record |
| `build_dominating_clocked_put` | Constructs a `ClockedSet` that dominates an existing VC |

## Deployment

### Build Pipeline

1. `HydroProjectDemoApp` builds the Rust binary and runs `export` to produce `hydro-manifest.json` — a JSON description of all processes, clusters, their ports, and binary names.
2. `HydroProjectDemoAppImageBuild` packages the binary into a Docker image.
3. `HydroProjectDemoAppCDK` reads the manifest at synth time and creates the ECS infrastructure.

### Infrastructure (CDK)

- **VPC**: 2 AZs, public subnets (NLB) + private isolated subnets (ECS tasks), no NAT gateway.
- **VPC Endpoints**: ECR, ECS, CloudWatch Logs, EC2, S3 — all AWS API traffic stays within the VPC.
- **ECS Cluster**: Fargate tasks, 256 CPU / 512 MiB memory per task.
- **NLB**: Internet-facing, port 80, TCP passthrough to router tasks on port 10001.
- **Security Group**: All TCP between tasks (ChannelMux), all TCP from NLB.
- **Service Discovery**: Hydro runtime resolves task IPs via ECS `ListTasks` + `DescribeTasks` API calls through the EC2 VPC endpoint.

### Inter-Service Communication

All inter-service communication uses the Hydro **ChannelMux** on port 10000. The ChannelMux multiplexes multiple named channels over a single TCP connection with a handshake protocol. Channel names used:

| Channel | Direction | Purpose |
|---|---|---|
| `router_to_nodes_gets` | Router → Node | Client Get commands |
| `router_to_nodes_reads` | Router → Node | Read-phase Gets (for Puts) |
| `router_to_nodes_puts` | Router → Node | Write-phase ClockedPuts |
| `nodes_to_router` | Node → Router | Responses back to originating router |
| `node_rebalance` | Node → Node | Key redistribution on membership change |

Connection targets are resolved by `resolve_task_ip`, which calls the ECS API to find task IPs by task ID.

## Testing

### Sim Tests (Exhaustive)

The Hydro simulator exhaustively explores non-deterministic interleavings of the dataflow. Sim tests (all in `src/tests.rs`, functions prefixed `sim_`) cover:

- **Storage layer**: single put, concurrent merge, sequential overwrite, idempotent duplicates, three-way concurrent, dominating VC clears all concurrent, secondary puts, multiple keys.
- **Response merging**: quorum completion, no emit before quorum, value union, VC merging, independent keys/clients, rep factor 1, `None` handling, unions across different nodes.
- **Routing**: sequential puts produce increasing sequence numbers in their vector clocks.
- **Pure functions**: unit tests for `rendezvous_targets`, `build_node_response`, `classify_merged_response`, `split_client_command`, `build_dominating_clocked_put`, and related helpers.

### Fuzz Test

`fuzz_distributed_kvs` drives the full `distributed_kvs` dataflow under randomized sim exploration. This is fuzz-mode rather than exhaustive because the `forward_ref` cycles in `distributed_kvs` (write-phase feedback and rebalance) create a state space too large for exhaustive search.

### Docker Integration Tests

- **`test_kvs_docker`**: Deploys 3 routers + 7 nodes in Docker, runs the full test suite (puts, gets, overwrites, missing keys, 100-key distribution check). All requests go through the Smithy client (`amzn-hydro-kvs-client`) via the `testing` module.
- **`test_kvs_rebalance_docker`**: Puts 50 keys, then kills nodes one at a time (keeping at least `REPLICATION_FACTOR` alive so every key still has a quorum), verifying all keys remain accessible after each kill via rebalancing. Also driven through the Smithy client.

### Hydra Integration Test

Runs in the deployment pipeline after each deploy. Connects to the live ECS service via NLB and runs the same test suite as the Docker test — again using the Smithy client. Configured with a warmup phase to handle post-deploy cluster initialization.
