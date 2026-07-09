# State Management
Nearly every service maintains **state**: counters, caches, session data, or replicated logs. In Hydro, state is not stored in mutable variables or external databases; instead, state is represented by live collections — [Singletons and Optionals](./singletons-optionals.md) for single values and [Keyed State](./keyed-state.md) for per-key values — that are **derived** from the streams flowing through your program.

Because state in Hydro is a live collection, it updates automatically as new inputs arrive, and the type system tracks how it can be safely observed. This section covers the tools for defining, reading, and mutating state.

## Deriving State with Aggregations
The preferred way to create state is with **declarative aggregations** such as `fold`, `reduce`, and `count` (or their keyed equivalents like `KeyedStream::fold` and `value_counts`). These APIs consume a stream and produce a `Singleton` (or `KeyedSingleton`) whose value is continuously updated as elements arrive.

```rust,no_run
# use hydro_lang::prelude::*;
# let mut flow = FlowBuilder::new();
# let process = flow.process::<()>();
# let increment_requests: Stream<(), Process<_>, Unbounded> = process.source_iter(q!(vec![(), ()])).into();
let current_count = increment_requests.count(); // Singleton<usize>
```

Aggregations are **deterministic**: no matter how inputs are batched or delayed, the state eventually converges to the same value. Whenever your state can be expressed as an aggregation, prefer this style, since it requires no [non-determinism guards](../correctness/nondet.md) and preserves [eventual determinism](../correctness/determinism.md) end-to-end.

## Reading State
State derived from asynchronous inputs is `Unbounded`: its value changes over time, so it cannot be directly observed without picking a *specific version* to look at. To read state — for example, to answer get requests in a counter service — you take a **snapshot** inside a [`sliced!`](./slices.mdx) block, which processes a batch of requests against a consistent snapshot of the state:

```rust,ignore
let get_response = sliced! {
    let request_batch = use(get_requests, nondet!(/** batch boundaries are not observable */));
    let count_snapshot = use(current_count, nondet!(/** requests may see any valid version of the count */));

    let count_ref = count_snapshot.by_ref();
    request_batch.map(q!(|_| *count_ref))
};
```

Inside the slice, the snapshot is a *bounded* singleton, so you can capture a [reference](./references-mutations.md) to it with `by_ref()` and read it inside `q!()` closures like a regular Rust value. The [Slice Hooks](./slice-hooks.md) page documents how each type of live collection is revealed inside a slice.

:::caution

Snapshots are **asynchronous** by default: a snapshot may lag behind the inputs that produced the state. If clients require read-after-write consistency (e.g., a get request that follows an acknowledged increment must observe the increment), use [Atomic Collections](../atomic-collections.mdx) to establish that guarantee.

:::

## Custom State with Mutable References
Some state cannot be expressed as an aggregation of a single stream, such as when **multiple streaming inputs** need to read and write the same value with interleaved effects. For these cases, `sliced!` supports declaring state that persists across slices (`use::state`), and [mutable references](./references-mutations.md) (`by_mut()`) allow imperative-style updates from within `q!()` closures:

```rust,ignore
let results = sliced! {
    let deposit_batch = use(deposits, nondet!(/** ... */));
    let withdrawal_batch = use(withdrawals, nondet!(/** ... */));
    let mut balance = use::state(|l| l.singleton(q!(0)));

    let balance_mut = balance.by_mut();
    let deposit_results = deposit_batch.map(q!(|amt| { *balance_mut += amt; *balance_mut }));
    let withdrawal_results = withdrawal_batch.map(q!(|amt| { *balance_mut -= amt; *balance_mut }));
    deposit_results.chain(withdrawal_results)
};
```

This is the most powerful — and least protected — way to manage state, so it should be used sparingly and covered with [simulation tests](../simulation/index.mdx). When a single input stream fully determines the state, a `fold` expresses the same logic with stronger guarantees.

## In This Section
- **[Singletons and Optionals](./singletons-optionals.md)**: the core live collections for single-value state
- **[Keyed State](./keyed-state.md)**: per-key state with `KeyedSingleton`, like a live `HashMap`
- **[Slice Blocks](./slices.mdx)**: processing batches of requests against snapshots of state with `sliced!`
- **[Slice Hooks](./slice-hooks.md)**: the `use` hooks available inside `sliced!` and their semantics
- **[References and Mutations](./references-mutations.md)**: reading and mutating state from `q!()` closures with `by_ref()` and `by_mut()`
