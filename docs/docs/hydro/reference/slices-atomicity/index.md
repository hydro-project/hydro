# Slices and Atomicity
By default, all live collections in Hydro are transformed **asynchronously**, which means that there may be arbitrary delays between when a live collection is updated and when downstream transformations see the updates. This is because Hydro is designed to work in a distributed setting where messages may be delayed. But for some programs, it is necessary to observe snapshots of asynchronous state or define consistency relationships between outputs; this is achieved with **slices** and **atomicity**.

## Slice Blocks
In many programs, you need to process batches of a live collection while observing the current state of another collection. For example, when handling get requests for a counter service, you need to observe the current count at the time each request is processed.

Hydro provides the [`sliced!`](./slices.mdx) macro for this purpose. It allows you to perform computations that rely on a _version_ of several live collections at some point in time. The way this version is revealed depends on the type of live collection:
- A `Stream` will be revealed as a **batch** of new elements at that point in time
- A `Singleton` will be revealed as a **snapshot** of the current state
- A `KeyedStream` will be revealed as a **batch** of new elements per key
- A `KeyedSingleton` will be revealed as a **snapshot** of the current state per key

Slicing is inherently non-deterministic because the boundaries of batches and the timing of snapshots depend on runtime factors like network delays. This is why all `use` statements in `sliced!` require a [`nondet!`](../live-collections/determinism.md#unsafe-operations-in-hydro) marker.

## Atomic Collections
In some cases, it is necessary to establish **consistency relationships** between different outputs of your program. For example, in a counter service, when a client receives an acknowledgement for an increment, subsequent get requests should observe the updated count.

Hydro provides [atomic collections](./atomicity.mdx) to establish these consistency guarantees. By marking a stream as atomic with `.atomic()`, you can ensure that downstream snapshots taken with `use::atomic` are **consistent with respect to** the outputs released from that atomic collections.

Importantly, Hydro's built-in atomicity cannot span multiple locations. Distributed atomicity requires distributed coordination protocols (e.g., two-phase commit) that can be built in Hydro, but have significant performance implications and will not be introduced without explicit intent.
