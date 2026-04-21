# coord-criterion branch: status and remaining work

## Resolved issues

### 1. ~~Dynamic broadcast to Cluster~~ (FIXED)
StaticCluster location type added. The analysis uses property-based
`complete_delivery = no_late_joiners && is_gap_free()` where
`no_late_joiners` is true for StaticCluster (location property) or
`is_durable()` (channel property via `durable_broadcast`).

The `label_fixed_membership` / `consistency_fixed` dual-label mechanism
is retained — it shows what the label would be under fixed membership,
useful for diagnosing dynamic cluster issues.

### 2. ~~Sink identifiers keyed by source spans~~ (MITIGATED)
The coordination analysis now reads `metadata.tag` (set by `ir_node_named()`)
for user-provided sink names. This provides stable identifiers that survive
refactoring. Example: `tx_responses @ service.rs:204:10` instead of
`foreach @ service.rs:204:10`.

A PR for a cleaner `.name()` API (following Flink's convention) was drafted
but withdrawn — it duplicated the existing `ir_node_named` mechanism. The
existing mechanism works; a future cleanup could rename `ir_node_named` to
`.name()` and unify with the channel naming system.

### 3. ~~viz/render.rs refactor~~ (NOT BLOCKING)
Not addressed. Low priority — the viz overlay is a future feature.

### 4. ~~Remaining review items~~ (ADDRESSED)
- `SinkResult` and `CoordinationReport` are intentionally public API.
- `HYDRO_CHECK_COORDINATION` env var: coordination analysis is now lazy
  (only runs when the env var is set), so the stderr output is opt-in.
- Inline codegen fixes: landed independently in upstream.

## Current state

The branch is rebased on current `origin/main` (including upstream PRs
for `entries_partially_ordered`, `KeyedStream::get` ordering preservation,
and `order_preserving` on MapFuncAlgebra).

### Features implemented
- Backward goal-seeking proof (Prefix, SetInclusion, Lattice)
- Forward consistency label propagation
- Cycle analysis with fixpoint iteration
- StaticCluster support (type-level, no manual proof needed)
- `durable_broadcast` for log-based recovery
- `ChannelDelivery` enum, `NetworkingInfo::Durable`, `DurableTransport`
- Associativity annotation and check for bounded folds under Prefix
- PER_KEY SEQ CONSISTENT label with `entries_partially_ordered` detection
- Sink naming via `ir_node_named` / `metadata.tag`
- Simulator integration (ConsistencyCollector)
- Replication analysis

### Label hierarchy
```
SEQ CONSISTENT > PER_KEY SEQ CONSISTENT > CONVERGENT > LOCAL > INCONSISTENT
```

## Remaining work

### Per-key sequential consistency for merged input streams
The PER_KEY SEQ CONSISTENT label exists and is sound for cases where
global Prefix passes through `entries_partially_ordered`. However,
services that merge multiple input streams (e.g., microdb's writes +
reads via `merge_unordered`) cannot achieve PER_KEY SEQ because global
Prefix fails at the chain. A sound `PerKeyPrefix` goal that passes
through chain was attempted but found to be unsound (shared state means
one client's responses depend on other clients' operations).

To achieve PER_KEY SEQ for write confirmations in microdb, the service
architecture must separate the write confirmation path (TotalOrder
journal outcomes) from the read path. This aligns with the Java MicroDB
model where offer callbacks are a separate event stream.

### kvs_zoo StaticCluster for Paxos
The linearizable_replicated example shows CONVERGENT instead of SEQ
CONSISTENT because all clusters use `flow.cluster()` (dynamic). Paxos
proposers/acceptors should use `flow.static_cluster()`. Requires
refactoring `KVSClusters` to support `StaticCluster`.

### Stream naming PR
A `.name()` API for labeling dataflow edges was prototyped (PR #2775,
closed). The existing `ir_node_named` works but has a less intuitive
name. A future PR could rename it to `.name()` and unify with the
channel naming system (`TCP.name("channel")`).

### Gossip complete delivery
CRDT gossip gets its consistency proof from lattice properties
(commutative + idempotent fold). A broadcast-based gossip with
`complete_delivery` was discussed but deferred — the optimization of
reordering merge and forward requires program transformation support
that doesn't exist yet.
