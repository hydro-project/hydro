# Ticks and Atomicity
By default, all live collections in Hydro are transformed **asynchronously**, which means that there may be arbitrary delays between when a live collection is updated and when downstream transformations see the updates. This is because Hydro is designed to work in a distributed setting where messages may be delayed. But for some programs, it is necessary to define local iterative loops where transformations are applied atomically; this is achieved with **ticks**.

## Ticks
In some programs, you may want to process batches or snapshots of a live collection in an iterative manner. For example, in a map-reduce program, it may be helpful to compute aggregations on small local batches of data before sending those intermediate results to a reducer.

To create and track such iterative loops, Hydro provides the concept of **ticks**. A **tick** captures the execution of the body of an infinite loop running locally to the machine (importantly, this means that ticks define a [**logical time**](https://en.wikipedia.org/wiki/Logical_clock) which is not comparable across machines). Ticks are non-deterministically generated, so batching data into ticks is an **unsafe** operation that requires special attention.

## Atomicity
In some cases it is necessary to define an atomic section where a set of transformations are guaranteed to be **executed sequentially without interrupts**. For example, in a transaction processing program, it is important that the transaction is applied **before** an acknowledgment is sent to the client.

In Hydro, this can be achieved by placing the transaction and acknowledgment in the same atomic **tick**. Hydro guarantees that all the outputs of a tick will be computed before any are released. Importantly, Hydro's built-in atomic ticks cannot span multiple locations.Distributed atomicity requires distributed coordination protocols (e.g. two-phase commit) that can be built in Hydro, but which have significant performance implications and are not provided by default.

<!--TODO: In future we will provide distributed atomicity constructs in the stdlib -->
