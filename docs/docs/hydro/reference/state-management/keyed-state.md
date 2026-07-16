---
sidebar_position: 2
---

# Keyed State
Many services maintain independent state for each key: a counter per metric name, a session per user, a balance per account. Hydro models this with the `KeyedSingleton` live collection, which maps keys of type `K` to values of type `V` — like a `HashMap` that stays live, with each entry updating as that key's inputs arrive. You can view the full API documentation [here](rust:hydro_lang::live_collections::KeyedSingleton).

This page focuses on keyed singletons used as **mutable per-key state**, where values are updated over time (the `Unbounded` bound). Keyed singletons whose values are *immutable once present* (the `BoundedValue` bound) behave more like streaming responses and are covered in [Keyed Singletons](../streaming-data/keyed-singletons.mdx).

## Creating Keyed State
Keyed state is derived from a [`KeyedStream`](../streaming-data/keyed-streams.mdx) using per-key aggregations such as [`fold`](rust:hydro_lang::live_collections::KeyedStream::fold), [`reduce`](rust:hydro_lang::live_collections::KeyedStream::reduce), and [`value_counts`](rust:hydro_lang::live_collections::KeyedStream::value_counts). These behave like SQL `GROUP BY` aggregations, maintaining a separate accumulator for each key:

```rust
# use hydro_lang::prelude::*;
# use futures::StreamExt;
# tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
let purchases = process.source_iter(q!(vec![
    ("alice", 10),
    ("bob", 5),
    ("alice", 15),
])).into_keyed();

# let purchases = KeyedStream::<_, _, _, Unbounded>::from(purchases);
let total_spent: KeyedSingleton<&str, i32, _, _> =
    purchases.fold(q!(|| 0), q!(|acc, amount| *acc += amount));
// { "alice": 25, "bob": 5 }
# sliced! {
#     let snapshot = use(total_spent, nondet!(/** test */));
#     snapshot.entries()
# }.map(q!(|(k, v)| (k.to_string(), v)))
# }, |mut stream| async move {
# let mut latest = std::collections::HashMap::new();
# while latest.get("alice") != Some(&25) || latest.get("bob") != Some(&5) {
#     let (k, v) = stream.next().await.unwrap();
#     latest.insert(k, v);
# }
# }));
```

When the input keyed stream is unbounded, the resulting keyed singleton is a live view: new keys appear as they are first seen, and each key's value updates as more of its elements arrive. Just like aggregations on streams, keyed aggregations require **property annotations** (`commutative = manual_proof!(...)`, `idempotent = manual_proof!(...)`) when the input has weaker ordering or retry guarantees; see [Keyed Streams](../streaming-data/keyed-streams.mdx) for details.

:::tip

Just as `Stream` has `count()` as a shorthand for a common fold, `KeyedStream` has `value_counts()`, which produces a `KeyedSingleton<K, usize>` counting the elements per key. This is the natural way to build a keyed counter service.

:::

## Reading Keyed State
Because keyed state updates asynchronously, reading it requires taking a **snapshot** inside a [`sliced!`](./slices.mdx) block, where the keyed singleton is revealed as a `Bounded` collection of its current entries. A common pattern is answering a batch of per-key requests by joining it against the snapshot with `join_keyed_singleton`, which looks up each request's key:

```rust,no_run
# use hydro_lang::prelude::*;
# let mut flow = FlowBuilder::new();
# let process = flow.process::<()>();
# let increments: KeyedStream<u32, u64, Process<_>, Unbounded> =
#     process.source_iter(q!(vec![(1u32, 5u64)])).into_keyed().into();
# let get_requests: KeyedStream<u32, (), Process<_>, Unbounded> =
#     process.source_iter(q!(vec![(1u32, ())])).into_keyed().into();
let totals = increments.fold(q!(|| 0), q!(|acc, amount| *acc += amount));

let get_response = sliced! {
    let request_batch = use(get_requests, nondet!(/** we never observe batch boundaries */));
    let totals_snapshot = use(totals, nondet!(/** each request observes some valid version of the totals */));

    request_batch
        .join_keyed_singleton(totals_snapshot)
        .map(q!(|((), total)| total))
};
```

The result is a keyed stream of responses, still grouped by key, which can be routed back to clients.

:::info

`join_keyed_singleton` only emits entries for keys that **exist** in the keyed singleton, so a request for a key that has never been written produces no response. To respond with a default instead, use `lookup_keyed_stream` or `lookup_keyed_singleton`, which match each request's *value* against the lookup table's keys and emit an `Option` that is `None` when no entry exists. To look up a single key, use `get`.

:::

Snapshots of keyed state are asynchronous, just like snapshots of singletons: each snapshot is at least as recent as the previous one, but may lag behind acknowledged writes. When clients need read-after-write consistency, derive the keyed state from an atomic stream and snapshot it with `use::atomic`; see [Atomic Collections](../atomic-collections.mdx) and the [keyed counter tutorial](../../learn/quickstart/keyed-counter.mdx) for a complete example.

## Materializing Keyed State
Sometimes you need the entire keyed collection as a single value:
- `into_singleton()` converts a `KeyedSingleton<K, V>` into a `Singleton<HashMap<K, V>>`, useful when a computation needs to observe all entries at once
- `key_count()` produces a `Singleton<usize>` counting the number of keys
- `entries()`, `keys()`, and `values()` flatten the collection into (unordered) streams

Since keys are unordered across groups, streams produced by flattening have `NoOrder` ordering, and downstream aggregations may require commutativity annotations.

## Rekeying and Transformation
Keyed singletons support per-key transformations like `map`, `filter`, and `filter_map` (operating on each value), and `map_with_key` when the transformation also needs the key. To change the *grouping key* of state (e.g., from client ID to resource name), regroup the underlying keyed stream **before** aggregating: flatten with `entries()`, remap the tuples, and regroup with `into_keyed()`, as shown in the [keyed counter tutorial](../../learn/quickstart/keyed-counter.mdx).
