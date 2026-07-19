# KVS Architecture

This crate implements a replicated, quorum-based key-value store as a single
Hydro dataflow. It is an example of composing reusable Hydro patterns across
multiple distributed locations rather than a production storage engine.

The store maps each string key to a grow-only set of string values. A `Put`
adds a value to the set; a `Get` returns the union of values reported by a read
quorum.

## System Topology

The dataflow has two logical clusters:

| Location               | Role                                                                  |
| ---------------------- | --------------------------------------------------------------------- |
| `Cluster<RouterNode>`  | Accept client requests, choose replicas, and collect quorum responses |
| `Cluster<StorageNode>` | Maintain in-memory state and serve replica reads and writes           |

The default deployment in `examples/kvs_docker.rs` runs three routers and nine
storage members. Each key is assigned to three storage members
(`REPLICATION_FACTOR = 3`) and an operation completes after two responses
(`QUORUM = 2`).

```text
                         HRW routing
 gRPC client           (top 3 replicas)
      |                         |
      v                         v
+----------------+      +-------------------+
| router sidecar | ---> | storage member A  |
| + router flow  | ---> | storage member B  |
|                | ---> | storage member C  |
+----------------+      +-------------------+
      ^                         |
      |      quorum of 2        |
      +-------------------------+
```

Every router runs the same dataflow. A client connects to one router, and
responses from storage are routed back to the router that originated the
request.

## Data Model and Protocol

The client protocol is defined in `src/lib.rs`:

- `ClientRequest` contains a request ID and an `Op`.
- `Op::Put` contains a key and value.
- `Op::Get` contains a key.
- `ClientResponse` is either a quorum write acknowledgement or the union of a
  read quorum's value sets.

The same `ClientRequest` is routed to storage and split into put and get flows
there. The complete cross-location path is written directly in `kvs()` so the
request lifecycle can be read in execution order without following a generic
orchestration abstraction.

Request IDs correlate asynchronous responses. They only need to be unique
among concurrent requests at one router because the network path also retains
the originating router's `MemberId`. The gRPC sidecar generates monotonically
increasing IDs independently on each router.

The external API is the unary gRPC `KvStore` service in `proto/kvs.proto`.
External clients do not see internal request IDs.

## Request Flow

The `kvs` function in `src/lib.rs` wires together the complete distributed
dataflow.

### Put

1. A router receives `Put { key, value }`.
2. `hrw_scatter` buffers it until the router sees all nine expected storage
   members.
3. The router sends the request to the highest-ranked three members.
4. Each storage member inserts the value into its local set for the key.
5. The storage member releases its acknowledgement only after the local state
   update has been applied.
6. The originating router returns `PutAck` after receiving two replica
   acknowledgements.

The remaining replica may apply the write after the client has received its
acknowledgement.

### Get

1. A router receives `Get { key }`.
2. The router uses the same membership-aware HRW path as puts to send the
   request to the key's three replicas.
3. Each replica reads a snapshot of its local map and returns the key's set. A
   missing key produces an empty set.
4. The router completes the request after receiving two responses.
5. It unions those replicas' sets and returns the result to the client.

Responses arriving after the quorum are absorbed and do not produce another
client response.

## Reusable Combinators

Protocol-independent distributed patterns live under `src/combinators/`.

### `hrw_scatter`

`hrw_scatter` performs highest-random-weight, or rendezvous, hashing. For every
`(routing_key, payload)`, it:

1. Takes a snapshot of the destination cluster's live membership.
2. hashes each `(routing_key, member)` pair;
3. sorts members by descending weight, with member ID as a tie-breaker; and
4. emits one `(member, payload)` pair for each selected replica.

Requests are persisted inside the combinator until the expected membership is
visible, rather than dropped or left for callers to delay. It also emits the
membership count derived from the same snapshot used for routing.

### `atomic_store`

`atomic_store` provides single-location read-after-write scaffolding while
leaving the state representation and read policy to its caller.

Writes enter an atomic Hydro context. The caller folds them into a state
singleton, and `end_atomic` releases acknowledgement tokens only after those
updates have been applied. Reads run in a `sliced!` block and use `use::atomic`
to obtain a snapshot consistent with previously released acknowledgements.

The KVS instantiates this generic combinator with:

```text
State = HashMap<String, HashSet<String>>
Write = (key, value)
Read  = (router, request ID, key)
```

This guarantee is local to one storage member. Distributed consistency is
provided separately by replication and quorums.

### `collect_quorum_responses`

`collect_quorum_responses` accepts a `BoundedValue` keyed singleton with one
immutable response per `(request ID, participant ID)`. It persists partial
response maps across slice boundaries and emits once when enough distinct
participants have responded.

The output is a `HashMap<participant, value>`, not an arrival-ordered `Vec`.
This preserves responder identity, prevents duplicate messages from inflating a
quorum, and avoids claiming that vector append is commutative. KVS uses the same
collector directly for write acknowledgements and read values. Because which
participants respond first is observable in the map, the collector requires a
`NonDet` guard. KVS discharges it by allowing gets to vary only in whether they
observe concurrent, unacknowledged writes.

## Consistency Model

The principal guarantee is read-after-write for acknowledged writes:

> After a `Put` is acknowledged, a later `Get` for the same key observes that
> value, provided routers use the same stable replica membership and the
> operation can reach a quorum.

This follows from two properties:

1. A storage acknowledgement is released only after that replica has applied
   the write, due to `atomic_store`.
2. With three replicas and quorum size two, every read quorum intersects every
   acknowledged write quorum in at least one replica.

Because values form grow-only sets, merging replica responses is set union:
commutative, associative, and idempotent. Concurrent puts cannot overwrite one
another. A get concurrent with unacknowledged writes may return any subset that
has reached its responding replicas.

This is not a general transactional or linearizable key-value store. There are
no deletes, overwrites, multi-key operations, versions, conflict resolution,
or distributed atomic transactions.

## Membership and Failure Assumptions

Replica selection depends on the membership snapshot at each router.
`hrw_scatter` buffers operations until `STORAGE_MEMBERS` members are visible,
and puts and gets share that routing path. The core `kvs` API still exposes the
observed count for diagnostics.

The current implementation assumes:

- storage membership is stable while requests are being served;
- routers agree on the members participating in replica selection;
- at least two selected replicas are reachable; and
- request IDs are not reused concurrently at the same router.

Inter-router membership coordination, rebalancing, hinted handoff, read repair,
anti-entropy, and retry/timeout policies are out of scope. Internal channels use
`TCP.fail_stop()`: a failed channel stops delivering a suffix of messages, so
an operation without a reachable quorum can remain pending. Partial quorum state
is retained until all expected replies arrive, so failed or permanently
incomplete operations are not reclaimed.

## Hydro Execution Model

The crate uses Hydro's type system to make distributed properties explicit:

- `Cluster` location types distinguish router and storage computation.
- `MemberId<StorageNode>` and `MemberId<RouterNode>` prevent mixing cluster
  identifiers.
- Network receives are keyed by sender, and flattening them yields `NoOrder`
  streams.
- Unordered folds carry manual commutativity proofs.
- `sliced!` turns unbounded request streams and state into bounded batches and
  snapshots.
- `nondet!` annotations document where batching, snapshot timing, membership,
  or ordering can vary.
- `atomic`, `end_atomic`, and `use::atomic` establish local happens-before
  relationships between state updates, acknowledgements, and reads.

`#[cfg(stageleft_runtime)] hydro_lang::setup!()` and `build.rs` support
Stageleft's two-stage compilation. Runtime closures are quoted with `q!` so
Hydro can compile the global flow into per-location binaries.

## External I/O and Deployment

`kvs_deploy` attaches a bidirectional sidecar to every router. The sidecar:

1. starts a Tonic gRPC server;
2. translates each RPC into a `ClientRequest`;
3. records a one-shot response channel by request ID;
4. sends the request into the Hydro dataflow; and
5. translates the correlated `ClientResponse` back to protobuf.

The sidecar owns bounded Tokio channels with capacity 1024. It is responsible
for external protocol handling and response correlation; the Hydro dataflow is
responsible for routing, storage, and quorum logic.

`build.rs` generates the Tonic bindings from `proto/kvs.proto` and runs
Stageleft code generation. `examples/kvs_docker.rs` maps the logical clusters
to Docker containers, exposes each router's gRPC port, runs a sample workload,
and verifies the request path through container logs.

## Storage and Durability

Replica state is an in-memory `HashMap<String, HashSet<String>>` owned by the
storage dataflow. A write acknowledgement means the update has been applied to
that in-memory state; it does not mean the value has been flushed to disk.
Restarting a storage member loses its data.

Adding durable storage would require defining recovery, replay, and replica
catch-up behavior in addition to replacing the in-memory state representation.

## Testing Strategy

The test suite has two layers:

- Focused simulation tests validate `hrw_scatter`, `atomic_store`, and
  `collect_quorum_responses` independently.
- End-to-end simulations validate write acknowledgement, cross-router
  read-after-write, multi-value union, and missing-key reads.

End-to-end tests submit requests without waiting for membership; successful
responses therefore exercise internal readiness buffering. They use Hydro's
fuzz simulator where exhaustive exploration would be too large for the
three-router, nine-storage topology. The Docker example exercises generated
binaries, gRPC sidecars, real TCP channels, and container deployment.

## Source Layout

| Path                                          | Responsibility                            |
| --------------------------------------------- | ----------------------------------------- |
| `src/lib.rs`                                  | Complete cross-location KVS dataflow      |
| `src/combinators/atomic_store.rs`             | Local atomic state and read-after-write   |
| `src/combinators/hrw_scatter.rs`              | Membership-aware rendezvous placement     |
| `src/combinators/collect_quorum_responses.rs` | Participant-keyed quorum collection       |
| `src/sidecar.rs`                              | gRPC-to-dataflow bridge                   |
| `src/tests.rs`                                | End-to-end simulation tests               |
| `proto/kvs.proto`                             | External client API                       |
| `examples/kvs_docker.rs`                      | Local container deployment and smoke test |
| `build.rs`                                    | Protobuf and Stageleft code generation    |
