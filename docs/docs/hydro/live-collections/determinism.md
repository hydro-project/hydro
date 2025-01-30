---
sidebar_position: 1
---

# Eventual Determinism
Many programs benefit from strong guarantees on **determinism**, the property that when provided the same inputs, the outputs of the program are always the same. This is critical for consistency across *replicated* services. It is also extremely helpful for predictability, including consistency across identical runs (e.g. for testing or reproducibility.) Determinism is particularly tricky to reason about in distributed systems due to the inherently non-deterministic nature of asynchronous event arrivals. However, even when the inputs and outputs of a program are live collections, we can focus on the _eventual_ state of the collection — as if we froze the input and waited until the output stopped changing.

:::info

Our consistency and safety model is based on the POPL'25 paper [Flo: A Semantic Foundation for Progressive Stream Processing](https://arxiv.org/abs/2411.08274), which covers the formal details and proofs underlying this system.

:::

Hydro thus guarantees **eventual determinism**: given a set of specific live collections as inputs, the outputs of the program will **eventually** have the same _final_ value. Eventual determinism makes it easy to build composable blocks of code without having to worry about non-deterministic runtime behavior such as batching or network delays.

:::note

Much existing literature in distributed systems focuses on data consistency levels such as "eventual consistency". These typically correspond to guarantees when reading the state of a _replicated_ object (or set of objects) at a _specific point_ in time. Hydro does not use such a consistency model internally, instead focusing on the values local to each distributed location _over time_. Concepts such as replication, however, can be layered on top of this model.

:::

## Unsafe Operations in Hydro
All **safe** APIs in Hydro (the ones you can call regularly in Rust) guarantee determinism. But often it is necessary to do something non-deterministic, like generate events at a fixed wall-clock-time interval, or split an input into arbitrarily sized batches.

Hydro offers APIs for such concepts behind an **`unsafe`** guard. This keyword is typically used to mark Rust functions that may not be memory-safe, but we reuse this in Hydro to mark APIs with non-deterministic constructs.

To call such an API, the Rust compiler will ask you to wrap the call in an `unsafe` block. It is typically good practice to also include a `// SAFETY: ...` comment to explain why the non-determinism is there.

```rust,no_run
# use hydro_lang::*;
# let flow = FlowBuilder::new();
# let stream_inputs = flow.process::<()>().source_iter(q!([123]));
use std::time::Duration;

unsafe {
  // SAFETY: intentional non-determinism
  stream_inputs
    .sample_every(q!(Duration::from_secs(1)))
}.for_each(q!(|v| println!("Sample: {:?}", v)))
```

When writing a function with Hydro that involves `unsafe` code, it is important to be extra careful about whether the non-determinism is exposed externally. In some applications, a utility function may involve local non-determinism (such as sending retries), but not expose it outside the function (e.g., via deduplicating received responses).

But other functions may expose the non-determinism, in which case they should be marked `unsafe` as well. If the function is public, Rust will require you to put a `# Safety` section in its documentation to explain the non-determinism.

```rust
# use hydro_lang::*;
use std::fmt::Debug;
use std::time::Duration;

/// ...
///
/// # Safety
/// This function will non-deterministically print elements
/// from the stream according to a timer.
unsafe fn print_samples<T: Debug, L>(
  stream: Stream<T, Process<L>, Unbounded>
) {
  unsafe {
    // SAFETY: documented non-determinism
    stream
      .sample_every(q!(Duration::from_secs(1)))
  }.for_each(q!(|v| println!("Sample: {:?}", v)))
}
```

## User-Defined Functions
Another source of potential non-determinism comes from user-defined functions or closures, such as those provided to `map` or `filter`. Hydro allows for arbitrary Rust functions to be called inside these closures, so it is possible to introduce non-determinism that will not be checked by the compiler.

In general, avoid using APIs like random number generators inside transformation functions unless that non-determinism is explicitly documented somewhere.

:::info

To help avoid such bugs, we are working on ways to use formal verification tools (such as [Kani](https://model-checking.github.io/kani/)) to check arbitrary Rust code for properties such as determinism and more. This remains active research for now and is not yet available.

:::
