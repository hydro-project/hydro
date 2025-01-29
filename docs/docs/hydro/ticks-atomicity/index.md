# Ticks and Atomicity
By default, all live collections in Hydro are transformed **asynchronously**, which means that there may be arbitrary delays between when a live collection is updated and when downstream transformations see the updates. This is because Hydro is designed to work in a distributed setting where messages may be delayed. But for some programs, it is necessary to define local iterative loops where transformations are applied atomically; this is achieved with **ticks**.

## Loops
In some programs, you may want to process batches or snapshots of a live collection in an iterative manner. For example, in a map-reduce program, it may be helpful to compute aggregations on small local batches of data before sending those intermediate results to a reducer.

To create such iterative loops, Hydro provides the concept of **ticks**. A **tick** captures the body of an infinite loop running locally to the machine (importantly, this means that ticks define a **logical time** which is not comparable across machines). Ticks are non-deterministically generated, so batching data into ticks is an **unsafe** operation that requires special attention.

## Atomicity
In other programs, it is necessary to define an atomic section where a set of transformations are guaranteed to be executed **all at once**. For example, in a transaction processing program, it is important that the transaction is applied **before** an acknowledgment is sent to the client.

In Hydro, this can be achieved by placing the transaction and acknowledgment in the same atomic **tick**. Hydro guarantees that all the outputs of a tick will be computed before any are released. Importantly, atomic ticks cannot span several locations, since that would require a locking mechanism that has significant performance implications.
