# Locations and Networking
Hydro is a **global**, **distributed** programming model. This means that the data and computation in a Hydro program can be spread across multiple machines, data centers, and even continents. To achieve this, Hydro uses the concept of **locations** to keep track of _where_ data is stored and computation is executed.

Each live collection type (`Stream`, `Singleton`, etc.) has a type parameter `L` which will always be a type that implements the `Location` trait (e.g. `Process` and `Cluster`, documented in this section). Most Hydro APIs that transform live collections will emit a new live collection with the same location type as the input, and APIs that consume multiple live collections will require them all to have the same location type.

To create distributed programs, live collections can be sent over the network using a variety of APIs. For example, `Stream`s can be sent from a process to another process using `.send_bincode(&loc2)` (which uses [bincode](https://docs.rs/bincode/latest/bincode/) as a serialization format). The sections for each location type discuss the networking APIs in further detail.

## Creating Locations
Locations can be created by calling the appropriate method on the global `FlowBuilder` (e.g. `flow.process()` or `flow.cluster()`). These methods will return a handle to the location that can be used to create live collections and run computations.

:::caution

It is possible to create **different** locations that still have the same type, for example:

```rust
# use hydro_lang::*;
let flow = FlowBuilder::new();
let process1: Process<()> = flow.process::<()>();
let process2: Process<()> = flow.process::<()>();

assert_ne!(process1, process2);
# let _ = flow.compile_no_network::<deploy::MultiGraph>();
```

These locations will not be unified and may be deployed to separate machines. When deploying a Hydro program, additional runtime checks will be performed to ensure that input locations match.

```rust
# use hydro_lang::*;
let flow = FlowBuilder::new();
let process1: Process<()> = flow.process::<()>();
let process2: Process<()> = flow.process::<()>();

# test_util::assert_panics_with_message(|| {
process1.source_iter(q!([1, 2, 3]))
    .cross_product(process2.source_iter(q!([1, 2, 3])));
// PANIC: assertion `left == right` failed: locations do not match
# }, "assertion `left == right` failed: locations do not match");
# let _ = flow.compile_no_network::<deploy::MultiGraph>();
```

:::
