---
sidebar_position: 2
---

# Eventual Determinism
Many programs benefit from strong guarantees on **determinism**, the property that when provided the same inputs, the outputs of the program are always the same. This is critical for consistency across *replicated* services. It is also extremely helpful for predictability, including consistency across identical runs (e.g. for testing or reproducibility.) Determinism is particularly tricky to reason about in distributed systems due to the inherently non-deterministic nature of asynchronous event arrivals. However, even when the inputs and outputs of a program are live collections, we can focus on the _eventual_ state of the collection — as if we froze the input and waited until the output stopped changing.

:::info

Our consistency and safety model is based on the POPL'25 paper [Flo: A Semantic Foundation for Progressive Stream Processing](https://arxiv.org/abs/2411.08274), which covers the formal details and proofs underlying this system.

:::

Hydro's type system is thus focused on **eventual determinism**: given a set of specific live collections as inputs, the outputs of the program will **eventually** have the same _final_ value. All safe APIs in Hydro preserve this property, and the operations that cannot are explicitly marked. Eventual determinism makes it easy to build composable blocks of code without having to worry about non-deterministic runtime behavior such as batching or network delays.

:::note

Much existing literature in distributed systems focuses on data consistency levels such as "eventual consistency". These typically correspond to guarantees when reading the state of a _replicated_ object (or set of objects) at a _specific point_ in time. Hydro does not use such a consistency model internally, instead focusing on the values local to each distributed location _over time_. Concepts such as replication, however, can be layered on top of this model.

:::

## Where Non-Determinism Can Appear
All **safe** APIs in Hydro (the ones you can call regularly in Rust) guarantee eventual determinism. But real systems often need behavior that is inherently non-deterministic, such as generating events on a wall-clock timer or processing an input in arbitrarily sized batches. Hydro requires such APIs to take explicit **non-determinism guards**, so every source of non-determinism in your program is visibly marked in the code.

See [Non-Determinism and `nondet!`](./nondet.md) for how these guards work and how to reason about non-determinism in your applications.

