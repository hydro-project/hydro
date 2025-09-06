---
sidebar_position: 0
---

# Bounded and Unbounded Types
Although live collections can be continually updated, some collection types also support **termination**, after which no additional changes can be made. For example, a live collection created by reading integers from an in-memory `Vec` will become terminated once all the elements of the `Vec` have been loaded. But other live collections, such as events streaming into a service from a network, may never become terminated.

In Hydro, certain APIs are restricted to only work on collections that are **guaranteed to terminate** (**bounded** collections). All live collections in Hydro have a type parameter (typically named `B`), which tracks whether the collection is bounded (has the type `Bounded`) or unbounded (has the type `Unbounded`). These types are used in the signature of many Hydro APIs to ensure that the API is only called on the appropriate type of collection.

## Converting Boundedness
In some cases, you may need to convert between bounded and unbounded collections. Converting from a bounded collection **to an unbounded collection** is always allowed and safe, since it relaxes the guarantees on the collection. This can be done by calling `.into()` on the collection.

```rust,no_run
# use hydro_lang::prelude::*;
# use futures::StreamExt;
# let flow = FlowBuilder::new();
# let process = flow.process::<()>();
# let tick = process.tick();
# let numbers = process.source_iter(q!(vec![1, 2, 3, 4]));
let input: Stream<_, _, Bounded> = // ...
#  numbers.batch(&tick, nondet!(/** test */));
let unbounded: Stream<_, _, Unbounded> = input.into();
```

```rust,no_run
# use hydro_lang::prelude::*;
# use futures::StreamExt;
# let flow = FlowBuilder::new();
# let process = flow.process::<()>();
# let tick = process.tick();
let input: Singleton<_, _, Bounded> = tick.singleton(q!(0));
let unbounded: Singleton<_, _, Unbounded> = input.into();
```

Converting from an unbounded collection **to a bounded collection**, however is more complex. This requires cutting off the unbounded collection at a specific point in time, which may not be possible to do deterministically. For example, the most common way to convert an unbounded `Stream` to a bounded one is to batch its elements non-deterministically using `.batch()`. Because this is non-deterministic, this API requires a [non-determinism guard](./determinism.md#unsafe-operations-in-hydro).

```rust,no_run
# use hydro_lang::prelude::*;
# use futures::StreamExt;
# let flow = FlowBuilder::new();
# let process = flow.process::<()>();
let unbounded_input = // ...
#  process.source_iter(q!(vec![1, 2, 3, 4]));
let tick = process.tick();
let batch: Stream<_, _, Bounded> =
  unbounded_input.batch(&tick, nondet!(/** ... */));
```
