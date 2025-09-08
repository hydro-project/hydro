# Locations and Networking
Hydro is a **global**, **distributed** programming model. This means that the data and computation in a Hydro program can be spread across multiple machines, data centers, and even continents. To achieve this, Hydro uses the concept of **locations** to keep track of _where_ data is located and computation is executed.

Each live collection type ([`Stream`](https://hydro.run/rustdoc/hydro_lang/stream/struct.Stream), [`Singleton`](https://hydro.run/rustdoc/hydro_lang/singleton/struct.Singleton) or [`Optional`](https://hydro.run/rustdoc/hydro_lang/optional/struct.Optional)) has a type parameter `L` which will always be a type that implements the `Location` trait (e.g. [`Process`](./processes) and [`Cluster`](./clusters), documented in this section). Computation has to happen at a single place, so Hydro APIs that consume multiple live collections will require all inputs to have the same location type. Moreover, most Hydro APIs that transform live collections will emit a new live collection output with the same location type as the input.

To create distributed programs, Hydro provides a variety of API calls to allow live collections to be sent over the network. For example, `Stream`s can be sent from one process to another process using `.send_bincode(&loc2)` (which uses [bincode](https://docs.rs/bincode/latest/bincode/) as a serialization format). The sections for each location type ([`Process`](./processes), [`Cluster`](./clusters)) discuss the networking APIs in further detail.

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
