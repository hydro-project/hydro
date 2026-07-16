---
sidebar_position: 1
---

# Bounded and Unbounded Types
Live collections capture data that changes over time, but not every collection is still in flux: some collections are already **complete**, with their full contents immediately available. For example, a live collection created from an in-memory `Vec` is complete from the start: all of its elements are available right away. A stream of requests arriving over the network, by contrast, may be extended with new elements at any point in the future.

Hydro tracks this distinction in the type system. All live collections have a type parameter (typically named `B`) which is one of two types:
- **`Bounded`**: the collection is complete; its full contents are **immediately available** and no further asynchronous changes can arrive
- **`Unbounded`**: the collection may still change asynchronously in the future

Because a bounded collection is complete, it is safe to observe it **in its entirety**: you can read the complete set of elements, combine it with other bounded collections, or capture a [reference](../state-management/references-mutations.md) to its value. For an unbounded collection, any such observation would capture the collection at some arbitrary intermediate point, and _which_ intermediate point is observed depends on runtime timing. To preserve [determinism](./determinism.md), APIs that observe an entire collection are only available when the collection is `Bounded`.

For example, a stream created from a fixed `Vec` is bounded, so we can aggregate it and immediately observe the final result:

```rust,no_run
# use hydro_lang::prelude::*;
# let mut flow = FlowBuilder::new();
# let process = flow.process::<()>();
let numbers: Stream<_, Process<_>, Bounded> = process.source_iter(q!(vec![1, 2, 3]));
let total: Singleton<_, Process<_>, Bounded> = numbers.fold(q!(|| 0), q!(|acc, x| *acc += x));
```

Because `total` is bounded, its complete value is available immediately; you can convert it into a stream with `Singleton::into_stream`, or pass it to `Stream::cross_singleton` to pair its value with each element of a stream. Both of these APIs observe the whole value of the singleton, so they require it to be `Bounded`: if `total` were unbounded, the observed value would depend on the timing of upstream updates, and the program would not compile.

:::note

Boundedness is a stronger guarantee than being _finite_. A collection that will only ever contain ten elements, but receives those elements asynchronously over the network, is still `Unbounded`: at any moment, more elements could still be on their way.

:::

## Converting Boundedness
In some cases, you may need to convert between bounded and unbounded collections. Converting from a bounded collection **to an unbounded collection** is always allowed and safe, since it relaxes the guarantees on the collection. This can be done by calling `.into()` on the collection.

```rust,no_run
# use hydro_lang::prelude::*;
# let mut flow = FlowBuilder::new();
# let process = flow.process::<()>();
let input: Stream<_, _, Bounded> = process.source_iter(q!(vec![1, 2, 3, 4]));
let unbounded: Stream<_, _, Unbounded> = input.into();
```

```rust,no_run
# use hydro_lang::prelude::*;
# let mut flow = FlowBuilder::new();
# let process = flow.process::<()>();
let input: Singleton<_, _, Bounded> = process.singleton(q!(0));
let unbounded: Singleton<_, _, Unbounded> = input.into();
```

Converting from an unbounded collection **to a bounded collection**, however, is more complex. This requires cutting off the unbounded collection at a specific point in time, which is not possible to do deterministically. In Hydro, this conversion is performed by taking a **slice** of the unbounded collection with the [`sliced!`](../state-management/slices.mdx) macro. Inside a `sliced!` block, each `use` hook reveals a bounded version of a live collection: a **batch** of new elements for a `Stream`, or a **snapshot** of the current value for a `Singleton`. Because the boundaries of batches and the timing of snapshots depend on runtime factors, each `use` hook requires a [non-determinism guard](./nondet.md):

```rust
# use hydro_lang::prelude::*;
# use futures::StreamExt;
# tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
let unbounded_input: Stream<_, _, Unbounded> =
    process.source_iter(q!(vec![1, 2, 3, 4])).into();

let doubled = sliced! {
    // inside the slice, `batch` is a **bounded** stream of newly arrived elements
    let batch = use(unbounded_input, nondet!(
        /// each element is transformed independently, so
        /// batch boundaries do not affect the final result
    ));
    batch.map(q!(|x| x * 2))
};
# doubled
# }, |mut stream| async move {
# for w in [2, 4, 6, 8] {
#     assert_eq!(stream.next().await, Some(w));
# }
# }));
```

Within the body of the slice, the batch is bounded, so the full set of bounded-only APIs is available. When the result is returned out of the `sliced!` block, it becomes unbounded again, since new slices will continue to be processed over time. See [Slice Blocks](../state-management/slices.mdx) for a full guide.

## Futures and Boundedness
When working with asynchronous futures in a stream, the choice of resolution strategy affects boundedness:

- **`resolve_futures`** and **`resolve_futures_ordered`** always produce an `Unbounded` stream, because the resolved values arrive asynchronously over time. Downstream computation continues while futures are pending, so the results are never available all-at-once.

- **`resolve_futures_blocking`** preserves the input stream's boundedness. It blocks until all futures in the input resolve, so if the input was bounded, the complete set of results is immediately available as well. This allows the output to be used with bounded-only APIs like `cross_singleton`.

**When to use which:** In most cases, prefer `resolve_futures` — it allows you to process results as they stream in without blocking. Use `resolve_futures_blocking` only when you need the bounded guarantee, for example to combine the resolved results with other bounded collections within a slice.
