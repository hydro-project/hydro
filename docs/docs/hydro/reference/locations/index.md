# Locations and Networking
Hydro is a **global**, **distributed** programming model. This means that the data and computation in a Hydro program can be spread across multiple machines, data centers, and even continents. To achieve this, Hydro uses the concept of **locations** to keep track of _where_ data is located and computation is executed.

Each [live collection](pathname:///rustdoc/hydro_lang/live_collections/) has a type parameter `L` which will always be a type that implements the `Location` trait (e.g. [`Process`](./processes.md) and [`Cluster`](./clusters.md), documented in this section). Computation has to happen at a single place, so Hydro APIs that consume multiple live collections will require all inputs to have the same location type. Moreover, most Hydro APIs that transform live collections will emit a new live collection output with the same location type as the input.

To create distributed programs, Hydro provides a variety of APIs to _move_ live collections between locations via network send/receive. For example, `Stream`s can be sent from one process to another process using `.send(&loc2, ...)`. The sections for each location type ([`Process`](./processes.md), [`Cluster`](./clusters.md)) discuss the networking APIs in further detail.

## Creating Locations
Locations can be created by calling the appropriate method on the global `FlowBuilder` (e.g. `flow.process()` or `flow.cluster()`). These methods will return a handle to the location that can be used to create live collections and run computations.

<!-- TODO(shadaj): provide documentation on FlowBuilder and link from the mention above -->

:::caution

It is possible to create **different** locations that still have the same type, for example:

```rust
# use hydro_lang::prelude::*;
let flow = FlowBuilder::new();
let process1: Process<()> = flow.process::<()>();
let process2: Process<()> = flow.process::<()>();

assert_ne!(process1, process2);
# let _ = flow.with_default_optimize::<hydro_lang::deploy::HydroDeploy>();
```

These locations will not be unified and may be deployed to separate machines. When deploying a Hydro program, additional runtime checks will be performed to ensure that input locations match.

```rust
# use hydro_lang::prelude::*;
let flow = FlowBuilder::new();
let process1: Process<()> = flow.process::<()>();
let process2: Process<()> = flow.process::<()>();

# hydro_lang::test_util::assert_panics_with_message(|| {
process1.source_iter(q!([1, 2, 3]))
    .cross_product(process2.source_iter(q!([1, 2, 3])));
// PANIC: assertion `left == right` failed: locations do not match
# }, "assertion `left == right` failed: locations do not match");
# let _ = flow.with_default_optimize::<hydro_lang::deploy::HydroDeploy>();
```

:::
