---
sidebar_position: 0
---

# Bounded and Unbounded Types
Although live collections can be continually updated, some collection types also support **termination**, after which no additional changes can be made. For example, a live collection created by reading integers from an in-memory `Vec` will become terminated once all the elements of the `Vec` have been loaded. But other live collections, such as one being updated by the network, may never become terminated.

In Hydro, certain APIs are restricted to only work on collections that are **guaranteed to terminate** (**bounded** collections). All live collections in Hydro have a type parameter (typically named `B`), which tracks whether the collection is bounded (has the type `Bounded`) or unbounded (has the type `Unbounded`). These types are used in the signature of many Hydro APIs to ensure that the API is only called on the appropriate type of collection.

## Converting Boundedness
In some cases, you may need to convert between bounded and unbounded collections. Converting from a bounded collection **to an unbounded collection** is always allowed and safe, since it relaxes the guarantees on the collection. This can be done by calling `.into()` on the collection.

```rust,no_run
# use hydro_lang::*;
# use dfir_rs::futures::StreamExt;
fn my_unbounded_transformation<'a, L: Location<'a>>(stream: Stream<usize, L, Unbounded>) -> Stream<usize, L, Unbounded> {
    stream.map(q!(|x| x + 1))
}

# let flow = FlowBuilder::new();
# let process = flow.process::<()>();
# let tick = process.tick();
# let numbers = process.source_iter(q!(vec![1, 2, 3, 4]));
# let batch = unsafe { numbers.timestamped(&tick).tick_batch() };
// assume batch is a bounded collection
let unbounded = my_unbounded_transformation(batch.into());
```
