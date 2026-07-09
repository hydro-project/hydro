---
sidebar_position: 1
---

# Live Collections
Traditional programs (like those written with Rust's standard library) manipulate **collections** of data, such as those stored in a `Vec` or `HashMap`. These collections are **fixed**: transformations such as `map` are immediately executed on a snapshot of the collection, so the output will not change if the input is modified later.

In Hydro, programs instead work with **live collections**, which dynamically change over time as new elements arrive (from API requests, streaming ingestion, state updates, etc). Applying a transformation like `map` to a live collection results in another live collection: whenever the input is updated, the changes asynchronously flow to the downstream collections.

Live collections are the most fundamental concept in Hydro. All network inputs and outputs are live collections — a service receives requests as a live collection and emits responses as another live collection — so the majority of application logic in Hydro consists of transforming live collections.

```rust
# use hydro_lang::prelude::*;
# use futures::StreamExt;
# tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
// `requests` is a live collection; new elements arrive over time
let requests = process.source_iter(q!(vec!["alice", "bob"]));
// `greetings` is also live; it updates as new requests arrive
let greetings = requests.map(q!(|name| format!("Hello, {}!", name)));
# greetings
# }, |mut stream| async move {
# assert_eq!(stream.next().await, Some("Hello, alice!".to_string()));
# assert_eq!(stream.next().await, Some("Hello, bob!".to_string()));
# }));
```

## Types of Live Collections
Hydro offers several types of live collections that capture different asynchronous semantics:
- **[Stream](../streaming-data/streams.md)**: a growing sequence of elements arriving over time (API requests, events, logs)
- **[Keyed Stream](../streaming-data/keyed-streams.mdx)**: a stream grouped by key, with independent ordering for each group (requests across several concurrent clients, "GROUP BY")
- **[Keyed Singleton](../streaming-data/keyed-singletons.mdx)**: a single value for each key; with immutable values this models one response per request, while mutable values model per-key state (sessions, grouped aggregations)
- **[Singleton / Optional](../state-management/singletons-optionals.md)**: a single value (or no value) changing over time (local state, aggregation results)

These collections come in two flavors based on how they are used. **Streaming data** — streams, keyed streams, and keyed singletons with immutable values — captures the requests and responses flowing through a service, and is covered in [Streaming Data](../streaming-data/index.md). **State** — singletons, optionals, and keyed singletons with mutable values — captures values that are updated in place as events are processed, and is covered in [State Management](../state-management/index.md).

## Live Collections and Correctness
Because live collections update asynchronously, you might expect Hydro programs to be riddled with race conditions. Instead, Hydro uses the Rust type system to preserve strong correctness guarantees:
- Each live collection type tracks whether it is [**bounded or unbounded**](../correctness/bounded-unbounded.md) — whether its final contents are already fully determined or new changes may still arrive. APIs that need to observe a collection "in its entirety" are only available on bounded collections.
- All safe APIs on live collections guarantee [**eventual determinism**](../correctness/determinism.md): given the same eventual inputs, a program always produces the same eventual outputs, regardless of network delays or event interleaving. The places where non-determinism is truly necessary are explicitly marked with [`nondet!`](../correctness/nondet.md).
