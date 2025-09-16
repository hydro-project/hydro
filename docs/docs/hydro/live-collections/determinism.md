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

These non-deterministic APIs take additional parameters called **non-determinism guards**. These values, with type `NonDet`, help you reason about how non-determinism affects your application. To pass a non-determinism guard, you must invoke `nondet!()` with an explanation for how the non-determinism affects the application. If the non-determinism is never exposed outside the function, or if it appears at the root of your application (and thus is documented in the service guarantees), you should use `nondet!()` with only a single parameter containing the explanation.

```rust,no_run
# use hydro_lang::prelude::*;
use std::time::Duration;

fn singleton_with_delay<T, L>(
  singleton: Singleton<T, Process<L>, Unbounded>
) -> Optional<T, Process<L>, Unbounded> {
  singleton
    .sample_every(q!(Duration::from_secs(1)), nondet!(/** non-deterministic samples will eventually resolve to deterministic result */))
    .last()
}
```

When writing a function with Hydro that involves non-deterministic APIs, it is important to be extra careful about whether the non-determinism is exposed externally. In some applications, a utility function may involve local non-determinism (such as sending retries), but not expose it outside the function (e.g., via deduplicating received responses).

If the outputs of your code are non-deterministic, you should take non-determinism guards and document this behavior in the Rustdoc. Then, when invoking APIs whose non-determinism propagates to the outputs of your code, you should use `nondet!` passing in an explanation as well the relevant guard parameter.

```rust
# use hydro_lang::prelude::*;
use std::fmt::Debug;
use std::time::Duration;

/// ...
///
/// # Non-Determinism
/// - `nondet_samples`: this function will non-deterministically print elements
///   from the stream according to a timer
fn print_samples<T: Debug, L>(
  stream: Stream<T, Process<L>, Unbounded>,
  nondet_samples: NonDet
) {
  stream
    .sample_every(q!(Duration::from_secs(1)), nondet!(
      /// non-deterministic timing will result in non-determistic samples printed
      nondet_samples
    ))
    .assume_retries(nondet!(
      /// non-deterministic duplicated logs are okay
      nondet_samples
    ))
    .for_each(q!(|v| println!("Sample: {:?}", v)))
}
```

## User-Defined Functions
Another source of potential non-determinism comes from user-defined functions or closures, such as those provided to `map` or `filter`. Hydro allows for arbitrary Rust functions to be called inside these closures, so it is possible to introduce non-determinism that will not be checked by the compiler.

In general, avoid using APIs like random number generators inside transformation functions unless that non-determinism is explicitly documented somewhere.

:::info

To help avoid such bugs, we are working on ways to use formal verification tools (such as [Kani](https://model-checking.github.io/kani/)) to check arbitrary Rust code for properties such as determinism and more. This remains active research for now and is not yet available.

:::
