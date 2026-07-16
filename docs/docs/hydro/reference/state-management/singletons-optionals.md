---
sidebar_position: 1
---

# Singletons and Optionals
The core building blocks for state in Hydro are the `Singleton` and `Optional` live collections. A `Singleton` represents a **single Rust value** which may change over time, such as a counter that updates as events are processed. An `Optional` is a value that additionally **may be absent**, like a `Singleton` of a Rust `Option`. You can view the full API documentation for Singletons [here](rust:hydro_lang::live_collections::Singleton) and Optionals [here](rust:hydro_lang::live_collections::Optional).

Both types share the same type parameters:
- `T`: the type of the value
- `L`: the location the value is materialized on (see [Locations](../locations/index.md))
- `B`: tracks whether the value is [bounded or unbounded](../correctness/bounded-unbounded.md)
  - A `Bounded` singleton has reached its final value and will never change
  - An `Unbounded` singleton is a live view that asynchronously changes over time

## Creating Singletons
The simplest way to create a singleton is `Location::singleton`, which materializes a constant value at a location. Because the value never changes, the result is `Bounded`:

```rust,no_run
# use hydro_lang::prelude::*;
# let mut flow = FlowBuilder::new();
# let process = flow.process::<()>();
let zero: Singleton<i32, Process<_>, Bounded> = process.singleton(q!(0));
```

More commonly, singletons are **derived from streams** using aggregations like [`fold`](rust:hydro_lang::live_collections::Stream::fold) and [`count`](rust:hydro_lang::live_collections::Stream::count). The aggregation result is a live collection: as new elements arrive on the input stream, the singleton's value is updated.

```rust
# use hydro_lang::prelude::*;
# use futures::StreamExt;
# tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
let numbers = process.source_iter(q!(vec![1, 2, 3, 4]));
let sum: Singleton<i32, Process<_>, Bounded> =
    numbers.fold(q!(|| 0), q!(|acc, x| *acc += x));
// 10
# sum.into_stream()
# }, |mut stream| async move {
# assert_eq!(stream.next().await, Some(10));
# }));
```

Here, the input stream is `Bounded` (it comes from a fixed `Vec`), so the aggregation result is also `Bounded`: once all four elements are folded, the sum is final. If the input stream were `Unbounded` (e.g., requests arriving over the network), the same code would produce an *unbounded* singleton whose value continues to grow as requests arrive.

Just like on streams, aggregations on unordered or at-least-once streams require **property annotations** (commutativity / idempotence with `manual_proof!`); see [Streams](../streaming-data/streams.md) for details.

## Optionals
An `Optional` is a value that may be absent. Optionals commonly arise from aggregations that have no result until the first element arrives, such as [`reduce`](rust:hydro_lang::live_collections::Stream::reduce), [`max`](rust:hydro_lang::live_collections::Stream::max), [`min`](rust:hydro_lang::live_collections::Stream::min), and [`first`](rust:hydro_lang::live_collections::Stream::first), or from filtering a singleton:

```rust
# use hydro_lang::prelude::*;
# use futures::StreamExt;
# tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
let numbers = process.source_iter(q!(vec![4, 2, 9, 5]));
let largest: Optional<i32, Process<_>, Bounded> = numbers.max();
// Some(9)
# largest.into_stream()
# }, |mut stream| async move {
# assert_eq!(stream.next().await, Some(9));
# }));
```

Optionals offer APIs analogous to Rust's `Option` for handling the empty case:
- `unwrap_or(other)` falls back to a `Singleton` when the optional is empty
- `or(other)` falls back to another `Optional`
- `into_singleton()` converts to a `Singleton<Option<T>>`
- `is_some()` / `is_none()` produce a `Singleton<bool>`, often used with `filter_if`

```rust
# use hydro_lang::prelude::*;
# use futures::StreamExt;
# tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
let numbers = process.source_iter(q!(Vec::<i32>::new()));
let largest = numbers.max(); // empty, since there are no elements
let with_default: Singleton<i32, Process<_>, Bounded> =
    largest.unwrap_or(process.singleton(q!(-1)));
// -1
# with_default.into_stream()
# }, |mut stream| async move {
# assert_eq!(stream.next().await, Some(-1));
# }));
```

## Transforming Values
Singletons and optionals can be transformed with `map`, `filter`, and `filter_map`, just like streams. Filtering a `Singleton` produces an `Optional`, since the value may be dropped. To combine multiple values, use `zip`, which pairs them into a tuple:

```rust
# use hydro_lang::prelude::*;
# use futures::StreamExt;
# tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
let numbers = process.source_iter(q!(vec![1, 2, 3, 4]));
let count = numbers.clone().count();
let sum = numbers.fold(q!(|| 0), q!(|acc, x| *acc += x));
let average = sum.zip(count).map(q!(|(s, c)| s as f64 / c as f64));
// 2.5
# average.into_stream()
# }, |mut stream| async move {
# assert_eq!(stream.next().await, Some(2.5));
# }));
```

When the inputs to `zip` are unbounded, the result is a live pair that updates as either side changes.

## Boundedness and Monotonicity
The `B` type parameter determines which APIs are available. A `Bounded` singleton is a plain, frozen Rust value: it can be observed in its entirety, converted to a stream with `into_stream()`, combined with bounded collections (e.g. `Stream::cross_singleton`), or captured by [reference](./references-mutations.md) inside `q!()` closures. An `Unbounded` singleton cannot be directly observed, because any observation would capture a non-deterministic version of the value.

In addition to `Bounded` and `Unbounded`, singletons support a third bound: `Monotonic`, which marks values that only **grow** over time (for example, `count()` on an unbounded stream returns `Singleton<usize, _, Monotonic>`). Monotonicity is a weaker guarantee than boundedness but still enables deterministic APIs like threshold checks (`threshold_greater_or_equal`), since a growing value crosses a fixed threshold at most once. Aggregations can be marked monotonic with a `monotone = manual_proof!(...)` annotation.

## Reading State in a Service
To read an unbounded singleton — for example, answering get requests with the current value of a counter — take a **snapshot** inside a [`sliced!`](./slices.mdx) block. The snapshot is a `Bounded` singleton, so within the slice you can capture it with `by_ref()` and read it like a regular Rust value:

```rust
# use hydro_lang::prelude::*;
# use futures::StreamExt;
# tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
let increments = // ... stream of increment requests
#     Stream::<(), _, Unbounded>::from(process.source_iter(q!(vec![(), (), ()])));
let get_requests = // ... stream of get requests
#     Stream::<(), _, Unbounded>::from(process.source_iter(q!(vec![()])));
let current_count = increments.count();

let get_response = sliced! {
    let request_batch = use(get_requests, nondet!(/** we never observe batch boundaries */));
    let count_snapshot = use(current_count, nondet!(/** each request observes some valid version of the count */));

    let count_ref = count_snapshot.by_ref();
    request_batch.map(q!(|_| *count_ref))
};
# get_response
# }, |mut stream| async move {
# let response = stream.next().await.unwrap();
# assert!(response <= 3);
# }));
```

Snapshots are asynchronous: each snapshot is at least as recent as the previous one, but may lag behind acknowledged writes. See [Slice Blocks](./slices.mdx) for the semantics of slicing, [References and Mutations](./references-mutations.md) for `by_ref()` and `by_mut()`, and [Atomic Collections](../atomic-collections.mdx) for establishing read-after-write consistency.
