---
sidebar_position: 5
---

# References and Mutations

Hydro's dataflow operators like `zip` and `cross_singleton` combine collections by pairing up their elements. This works well for simple combinations, but it can become awkward when a transformation needs to consult *several* pieces of state, or when state must be **updated** while processing each element. For these cases, Hydro provides **reference handles**: lightweight handles to a live collection that can be captured inside `q!()` closures and used like ordinary Rust references.

## Reference Handles with `by_ref`

Calling `.by_ref()` on a live collection returns a handle that can be captured by any `q!()` closure at the same location. At runtime, the handle resolves to a shared reference to the collection's current contents:

| Collection | Handle resolves to |
|---|---|
| `Singleton<T>` | `&T` |
| `Optional<T>` | `&Option<T>` |
| `Stream<T>` | `&Vec<T>` (the stream's buffered elements) |

For example, we can compute an aggregate and then read it while transforming another stream, without any tuple plumbing:

```rust
# use hydro_lang::prelude::*;
# use futures::StreamExt;
# tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
let total: Singleton<i32, _, Bounded> = process
    .source_iter(q!(0..5i32))
    .fold(q!(|| 0), q!(|acc, x| *acc += x)); // 0 + 1 + 2 + 3 + 4 = 10
let total_ref = total.by_ref();

let shifted = process
    .source_iter(q!(vec![1, 2, 3]))
    .map(q!(|x| x + *total_ref));
// 11, 12, 13
# shifted
# }, |mut stream| async move {
# let mut results = vec![];
# for _ in 0..3 { results.push(stream.next().await.unwrap()); }
# results.sort();
# assert_eq!(results, vec![11, 12, 13]);
# }));
```

Reference handles require the collection to be [**bounded**](../correctness/bounded-unbounded.md). A bounded collection's contents are fully determined, so reading it as a whole is deterministic. Reading an *unbounded* collection this way would expose whatever portion happened to have arrived — a non-deterministic result. This restriction is enforced at compile time.

In practice, most reference handles appear inside [slice blocks](./slices.mdx): the batches and snapshots revealed by [slice hooks](./slice-hooks.md) are bounded, so `by_ref` is the natural way to read a snapshot of state while processing a batch of requests:

```rust
# use hydro_lang::prelude::*;
# use futures::StreamExt;
# tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
let get_requests = process.source_iter(q!(vec![1, 2, 3]));
let highest_bid: Optional<i32, _, _> = process
    .source_iter(q!(vec![55, 82]))
    .max();

let responses = sliced! {
    let request_batch = use(get_requests, nondet!(/** each request is handled independently */));
    let bid_snapshot = use(highest_bid, nondet!(/** each request observes the highest bid at the time it is processed */));
    let bid_ref = bid_snapshot.by_ref();
    request_batch.map(q!(|req| (req, bid_ref.unwrap_or(0))))
};
# responses.map(q!(|(a, b)| a + b))
# }, |mut stream| async move {
# let mut results = vec![];
# for _ in 0..3 { results.push(stream.next().await.unwrap()); }
# assert_eq!(results.len(), 3);
# }));
```

## Mutable References with `by_mut`

Calling `.by_mut()` returns a **mutable** handle, resolving to `&mut T` (or `&mut Option<T>` / `&mut Vec<T>`). Closures capturing the handle can update the value in place, and the mutation is observed by all later reads of the collection.

Mutable references shine inside slice blocks, combined with [state hooks](./slice-hooks.md). Instead of expressing a stateful computation as a fold-style reassignment, you can mutate the state directly while processing each element:

```rust
# use hydro_lang::prelude::*;
# use futures::StreamExt;
# tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
let deposits = process.source_iter(q!(vec![10, 20, 30]));

let running_balance = sliced! {
    let batch = use(deposits, nondet!(/** running totals are unaffected by batch boundaries */));
    let mut balance = use::state(|l| l.singleton(q!(0)));
    let balance_mut = balance.by_mut();

    batch.map(q!(|amt| {
        *balance_mut += amt;
        *balance_mut
    }))
};
// 10, 30, 60
# running_balance
# }, |mut stream| async move {
# let mut results = vec![];
# for _ in 0..3 { results.push(stream.next().await.unwrap()); }
# assert_eq!(results, vec![10, 30, 60]);
# }));
```

Because `balance` is a state hook, mutations made through `balance_mut` persist across slice iterations — the balance keeps accumulating no matter how the deposits are batched.

## Mutation Order Follows Code Order

When a collection is accessed by several closures — especially when some of them mutate it — Hydro must decide the order in which those accesses execute. The rule is simple: **accesses execute in the order they appear in your code**, not in the order the collections are consumed downstream. Mutable accesses are exclusive: each mutation completes before the next access begins.

```rust
# use hydro_lang::prelude::*;
# use futures::StreamExt;
# tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
let input = process.source_iter(q!(vec![3]));

let out = sliced! {
    let batch = use(input, nondet!(/** single input element, so batching is not observable */));
    let mut total = use::state(|l| l.singleton(q!(0)));
    let total_mut = total.by_mut();

    // Declared FIRST in code: addition
    let added = batch.clone().map(q!(|x| {
        *total_mut += x;
        *total_mut
    }));
    // Declared SECOND in code: doubling
    let doubled = batch.map(q!(|_x| {
        *total_mut *= 2;
        *total_mut
    }));
    // Consumed in the OPPOSITE order; mutations still run in code order
    doubled.chain(added)
};
// 6, 3
# out
# }, |mut stream| async move {
# let mut results = vec![];
# for _ in 0..2 { results.push(stream.next().await.unwrap()); }
# assert_eq!(results, vec![6, 3]);
# }));
```

Even though `doubled` is consumed before `added`, the addition runs first (total becomes `0 + 3 = 3`), then the doubling (total becomes `3 * 2 = 6`), because that is the order the transformations were written. This makes imperative state updates read top-to-bottom, just like sequential Rust code.

## Shared State Across Multiple Inputs

The most important use of mutable references is state that must be **read and written by several streaming inputs**. A single `fold` can only aggregate one input stream; when multiple request types interleave reads and writes to the same state, mutable references inside a slice express the logic directly:

```rust
# use hydro_lang::prelude::*;
# use futures::StreamExt;
# tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
# let deposits = process.source_iter(q!(vec![10, 20]));
# let balance_reads = process.source_iter(q!(vec![()]));
let (deposit_acks, read_responses) = sliced! {
    let deposit_batch = use(deposits, nondet!(/** deposits are commutative, so batch boundaries don't affect the balance */));
    let read_batch = use(balance_reads, nondet!(/** each read observes the balance at the time it is processed */));
    let mut balance = use::state(|l| l.singleton(q!(0)));

    // Writes are declared first: apply all deposits in this batch...
    let balance_mut = balance.by_mut();
    let deposit_acks = deposit_batch.map(q!(|amt| {
        *balance_mut += amt;
        amt
    }));

    // ...then reads, which observe the fully-updated balance
    let balance_read = balance.by_ref();
    let read_responses = read_batch.map(q!(|_| *balance_read));

    (deposit_acks, read_responses)
};
# read_responses
# }, |mut stream| async move {
# let first = stream.next().await.unwrap();
# assert!(first == 0 || first == 10 || first == 30);
# }));
```

Within each slice, the deposits are applied before the reads because the write closure is declared first, so reads always observe a balance that reflects every deposit in the same batch. Across slices, the balance persists via the state hook.

:::tip

When you need a guarantee that reads observe *previously acknowledged* writes (read-after-write consistency), combine this pattern with [atomic collections](../atomic-collections.mdx): otherwise, an acknowledgement may be released before a subsequent read's slice observes the write.

:::

## Determinism Considerations

Mutable references are imperative escape hatches, and they demand the same care as other non-deterministic patterns:

- **Element order**: mutations run per-element in the order of the batch. For a [`TotalOrder`](../streaming-data/streams.md) stream this order is deterministic; for a `NoOrder` stream, the mutation sequence is not, so the final state is only deterministic if your updates are commutative — the same reasoning required for `fold` with a `commutative` annotation.
- **Batch boundaries**: if outputs depend on *where* batch boundaries fall (for example, reads interleaved with writes across slices), that non-determinism is exactly what the `nondet!` guards on your [slice hooks](./slice-hooks.md) must justify. See [Non-Determinism and `nondet!`](../correctness/nondet.md).
- **Test with the simulator**: the [Hydro simulator](../simulation/index.mdx) explores different batch boundaries and interleavings, which is the best way to validate claims made in your `nondet!` explanations.

You can view the full API documentation for reference handles [here](rust:hydro_lang::handoff_ref).
