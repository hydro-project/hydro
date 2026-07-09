---
sidebar_position: 4
---

# Slice Hooks

Every [slice block](./slices.mdx) begins with one or more **hooks**: `use` statements that reveal a version of a live collection inside the slice. This page is a reference for the available hooks and their semantics for each type of live collection.

```rust,ignore
let output = sliced! {
    // hooks come first...
    let batch = use(requests, nondet!(/** ... */));
    let snapshot = use::atomic(current_count, nondet!(/** ... */));
    let mut buffer = use::state_null::<Stream<_, _, _, TotalOrder>>();

    // ...followed by the body
    ...
};
```

All hooks must appear before the body of the slice, and all collections consumed by hooks must live at the same [location](../locations/index.md).

## The Default Hook: `use`

The default hook, `use(collection, nondet!(...))`, reveals a **bounded** version of the collection: either a *batch* of new elements or a *snapshot* of the current value, depending on the collection type.

| Input collection | Revealed as |
|---|---|
| `Stream` | **Batch** of elements that arrived since the last slice |
| `Singleton` | **Snapshot** of the current value |
| `Optional` | **Snapshot** of the current value (possibly absent) |
| `KeyedStream` | **Batch** of new elements, grouped per key |
| `KeyedSingleton` (unbounded values) | **Snapshot** of the current entries |
| `KeyedSingleton` (`BoundedValue`) | **Batch** of newly arrived entries |

In all cases, the revealed collection is [`Bounded`](../correctness/bounded-unbounded.md): its contents are frozen for the duration of the slice, so you can safely observe it in its entirety (including with [reference handles](./references-mutations.md)).

For example, slicing a `Stream` alongside a `Singleton` reveals a batch of stream elements and a snapshot of the singleton's value at the same point in time:

```rust
# use hydro_lang::prelude::*;
# use futures::StreamExt;
# tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
let requests = process.source_iter(q!(vec![1, 2, 3]));
let scale = process.singleton(q!(10));

let scaled = sliced! {
    let batch = use(requests, nondet!(/** batch boundaries don't affect per-element results */));
    let scale_snapshot = use(scale, nondet!(/** the scale is constant, so all snapshots are identical */));
    batch.cross_singleton(scale_snapshot).map(q!(|(x, s)| x * s))
};
// 10, 20, 30
# scaled
# }, |mut stream| async move {
# let mut results = vec![];
# for _ in 0..3 { results.push(stream.next().await.unwrap()); }
# results.sort();
# assert_eq!(results, vec![10, 20, 30]);
# }));
```

A [`KeyedSingleton`](../streaming-data/keyed-singletons.mdx) with the `BoundedValue` bound gets special treatment. Because each key's value is immutable once it appears, there is no need to re-observe existing entries: the hook reveals a batch containing only the *newly arrived* entries, and each entry is revealed in exactly one slice. This makes `BoundedValue` keyed singletons behave like a stream of request/response entries:

```rust
# use hydro_lang::prelude::*;
# use futures::StreamExt;
# tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
let events: Stream<(&str, i32), _, Unbounded> = process
    .source_iter(q!(vec![("alice", 1), ("bob", 2), ("alice", 3)]))
    .into();
let first_events = events.into_keyed().first(); // KeyedSingleton<&str, i32, _, BoundedValue>

let processed = sliced! {
    let new_entries = use(first_events, nondet!(/** each entry is handled independently, so batching is not observable */));
    new_entries.entries()
};
// ("alice", 1), ("bob", 2) in some order
# processed.map(q!(|(k, v)| (k.to_string(), v)))
# }, |mut stream| async move {
# let mut results = vec![];
# for _ in 0..2 { results.push(stream.next().await.unwrap()); }
# results.sort();
# assert_eq!(results, vec![("alice".to_string(), 1), ("bob".to_string(), 2)]);
# }));
```

### Guarantees

Although the *timing* of slices is non-deterministic, hooks provide several guarantees that make it possible to reason about correctness:

- **Batches partition the input**: every element of a stream (or entry of a `BoundedValue` keyed singleton) appears in **exactly one** batch, and batches preserve the order of the underlying stream. Concatenating all batches yields the original collection.
- **Snapshots are monotone**: the snapshot revealed in a later slice includes at least all data that contributed to the snapshot in an earlier slice. State never appears to "go backwards" — but a snapshot *may* lag behind the latest writes, since updates propagate asynchronously.
- **A single point in time**: all hooks in one `sliced!` block are sliced together, at the same logical point in time.

Because batch boundaries and snapshot timing remain non-deterministic, every `use` of an external collection requires a `nondet!` guard explaining why this non-determinism is acceptable. See [Non-Determinism and `nondet!`](../correctness/nondet.md) for how to write these explanations.

## Atomic Hooks: `use::atomic`

By default, a snapshot may lag arbitrarily behind outputs your program has already released, which can violate guarantees like read-after-write consistency. The `use::atomic(collection, nondet!(...))` hook strengthens the default hook for collections in an atomic context (created with `.atomic()`): the revealed batch or snapshot is guaranteed to be **consistent with respect to** the outputs released via `end_atomic()` on that same atomic context.

```rust,ignore
let increment_request_processing = increment_requests.atomic();
let current_count = increment_request_processing.clone().count();
let increment_ack = increment_request_processing.end_atomic();

let get_response = sliced! {
    let request_batch = use(get_requests, nondet!(/** we never observe batch boundaries */));
    let count_snapshot = use::atomic(current_count, nondet!(/** atomicity guarantees consistency wrt increments */));
    let count_ref = count_snapshot.by_ref();
    request_batch.map(q!(|_| *count_ref))
};
```

If a client has received an acknowledgement released by `end_atomic()`, any later `use::atomic` snapshot will reflect the acknowledged operation. See [Atomic Collections](../atomic-collections.mdx) for the full story.

## State Hooks: `use::state` and `use::state_null`

State hooks declare collections that are **internal** to the slice and persist across slice iterations. They are declared with `let mut`, and the value assigned to the binding at the end of the body is carried over to the next iteration of the slice.

Use `use::state(|l| initial)` when the state has a known initial value. The closure receives the slice's location and returns the state for the first iteration:

```rust
# use hydro_lang::prelude::*;
# use futures::StreamExt;
# tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
# let input_stream = process.source_iter(q!(vec![1, 2, 3]));
let running_count = sliced! {
    let batch = use(input_stream, nondet!(/** batch boundaries don't affect the final count */));
    let mut counter = use::state(|l| l.singleton(q!(0)));

    // Increment the counter by the number of items in this batch
    let new_count = counter.clone().zip(batch.count())
        .map(q!(|(old, add)| old + add));
    counter = new_count.clone();
    new_count.into_stream()
};
# running_count
# }, |mut stream| async move {
# let mut last = 0;
# while last != 3 { last = stream.next().await.unwrap(); }
# }));
```

Use `use::state_null::<Type>()` when the state should start out null: an empty `Stream`, an absent `Optional`, and so on. Because there is no initial value to infer the type from, you must annotate it explicitly. A common pattern is buffering elements until a condition is met, such as holding payloads until a leader is elected:

```rust,ignore
let payloads_with_leader = sliced! {
    let mut unsent_payloads = use::state_null::<Stream<_, _, _, TotalOrder>>();

    let payload_batch = use(payloads, nondet!(/** ... */));
    let latest_leader = use(leader_id, nondet!(/** ... */));

    // Combine buffered and new payloads
    let all_payloads = unsent_payloads.chain(payload_batch);

    // If no leader, buffer everything; otherwise clear the buffer
    unsent_payloads = all_payloads.clone().filter_if(latest_leader.clone().is_none());
    all_payloads.cross_singleton(latest_leader)
};
```

Unlike the other hooks, state hooks do not take a `nondet!` guard: the state itself is just a value carried between iterations. But because the state *evolves* according to non-deterministically sliced inputs, code using state hooks deserves the same careful review as the hooks that feed it.

Instead of reassigning the state binding, you can also mutate state in place with [mutable references](./references-mutations.md) (`by_mut`), which is often clearer when several inputs read and write the same state.

### State Hooks vs. Sliced Singletons

State hooks differ from singletons consumed with `use` in an important way:

- **Sliced singletons** observe *external* state that is derived deterministically (e.g. by `fold`) and updates independently of the slice.
- **State hooks** are *internal* to the slice and hold values you compute between iterations.

Prefer deriving state with deterministic APIs and observing it via `use` when possible; the type system provides stronger guarantees for such state. Reach for state hooks when the update logic fundamentally depends on the slice structure (buffering, batched accumulation, multi-input mutation).

## Unslicing: Returning Values from a Slice

The body of a `sliced!` block returns bounded collections, which are automatically **unsliced** back into live collections that evolve across slices:

| Returned from body | Unsliced result |
|---|---|
| `Stream` (bounded) | Unbounded `Stream` concatenating the elements from every slice |
| `Singleton` | Unbounded `Singleton` continually updated to the latest slice's value |
| `Optional` | Unbounded `Optional` continually updated to the latest slice's value |
| `KeyedStream` (bounded) | Unbounded `KeyedStream` concatenating each key's elements from every slice |
| Tuple of the above | Tuple of unsliced collections |

A `KeyedSingleton` cannot be returned directly; convert it with `.into_keyed_stream()` and return the resulting `KeyedStream` instead.

To keep an output inside the atomic context associated with the slice (so that downstream consumers can establish consistency guarantees), wrap it with `yield_atomic`; see [Atomic Collections](../atomic-collections.mdx).
