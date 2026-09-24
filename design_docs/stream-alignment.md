# Stream Alignment

_significant contributions from @davidchuyaya_

This design proposes a replacement for the `.atomic()` set of APIs with an "alignment" API that
- better matches how existing distributed systems reason about concurrent execution
- simplifies the type system for locations and associated cognitive overhead
- reduces the use of "ticks" across several different patterns

## Background: Atomic Processing with Ticks
In Hydro, all operators operating on an `Unbounded` (stream-y) collection at the top level (outside a `Tick`, so on a `Process` or `Cluster`) execute with asynchronous semantics as described in Flo. A simple mental model for this asynchrony is to introduce buffers between all adjacent operators. Then, the scheduler is free to run operators in a dataflow graph in any order.

This asynchrony encourages a form of "coordination-free" programming where the developer does not rely on synchronous data processing. It also gives significant power to optimizers, which can take advantage of the implicit asynchrony to split computation over the network. But it can lead to somewhat surprising behavior for stateful services.

Consider a simple application with two streaming inputs (one for incrementing a counter and one for querying it) and two streaming outputs (one acknowledging an increment and one yielding a query response). To track the counter, we can use a fold over the stream of increment requests.

```rust
let increment: KeyedStream<ClientId, (), Process, Unbounded> = ...
let query: KeyedStream<ClientId, (), Process, Unbounded> = ...
let current_count: Singleton<usize, Process, Unbounded> =
    increment.values().fold_commutative(q!(|| 0), q!(|c, _| *c += 1));
```

The result of this fold is an unbounded singleton, which captures an asynchronously updating value that will change over time as new increment requests are processed. To process queries, we need to look up the current counter value. Because the counter is asynchronously changing, we cannot directly read the value (part of Hydro's definition of safety is not permitting programs to read transient state). Instead, Hydro _correctly_ points out that responding to queries involves non-determinism, since we will pick a non-deterministic "version" of the counter to use when responding to each query.

To explicitly write out this non-deterministic logic of picking a set of queries and a version of the counter, we will use a `Tick`, which lets us identify a logical point in time at which we can capture a `snapshot()` of the counter (a non-deterministically picked version) and a `batch()` of the queries (with a non-deterministic boundary). We can then respond to each batch of queries with the corresponding counter snapshot.

```rust
let query_response_tick = process.tick();
let query_responses =
    query.batch(&query_response_tick, nondet!(/** ... */)) // KeyedStream<ClientId, (), Tick<Process>, Bounded>
        .entries().map(q!(|(cid, _)| cid)) // Stream<ClientId, Tick<Process>, Bounded>
        .cross_singleton(
            current_count.snapshot(&query_response_tick, nondet!(/** ... */)), // Singleton<usize, Process, Bounded>
        ) // Stream<(ClientId, usize), Tick<Process>, Bounded>
        .into_keyed() // KeyedStream<ClientId, usize, Tick<Process>, Bounded>
        .all_ticks(); // KeyedStream<ClientId, usize, Process, Unbounded>
```

> [!NOTE]
> While the use of ticks here is *also* confusing and unfortunate, it isn't the focus of this design document. Be on the lookout for additional design proposals to address further usability issues with ticks.

We also need to send out acknowledgements for each increment request. A naive (and natural) approach is to `clone()` the stream of increment requests so that each request is used to both update the counter and generate an acknowledgement. The updated code might look like this:

```rust
let current_count: Singleton<usize, Process, Unbounded> =
    increment.clone().fold(q!(|| 0), q!(|c, _| *c += 1));
let increment_ack = increment; // just echo back the increment request as an ack
```

Now, if we were to write a hypothetical simulator test, we might write a simple one like:
```rust
fn test_increment() {
    simulator::fuzz_dst(async |sim| {
        // ... setup simulated inputs
        let (send_increment, increment) = ...;
        let (send_query, query) = ...;
        // ...
        let recv_increment_ack = ...;
        let recv_query_response = ...;
        // end setup

        send_increment.send((..., ()));
        recv_increment_ack.recv().await.unwrap();
        send_query.send((..., ()));
        assert_eq!(
            recv_query_response.recv().await.unwrap(),
            (..., 1)
        );
    })
}
```

If we were to run this test with our imaginary (soon-to-exist) simulator, we would get a FAILURE! If we engineer the simulator right, we would see a test failure like this:
```
thread 'test_increment' panicked at 'assertion failed: `(..., 0) == (..., 1)`'
Simulation Steps:
- *****
    - `query.batch(&query_response_tick, nondet!(/** ... */))`: yield [(..., ())]
    - `current_count.snapshot(&query_response_tick, nondet!(/** ... */))`: yield 0
```

What's going on? We got an acknowledgement from the increment, but somehow it doesn't show up in the query result. The error log is helpful; we see that the snapshot for the `current_count` was 0 (when we would have expected it to be 1). The issue is that `snapshot` returns _some_ version of the asynchronously changing value. Furthermore, when we `.clone()` the `increment` stream, each downstream copy will propagate the elements independently, so the ack can be sent before any of the increments are processed into the counter!

How might we fix this? We might be tempted to use a `Tick` to wrap _both_ the counting and ack logic into a shared tick. Currently, ticks guarantee atomic processing of the entire nested graph before any outputs are released. This will help avoid the violation we observed earlier.

First, to bring increment requests from the top level into the tick, we need to use `.batch()` to define the window of requests that will be processed on each tick. To emit acks, we simply invert this to yield values from the tick using `.all_ticks()`.
```rust
let process_increment_tick = process.tick();
let increment_batch = increment.batch(&process_increment_tick, nondet!(/** ... */));
let increment_ack = increment_batch.all_ticks();
```

Next, we need to update our counter _inside_ the tick, so that the update is synchronous with the ack. Now, if we were to just `fold` the batch, it would (naturally) return the number of increments in the _batch_. But we want to get the accumulated counter value across _all ticks_. There are two ways to do this:
1) Ticks are useful for iterative compute. We can send the value of the counter to the next tick, and initialize the fold in the next tick with that value.
2) Instead of folding over the batch, we fold over the entire prefix of elements, which we can get using `.persist()`.

Let's do Option #2 for now, since it requires the least code:
```rust
let current_count: Singleton<usize, Tick<Process>, Bounded> =
    increment.values().persist().fold_commutative(q!(|| 0), q!(|c, _| *c += 1));
```

The `.persist()` API is problematic for serveral reasons:
- Confusing naming: there is no disk persistence involved, just in-memory accumulation
- Confusing performance characteristics: if used in the wrong place, `.persist()` leads to linear memory growth as all elements over time are stored. But Hydro optimizes away this accumulation if the following operator is something like `.fold()` (by disabling the code that resets the accumulator). _When_ this optimization happens is unpredictable to the developer. This means that sometimes they might expect a `.persist()` to be optimized away, but it's not. Which is bad.

We'll come back to these issues in just a second. The last piece of our code is handling lookups for queries. Our counter is now materialized _inside_ a `Tick`, so we already have a snapshot, and there is no need to call `.snapshot` again. A developer might try to to patch the code using `.latest()`, which pulls a singleton _outside_ a tick into an unbounded singleton which is asynchronously updated as ticks are processed.

```rust
let query_responses =
    query.batch(&query_response_tick, nondet!(/** ... */)) // KeyedStream<ClientId, (), Tick<Process>, Bounded>
        .entries().map(q!(|(cid, _)| cid)) // Stream<ClientId, Tick<Process>, Bounded>
        .cross_singleton(
            current_count // Singleton<usize, Tick<Process>, Bounded>
                .latest() // Singleton<usize, Process, Unbounded>
                .snapshot(&query_response_tick, nondet!(/** ... */)), // Singleton<usize, Process, Bounded>
        ) // Stream<(ClientId, usize), Tick<Process>, Bounded>
        .into_keyed() // KeyedStream<ClientId, usize, Tick<Process>, Bounded>
        .all_ticks(); // KeyedStream<ClientId, usize, Process, Unbounded>
```

But this code will have the same bug as before, as there is asynchrony between the `.latest()` and `.snapshot()`. To ensure that queries are handled with a counter that is consistent with the acks, we need to use the _same tick_ for both processing increments and responding to queries. So our code will look more like:
```rust
let query_responses =
    query.batch(&process_increment_tick, nondet!(/** ... */)) // KeyedStream<ClientId, (), Tick<Process>, Bounded>
        .entries().map(q!(|(cid, _)| cid)) // Stream<ClientId, Tick<Process>, Bounded>
        .cross_singleton(
            current_count // Singleton<usize, Tick<Process>, Bounded>
        ) // Stream<(ClientId, usize), Tick<Process>, Bounded>
        .into_keyed() // KeyedStream<ClientId, usize, Tick<Process>, Bounded>
        .all_ticks(); // KeyedStream<ClientId, usize, Process, Unbounded>
```

This code will (finally) work as expected!

### Improving Safety with `.atomic()`
While this code is correct, the changes we've had to make reveal several issues.
1. We introduced one new use of `batch()` for the increment acks. Each `nondet!()` comes with a massive cognitive burden for the developer and weakens Hydro's safety story.
2. We had to use `.persist()` (or worse, manual iterative state) inside the tick to accumulate the counter over batches. As discussed, `.persist()` has confusing semantics and unpredictable performance. Manually maintaining the state across ticks would have predictable performance, but requires much more code.
3. We entangled seemingly-unrelated ticks for increment processing and queries. This is necessary to process queries with consistent views of the counter, but it is strange to use the `process_increment_tick` to batch queries.

The primary issue with the current design is that ticks are used for several (seemingly unrelated) reasons:
1) To ensure atomic processing of elements
2) To define "slices" of time where we can read snapshots / batches of asynchronously updated state / streams
3) To define iterative stateful computations with cycles across ticks

Our first attempt to address this issue is the `.atomic()` operator, which allows developers to declare a region where elements are atomically processed before any outputs are yielded. This allows developers to continue manipulating unbounded streams (so `.persist` is no longer required) while guaranteeing atomic processing. To define an atomic section, we use `.atomic(tick)` on an asynchronous (`Unbounded`) stream. We will see why it is helpful to pass in an explicit tick in a second. In our example, we must use `.atomic(tick)` _before_ the `.clone()`, since we want both the acks and counter branches to process synchronously.

```rust
let increment: KeyedStream<ClientId, (), Process, Unbounded> = ...
let query: KeyedStream<ClientId, (), Process, Unbounded> = ...
let process_increment_tick = process.tick();
let increment_atomic: KeyedStream<ClientId, (), Atomic<Process>, Unbounded> = increment.atomic(&process_increment_tick);
let current_count: Singleton<usize, Atomic<Process>, Unbounded> =
    increment_atomic.clone().values().fold_commutative(q!(|| 0), q!(|c, _| *c += 1));
let increment_ack = increment_atomic.end_atomic();
```

Why does the location type change to `Atomic<Process>` if semantically it is the same as on `Process`? Atomic regions inhibit optimizations that pipeline / decouple the dataflow graph. We want developers to minimize the size of their atomic sections, so we keep the `Atomic` tag to remind them of this. To get back out of the atomic, you must call `.end_atomic()` explicitly.

Notice that we got rid of the use of `.batch()` for the increment acks, which is great! When processing queries, we need to make sure we are using the same consistent snapshot of the count. Given an atomically updated singleton, we have two APIs:
- `.snapshot(nondet!(...))`, which _does not_ take a tick, but instead outputs a snapshot inside the tick that the atomic section was initialized with
    - this API guarantees that the snapshot is always consistent with respect to the rest of the atomic section
    - it still requires a `nondet!(...)` parameter because the code will _observe_ a specific version of the singleton, which is still non-deterministic
- `.end_atomic().snapshot(tick, nondet!(...))` uses the same snapshot API as above, but arbitrary asynchrony is introduced at the `.end_atomic()`, so this will cause the same bugs as earlier

This is why atomic sections are initialized with a tick: the tick defines where the atomic updates can be consistently batched / snapshotted without introducing asynchrony. So when responding to queries, we must still batch the queries into the same tick as the atomic section:
```rust
let query_responses =
    query.batch(&process_increment_tick, nondet!(/** ... */)) // KeyedStream<ClientId, (), Tick<Process>, Bounded>
        .entries().map(q!(|(cid, _)| cid)) // Stream<ClientId, Tick<Process>, Bounded>
        .cross_singleton(
            current_count.snapshot(nondet!(/** ... */)) // Singleton<usize, Tick<Process>, Bounded>
        ) // Stream<(ClientId, usize), Tick<Process>, Bounded>
        .into_keyed() // KeyedStream<ClientId, usize, Tick<Process>, Bounded>
        .all_ticks(); // KeyedStream<ClientId, usize, Process, Unbounded>
```

This code will behave correctly, with one less `nondet!(...)` compared to earlier!

## What's Wrong with Atomic
Now that we've introduced the current Hydro APIs for synchronous processing, let's discuss the issues.

1. **There are multiple ways to use ticks to solve the same problem.** For example, we can do atomic processing with either `.atomic()` or manual batching. Both of these require creating a tick, so it's very confusing which version developers should use and when. A common point of confusion is the _difference_ between ticks and atomic, since they seem to do the same thing
2. **Atomic is viral in the type system.** Because we want to avoid larger atomic sections, all atomically processed collections have an `Atomic<...>` location. But this makes the types complicated and is particularly painful to deal with when writing utility helpers. For example, quorums in `hydro_std` have an awkward type-signature in order to handle atomic processing.
3. **Atomic sections do not solve tick entanglement.** We still have to use `process_increment_tick` to batch queries, which is confusing since queries are a conceptually separate task.

All of these share one common thread: **synchronous processing should be a separate concept from ticks**. This is in alignment with our high-level goal of splitting out the roles of ticks into more specific interfaces. Furthermore, in line with othe Hydro principles, atomicity is an _implementation concern_, but not the end-goal. The end-goal is that we should be able to snapshot asynchronous state in a manner that is **consistent** with other streaming outputs that rely on shared inputs.

## Proposal: Stream Alignment
Our proposal is to introduce a separate API for declaring points in Hydro program where streams / batches / snapshots that rely on a shared asynchronous collection should be **aligned**. At such an **alignment point**, Hydro's semantics will guarantee that these aligned collections will emit values based on the same version of any shared upstream **on the same machine**:
- If the shared collection is a stream, the aligned collections will emit values based on the same prefix of streaming input
- If the shared collection is a singleton (/ optional), the aligned collections will emit values based on the same version of the singleton

Let's see this in example with our working example. We'll start with the buggy program that a developer naively wrote:
```rust
let increment: KeyedStream<ClientId, (), Process, Unbounded> = ...
let query: KeyedStream<ClientId, (), Process, Unbounded> = ...

let current_count: Singleton<usize, Process, Unbounded> =
    increment.clone().fold(q!(|| 0), q!(|c, _| *c += 1));
let increment_ack = increment; // just echo back the increment request as an ack

let query_response_tick = process.tick();
let query_responses =
    query.batch(&query_response_tick, nondet!(/** ... */)) // KeyedStream<ClientId, (), Tick<Process>, Bounded>
        .entries().map(q!(|(cid, _)| cid)) // Stream<ClientId, Tick<Process>, Bounded>
        .cross_singleton(
            current_count.snapshot(&query_response_tick, nondet!(/** ... */)), // Singleton<usize, Process, Bounded>
        ) // Stream<(ClientId, usize), Tick<Process>, Bounded>
        .into_keyed() // KeyedStream<ClientId, usize, Tick<Process>, Bounded>
        .all_ticks(); // KeyedStream<ClientId, usize, Process, Unbounded>
```

As we recall, the issue is that the snapshot of the `current_count` may emit a version of the count based on a different prefix of the `increment` input compared to what is sent with `increment_ack`, so even after receiving an ack we may get a query response that does not capture that increment. What we want is to make sure that the snapshot of `current_count` is _in alignment with_ what is emitted in the acks.

We can do this by first declaring an `LocalAlignPoint`:
```rust
let consistent_count_align = process.local_align_point(); // LocalAlignPoint<Process>
```

> [!NOTE]
> **More Explicit Design:**
>
> Then, we need to declare _which_ upstream values will be aligned at this point. In our case, we want alignment for the increment stream, so we call `aligned_at` to declare that this stream will be aligned when accessed at that point.
>
> ```rust
> let increment = increment.aligned_at(&consistent_count_align);
> ```

Then we can use this align point to ensure we are getting a consistent snapshot. We do this with a `snapshot_aligned` API that behaves like `snapshot` but guarantees that the emitted value is consistent with respect to the alignment point (we will also offer a similar `batch_aligned` API for streams):
```rust
let query_responses =
    query.batch(&query_response_tick, nondet!(/** ... */)) // KeyedStream<ClientId, (), Tick<Process>, Bounded>
        .entries().map(q!(|(cid, _)| cid)) // Stream<ClientId, Tick<Process>, Bounded>
        .cross_singleton(
            current_count.snapshot_aligned(&query_response_tick, &consistent_count_align, nondet!(/** ... */)), // Singleton<usize, Process, Bounded>
        ) // Stream<(ClientId, usize), Tick<Process>, Bounded>
        .into_keyed() // KeyedStream<ClientId, usize, Tick<Process>, Bounded>
        .all_ticks(); // KeyedStream<ClientId, usize, Process, Unbounded>
```

And finally, we need to make sure that the acknowledgement is aligned with the count. We do this with a `release_aligned` API on streams that only releases elements _after_ the other uses of the alignment point are up-to-date (i.e. all future snapshots of `current_count` incorporate the request):
```rust
let increment_ack = increment.release_aligned(&consistent_count_align);
```

This improves over the previous implementation in several ways:
1) We no longer need to declare the beginning of the `.atomic()` section, instead we automatically infer it by finding all common ancestors of the alignment point
2) We can continue to use the `query_response_tick` to respond to queries, since the use of ticks for (still non-deterministic) co-incidence is now orthogonal to synchronous processing
3) The APIs direct attention to the parts of logic that require alignment. If we jump to rereferences for `consistent_count_align`, we will see all the places that require a consistent snapshot.

### Semantics with Distribution
Local alignment points (created with `.local_align`) only guarantee alignment with respect to shared streams on the same machine, and make no guarantees about behavior across the network. If a shared upstream is independently sent over the network to two different locations, the alignment point cannot be used to synchronize across these locations (this will result in a type error, since a `LocalAlignPoint` is associated with a specific location).

If the alignment point is used on a cluster, the guarantees only apply locally on _each_ cluster member, and makes no guarantees across members. For example, if you broadcast data across a cluster, alignment points will not guarantee any synchronization across these machines.

To guard against potential confusion, we alert developers to this behavior in several places:
- We name the API `.local_align` to make it clear that the alignment point only applies locally (not across machines)
- We disallow use of the alignment point across different locations with the type system
- If there are no shared upstreams of an alignment point on the same machine, we throw an error since the alignment point will not do anything
    - If there is a shared upstream behind a network, we additionally provide guidance to the user that they cannot align across the network

> [!NOTE]
> **More Explicit Design:**
>
> With explicitly annotated shared upstreams for alignnment points, we can further guard against accidental use across the network in a cluster. Without explicit annotations, there may be situations where an alignment point has one shared upstream across the network and one local upstream, in which case we would align on the local upstream and not throw any errors (since this could be intentional to semi-align).
>
> With explicit annotations, if the shared upstream across the network is annotated with the same align point, we can instead throw an error, since we can precisely detect an attempt to align over the network.

### Compilation
How do we actually run this? Since the underlying DFIR model only offers ticks to define atomic sections, we need to compile this code back into using ticks. We can do this in a couple steps. For each alignment point, we:
1. Find all IR nodes that are upstream (transitive inputs) of at least two instances of the alignment point (and whose downstream nodes are _not_ shared by the same set of instances)
    - IR nodes upstream of a network node do not count as shared, since we only guarantee _local_ alignment
2. Identify all ticks that are used in `batch_aligned` or `snapshot_aligned` with that alignment point. If there are multiple ticks, merge them together so that all the code associated with both are emitted into the same tick (nested subgraph) in DFIR.
3. When emitting DFIR, all the shared IR nodes from step #1 will be compiled into the merged tick found above

> [!NOTE]
> **More Explicit Design:**
>
> In step 1, only consider the IR nodes that were explicitly tagged with `.align_at`. Effectively, we do not rely on a tracing algorithm, but instead look at regions between an `.align_at` and use of the same alignment point.

> [!NOTE]
> A careful reading of the above steps provides some intuition on when alignment is necessary. If **either** of the following are true, an alignment point acts as a no-op:
> - There are no shared IR nodes upstream of the alignment point
> - There are no instances of `batch_aligned` or `snapshot_aligned` that use the alignment point
>     - Equivalently: The only use of the alignment point is `release_aligned`
> - There are no instances of `release_aligned` AND only one instance of `batch_aligned` | `snapshot_aligned`

This API is exactly as expressive as the `.atomic()` API, but is lighterweight and more closely matches the mental model when thinking about consistency. While it shifts additional burden to the compiler (which now has to trace common ancestors), it does not result in unpredictable performance since atomic sections do not require any additional runtime behavior in DFIR (it only affects the simulator and opportunities for optimization through pipelining / decoupling).

## FAQs
### Why not have the atomic section bounded by a block / closure of code, so that it appears indented with clear boundaries?
I think the tendency towards a closure / block arises from a bit of historical mental model. We've been thinking of this in terms of atomic compute sections that execute in a single tick on a single machine, with the effect being that all downstreams will see the same version of data.

The new API **does away with atomic sections entirely**. The user is not expected to think about atomicity, nor are they exposed in any way to atomic computation. Instead this API provides consistency guarantees when asking for a version of some asynchronously updated data in multiple places. The necessary atomic region is inferred (in fact, there may be methods other than atomic regions for achieving the same result!). As a result, it's not clear where the beginning of a closure / block would need to be.

The other reason for the non-block approach is it's common to have semantically separate pieces of logic that need to share a consistent view of some shared data. For example different request handlers that need to read the same key. Ideally, these separate components can be in different Rust functions, but that means that we can't have a "block" around the portions of each function that deal with the shared data. Instead, the token approach lets us pass in a "token that points to some consistent but non-deterministic version of the data".

For co-incidence and loops, the latter situation is far less common; the entirety of a loop is almost always in a single function. So there I think we will want to move towards a block / closure approach (but that is for a separate RFC).
