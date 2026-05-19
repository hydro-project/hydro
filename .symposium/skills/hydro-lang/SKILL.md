---
name: hydro-lang
description: Guidance for building distributed systems with the Hydro dataflow framework
crates: hydro_lang, hydro_std, dfir_rs
---

# Hydro: Distributed Dataflow Programming

Hydro is a Rust framework for building distributed systems using **declarative dataflow** rather than imperative message-passing. Programs describe data transformations and routing; the runtime handles execution, networking, and scheduling.

## Core Concepts

### Locations
Every stream lives on a **location** — either a `Process` (single node) or `Cluster` (replicated group):

```rust
use hydro_lang::prelude::*;

enum Leader {}
enum Workers {}

let mut flow = FlowBuilder::new();
let leader = flow.process::<Leader>();
let workers = flow.cluster::<Workers>();
```

### Quoted Closures (`q!`)
All user logic runs inside `q!(...)` — a compile-time quoted closure that gets shipped to the target location. Variables from the surrounding scope are captured automatically:

```rust
let threshold = 10;
stream.filter(q!(|x| x > threshold))
      .map(q!(|x| x * 2))
```

**Key rule:** Code inside `q!()` runs on the deployed host, not at compile time. Use `q!({...})` (block form) for multi-statement initialization.

### Live Collections
Hydro has four live collection types:
- **`Stream<T, Loc>`** — unbounded sequence of values
- **`Singleton<T, Loc>`** — exactly one value (accumulator, state)
- **`Optional<T, Loc>`** — zero or one value
- **`KeyedStream<K, V, Loc>`** — stream of key-value pairs

### Boundedness
Streams are either `Bounded` (finite, will complete) or `Unbounded` (infinite, runs forever). This is tracked in the type system.

## Stream Operations

### Transformations
```rust
stream.map(q!(|x| x + 1))
stream.filter(q!(|x| x > 0))
stream.filter_map(q!(|x| if x > 0 { Some(x) } else { None }))
stream.flatten_unordered()  // flatten nested collections
stream.unique()             // deduplicate
stream.enumerate()          // add GLOBAL index (persists across batches/ticks)
```

**`enumerate()` behavior depends on context:**
- **Outside `sliced!`** (on `Stream<T, L, Unbounded>`): **Global** counter, persists across ticks. If batch 1 has 3 items (0,1,2), batch 2 starts at 3.
- **Inside `sliced!`** (on `Stream<T, Tick<L>, Bounded>`): **Per-batch** counter, resets to 0 each tick. Use `cross_singleton(base_offset)` to compute global offsets.

Requires `TotalOrder` + `ExactlyOnce` (compile error otherwise — the index is only meaningful with deterministic ordering and no duplicates).

**Offset assignment pattern (from Paxos):**
```rust
let indexed = sliced! {
    let mut next_slot = use::state(|l| l.singleton(q!(0usize)));
    let batch = use(input_stream, nondet!(/** ... */));

    // enumerate() gives per-batch indices (0, 1, 2, ...)
    let indexed = batch.enumerate()
        .cross_singleton(next_slot.clone())
        .map(q!(|((idx, item), base)| (base + idx, item)));

    // Update next_slot for the next tick
    let count = indexed.clone().count();
    next_slot = count.zip(next_slot).map(q!(|(n, base)| base + n));

    indexed
};
```

### Aggregation
```rust
stream.count()  // -> Singleton<usize>
stream.max()    // -> Optional<T>
stream.fold(q!(|| 0), q!(|acc, x| *acc += x))  // -> Singleton<T>
stream.into_keyed().fold(q!(|| 0), q!(|acc, x| *acc += x))  // per-key fold
```

### Combining Streams
```rust
a.merge_unordered(b)       // interleave two streams (no ordering guarantee)
a.chain(b)                 // concatenate (ordered)
a.cross_singleton(s)       // pair each element with a singleton value
singleton_a.zip(singleton_b)  // pair two singletons
```

## Networking

### Process-to-Process
```rust
// Send stream from process A to process B
stream_on_a.send(&process_b, TCP.fail_stop().bincode())
```

### Broadcasting to a Cluster
```rust
// Dynamic membership: third arg is nondet for MEMBERSHIP snapshot timing
stream.broadcast(&workers, TCP.fail_stop().bincode(), nondet!(/** membership is static */))

// Static membership (fixed at deploy time): no nondet needed, simpler
stream.broadcast_closed(&workers, TCP.fail_stop().bincode())
```

The `nondet` parameter on `broadcast` controls **when the cluster membership set is snapshotted** — not message ordering. Internally, `broadcast` uses a `sliced!` block that snapshots the membership list. For static clusters (fixed at deploy time), use `broadcast_closed` instead (no nondet needed, no late-joiner support).

### Demuxing to Specific Members
```rust
// Route (member_id, value) pairs to specific cluster members
keyed_stream.demux(&workers, TCP.fail_stop().bincode())
```

### Transport Options
- `TCP.fail_stop().bincode()` — reliable TCP, binary serialization
- `TCP.lossy(nondet!()).bincode()` — lossy TCP (for eventually-consistent protocols)

## Non-Determinism and `nondet!()`

Every point where the runtime makes a non-deterministic choice (batching, timing, message ordering) is marked with `nondet!()`. This serves two purposes:
1. **Documentation** — explains why non-determinism is acceptable
2. **Simulation** — the simulator varies these choices to find bugs

```rust
// Good: document WHY the non-determinism is safe
stream.sample_every(q!(Duration::from_millis(100)), nondet!(/** leader election is idempotent */))
stream.broadcast(&cluster, TCP.lossy(nondet!(/** state is CRDT, convergent */)).bincode(), nondet!(/** broadcast order doesn't matter */))

// Bad: unexplained
stream.sample_every(q!(Duration::from_millis(100)), nondet!())
```

## The `sliced!` Macro — Batching and Atomicity

`sliced!` defines a **computation slice** where the simulator can vary batch boundaries and state snapshots. The body's last expression is "unsliced" back to an unbounded collection:

- Body returns `Stream<T, Tick<L>, Bounded>` → result is `Stream<T, L, Unbounded>`
- Body returns `Singleton<T, Tick<L>, Bounded>` → result is `Singleton<T, L, Unbounded>`
- Body returns `Optional<T, Tick<L>, Bounded>` → result is `Optional<T, L, Unbounded>`
- Body returns a **tuple** of the above → result is a tuple of unbounded collections (any arity supported)

```rust
// Returns Stream<Response, Process<Leader>, Unbounded, ...>
let response_stream = sliced! {
    let request_batch = use(requests, nondet!(/** batch boundaries don't affect correctness */));
    let state_snapshot = use::atomic(current_state, nondet!(/** atomic read of state */));

    // Last expression determines the return type
    request_batch.cross_singleton(state_snapshot).map(q!(|(req, state)| {
        compute_response(req, state)
    }))
};

// Returns (Stream<...>, Singleton<...>) as a tuple
let (events, counter) = sliced! {
    let batch = use(input, nondet!(/** ... */));
    let mut count = use::state(|l| l.singleton(q!(0usize)));
    let new_count = count.clone().zip(batch.count()).map(q!(|(old, add)| old + add));
    count = new_count.clone();
    (batch.all_ticks(), new_count.into_stream().all_ticks())  // tuple return
};
```

### `use` variants inside `sliced!`:
- `use(stream, nondet!())` — batch elements from a stream (result is `Stream<T, Tick<L>, Bounded>`)
- `use::atomic(singleton, nondet!())` — snapshot a singleton atomically
- `use::state(|l| initial)` — mutable state with initial value (persists across ticks)
- `use::state_null::<Stream<...>>()` — mutable state starting empty (persists across ticks)

### `all_ticks()` and `all_ticks_atomic()`

Inside `sliced!`, streams are tick-bounded (`Stream<T, Tick<L>, Bounded>`). To yield them out of the slice as unbounded streams, use `all_ticks()`:

```rust
let unbounded_result = sliced! {
    let batch = use(input, nondet!(/** ... */));
    let processed = batch.map(q!(|x| x * 2));
    processed  // This is Stream<_, Tick<L>, Bounded> — automatically unsliced
};
// unbounded_result is Stream<_, L, Unbounded> (unsliced by the macro)
```

**You do NOT need to call `all_ticks()` on the final expression** — the `sliced!` macro automatically unslices it. Use `all_ticks()` only when you need to convert a tick-bounded stream to unbounded **inside** the slice body (e.g., to feed it to `broadcast` which expects unbounded input):

```rust
let result = sliced! {
    let batch = use(input, nondet!(/** ... */));
    // Need unbounded stream for broadcast inside the slice
    batch.all_ticks_atomic().broadcast_closed(&cluster, TCP.fail_stop().bincode());
    batch  // return the batch (auto-unsliced)
};
```

### State Feedback in `sliced!`

`use::state_null` creates **local mutable state within the slice** that persists across ticks. You can read the current accumulated value and extend it in the same slice:

```rust
let accumulated = sliced! {
    let new_items = use(input_stream, nondet!(/** ... */));
    let mut state = use::state_null::<Stream<_, _, _, NoOrder>>();

    // state already contains items from previous ticks
    // chain new items onto existing state
    state = state.chain(new_items).unique();

    // Read current state as a singleton (e.g., to get max offset)
    let current_max = state.clone().fold(q!(|| 0u64), q!(|max, val| { /* update max */ }));

    // Use current_max to compute new values
    new_items.cross_singleton(current_max).map(q!(|(item, max)| { /* ... */ }))
};
```

**Important:** The fold over `state.clone()` sees ALL accumulated items (from previous ticks + current batch). This is how you read "current state" within a slice. The state variable is reassigned each tick — the new value becomes the state for the next tick.

### Intra-Tick Visibility

When using `use::atomic(some_singleton, nondet!())` to snapshot a Singleton that is produced by a `fold` elsewhere in the dataflow:

- The **simulator decides which version** of the Singleton to observe — this is the non-determinism.
- A fold update from the **current tick may or may not be visible** to a `use::atomic` snapshot in the same tick. The simulator explores both possibilities.
- For **write-then-read consistency**, ensure the read path's `use::atomic` snapshots a Singleton that is causally downstream of the write. If writes and reads are in separate `sliced!` blocks, the simulator will test the case where the read sees stale state.
- To **guarantee** a read sees a prior write, they must be in the **same `sliced!` block** where the state is computed from the write within that block's body (using `use::state` or direct computation).

## Feedback Loops: `forward_ref` vs `use::state_null`

These serve different purposes:

### `use::state_null` — State within a `sliced!` block
Use for **local accumulation** within a single location's computation slice. State persists across ticks but stays on one node.

```rust
let result = sliced! {
    let mut local_log = use::state_null::<Stream<LogEntry, _, _, NoOrder>>();
    local_log = local_log.chain(new_entries);
    local_log.clone().fold(q!(|| vec![]), q!(|v, e| v.push(e)))
};
```

### `forward_ref` — Circular dataflow references
Use for **cross-location feedback loops** where a stream's output feeds back as its own input (e.g., gossip protocols, convergence loops). Creates a cycle in the dataflow graph.

```rust
let (forward_handle, received_stream) = cluster.forward_ref::<Stream<_, _, Unbounded, NoOrder, AtLeastOnce>>();

// Use received_stream as input to computation...
let output = compute(received_stream);

// Complete the cycle — output feeds back as input
forward_handle.complete(
    output.broadcast(&cluster, TCP.lossy(nondet!()).bincode(), nondet!()).values()
);
```

**Decision rule:** If state stays on one node → `use::state_null`. If data flows between nodes in a cycle → `forward_ref`.

## Simulation Testing

Hydro's **deterministic simulator** exhaustively explores all possible distributed executions. This is the primary testing mechanism.

### Test Structure
```rust
#[test]
fn test_my_protocol() {
    let mut flow = FlowBuilder::new();
    let process = flow.process::<MyProcess>();

    // Create simulation I/O ports — type is inferred or explicit
    let (input_port, input_stream) = process.sim_input::<MyMessage>();
    let output_stream = my_protocol(input_stream);
    let output_port = output_stream.sim_output();

    // Run exhaustive simulation
    flow.sim().exhaustive(async || {
        input_port.send(MyMessage { ... });
        output_port.assert_yields([expected_response]).await;
    });
}
```

### Simulation I/O Type Signatures

For a **Process**:
```rust
// sim_input returns (SimSender<T, TotalOrder, ExactlyOnce>, Stream<T, Process, Unbounded, TotalOrder, ExactlyOnce>)
// Always TotalOrder + ExactlyOnce — this is the only available variant for Process
let (sender, stream) = process.sim_input::<MyType>();

// sim_output returns SimReceiver<T, O, R> (inherits ordering from the stream)
let receiver = stream.sim_output();
```

For a **Cluster**:
```rust
// sim_input returns (SimClusterSender<T, TotalOrder, ExactlyOnce>, Stream<T, Cluster, ...>)
let (sender, stream) = cluster.sim_input::<MyType>();

// Send to a specific cluster member by ID
sender.send(member_id: u32, value: T);

// sim_cluster_output returns SimClusterReceiver — values are (member_id, T)
let receiver = stream.sim_cluster_output();
receiver.next(member_id: u32).await  // get next value from specific member
```

**Ordering variants for sending:**
- `sender.send(value)` — available only on `SimSender<T, TotalOrder, ExactlyOnce>` (ordered delivery)
- `sender.send_many(iter)` — send multiple ordered messages
- `sender.send_many_unordered(iter)` — available on any `SimSender<T, _, ExactlyOnce>` (no ordering guarantee)

### Test Timing Model

`send()` **enqueues messages asynchronously** — it does NOT block or immediately deliver. The simulator advances execution only when you `.await` an assertion:

```rust
flow.sim().exhaustive(async || {
    sender.send(msg1);           // enqueues msg1 (no execution yet)
    sender.send(msg2);           // enqueues msg2 (no execution yet)
    output.assert_yields([...]).await;  // THIS drives the simulator forward
    // After .await returns, the simulator has processed enough ticks
    // to produce the expected output (or panicked if impossible)

    // You can send more after an assertion completes:
    sender.send(msg3);
    output.assert_yields([...]).await;  // drives simulator again
});
```

The simulator explores all possible batch boundaries for the enqueued messages. If you send 2 messages before an assert, the simulator tests: both in one batch, first alone then second, etc.

### Configuring Cluster Size in Simulation
```rust
flow.sim()
    .with_cluster_size(&my_cluster, 3)  // 3 members
    .exhaustive(async || { ... });
```

Without `.with_cluster_size()`, the simulator uses a default size. Always set it explicitly for deterministic tests.

### Key Testing APIs
- `process.sim_input::<T>()` → `(SimSender<T, O, R>, Stream<T, ...>)` — create a test input
- `cluster.sim_input::<T>()` → `(SimClusterSender<T, O, R>, Stream<T, ...>)` — cluster test input
- `stream.sim_output()` → `SimReceiver<T, O, R>` — capture output for assertions
- `stream.sim_cluster_output()` → `SimClusterReceiver<T, O, R>` — cluster output with member IDs
- `flow.sim().exhaustive(async || { ... })` — explore ALL executions
- `flow.sim().fuzz(async || { ... })` — coverage-guided fuzzing for complex protocols
- `flow.sim().with_cluster_size(&cluster, n)` — set cluster size
- `flow.sim().test_safety_only()` — for lossy networking (only tests safety, not liveness)

### Assertion Methods
- `.assert_yields([...]).await` — expect these values (ordered)
- `.assert_yields_only([...]).await` — expect exactly these values, then stream ends
- `.assert_yields_unordered([...]).await` — expect these values in any order
- `.next(member_id).await` — get next value from a specific cluster member (for `SimClusterReceiver`)

### What the Simulator Varies
- **Batch boundaries** — how many messages arrive in each tick
- **Message ordering** — for unordered streams
- **State snapshots** — which version of state is observed
- **Network timing** — when messages arrive at destinations

## Common Patterns

### Broadcast + Converge (Gossip)
```rust
let (forward_ref, received) = cluster.forward_ref::<Stream<_, _, Unbounded, NoOrder, AtLeastOnce>>();

let state = sliced! {
    let local_writes = use(writes, nondet!());
    let remote_writes = use(received, nondet!());
    let mut accumulated = use::state_null::<Stream<_, _, _, NoOrder>>();

    accumulated = accumulated.chain(local_writes).chain(remote_writes.flatten_unordered()).unique();
    accumulated.clone().fold(q!(|| HashSet::new()), q!(|s, v| { s.insert(v); }))
};

forward_ref.complete(
    state.sample_every(q!(Duration::from_millis(50)), nondet!())
         .broadcast(&cluster, TCP.lossy(nondet!()).bincode(), nondet!())
         .values()
);
```

### Request-Response with State
```rust
let response = sliced! {
    let reqs = use(requests, nondet!());
    let snapshot = use::atomic(state, nondet!());
    reqs.cross_singleton(snapshot).map(q!(|(req, s)| handle(req, s)))
};
```

### External I/O (Kafka, HTTP, etc.)
Use `source_iter`, `singleton`, `flat_map_stream_blocking`, and `dest_sink` for external system integration:

```rust
// Source: create a singleton resource, convert to stream
let consumer = location.singleton(q!({ create_consumer(config) }));
let messages = consumer.into_stream()
    .flat_map_stream_blocking(q!(|c| async_message_stream(c)))
    .weaken_retries()
    .weaken_ordering();

// Sink: use dest_sink with a futures::Sink implementation
stream.dest_sink(q!({ create_my_sink(config) }));
```

### Ordering and Retry Assumptions
When consuming from external sources with known guarantees:
```rust
stream
    .assume_ordering::<TotalOrder>(nondet!(/** Kafka partitions are totally ordered */))
    .assume_retries::<ExactlyOnce>(nondet!(/** consumer group handles exactly-once */))
```

## Deployment

```rust
let built = flow.finalize();
let mut hosts = built.with_default_optimize();
hosts = hosts.with_process(&leader, TrybuildHost::new(localhost.clone()));
hosts = hosts.with_cluster(&workers, (0..3).map(|_| TrybuildHost::new(localhost.clone())));
let nodes = hosts.deploy(&mut deployment);
deployment.deploy().await.unwrap();
deployment.start().await.unwrap();
```

## Anti-Patterns

### ❌ Imperative message handling
```rust
// WRONG: Don't write imperative receive loops
loop {
    let msg = recv().await;
    match msg { ... }
}
```

### ✅ Declarative dataflow
```rust
// RIGHT: Declare transformations on streams
input.filter_map(q!(|msg| match msg {
    Request::Read(r) => Some(r),
    _ => None,
})).cross_singleton(state).map(q!(|(req, s)| respond(req, s)))
```

### ❌ Shared mutable state across streams
```rust
// WRONG: Don't use Arc<Mutex<...>> across stream operations
```

### ✅ Use fold/scan for state
```rust
// RIGHT: State lives in fold accumulators or sliced! state
stream.fold(q!(|| initial), q!(|acc, item| update(acc, item)))
```

### ❌ Manual serialization
```rust
// WRONG: Don't manually serialize/deserialize for networking
```

### ✅ Let transport handle it
```rust
// RIGHT: Use typed streams with transport serialization
stream.send(&dest, TCP.fail_stop().bincode())
```

## Key Divergences from Imperative Rust

1. **No `async fn` handlers** — logic is stream transformations, not request handlers
2. **No shared mutable state** — state is in `Singleton`/`fold`, not `Arc<Mutex<>>`
3. **No explicit message sends** — use `.send()`, `.broadcast()`, `.demux()` on streams
4. **Compile-time distribution** — `q!()` closures are compiled and shipped to locations
5. **Batching is explicit** — `sliced!` + `nondet!()` marks where batching decisions happen
6. **Testing explores interleavings** — simulation tests don't run once, they explore all schedules
