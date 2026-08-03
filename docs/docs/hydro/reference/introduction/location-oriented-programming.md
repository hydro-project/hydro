---
sidebar_position: 2
---

# Location-Oriented Programming
In most distributed frameworks, each machine in the system is written as its own program: one service lives in one codebase, another in a separate handler or actor definition, and the network calls that tie them together are buried inside RPC stubs or message handlers. The end-to-end logic of the system (the _protocol_) is scattered across all these pieces and is never visible in one place.

Hydro takes a different approach, called **location-oriented programming**: the code for an _entire distributed system_, spanning many machines, is written in a **single function**. Instead of splitting logic into separate programs, Hydro uses **locations**, values such as `Process` and `Cluster`, to describe _where_ each piece of data lives and each piece of computation runs.

## One Function, Many Machines
Consider a simple monitoring system with a sensor machine that emits readings and a server that filters them for alerts. In Hydro, both machines are described together:

```rust
# use hydro_lang::prelude::*;
# use futures::StreamExt;
# tokio_test::block_on(hydro_lang::test_util::multi_location_test(|flow, p_out| {
struct Sensor {}
struct Server {}
let sensor: Process<Sensor> = flow.process::<Sensor>();
let server: Process<Server> = flow.process::<Server>();

// this code will run on the sensor machine
let readings = sensor.source_iter(q!(vec![78, 79, 81]));

// `send` moves the readings across the network to the server
let readings_on_server = readings.send(&server, TCP.fail_stop().bincode());

// this code will run on the server
let alerts = readings_on_server.filter(q!(|r| *r > 80));
# alerts.send(p_out, TCP.fail_stop().bincode())
# }, |mut stream| async move {
# assert_eq!(stream.next().await, Some(81));
# }));
```

Even though this is one function, it does **not** run on a single machine, and there is no central coordinator at runtime. The location types tell the Hydro compiler where each operator executes, so it can split the program into a separate, efficient binary for each location, with network senders and receivers automatically inserted wherever data crosses machines (such as the `send` call above).

Hydro provides a few kinds of locations, each described in detail in [Locations and Networking](../locations/index.md):
- **[Process](../locations/processes.md)**: a single thread running on a single machine, with no internal concurrency
- **[Cluster](../locations/clusters.md)**: a group of members all running the _same_ logic (Single-Program-Multiple-Data), used for scale-out patterns like sharding and replication; the number of members is chosen at deployment time
- **[External](../locations/external-clients.md)**: a process _outside_ the Hydro program, such as a client, which interacts with the system over network ports

Locations are created by calling methods on the global `FlowBuilder` (e.g. `flow.process()` or `flow.cluster()`), each with a marker type that distinguishes it from other locations.

## The Network is Explicit
Every [live collection](./live-collections.md) type carries the location it lives on as a type parameter. For example, `Stream<i32, Process<Sensor>>` and `Stream<i32, Process<Server>>` are **different types**. Transformations like `map` and `filter` run where the data is, producing a new collection at the same location.

Because computation happens at a single place, APIs that combine multiple live collections require all inputs to be at the same location. If you forget to move data across the network, your program simply will not compile:

```compile_fail
# use hydro_lang::prelude::*;
# let mut flow = FlowBuilder::new();
struct Sensor {}
struct Server {}
let sensor: Process<Sensor> = flow.process::<Sensor>();
let server: Process<Server> = flow.process::<Server>();

let readings = sensor.source_iter(q!(vec![78, 79, 81]));
let thresholds = server.source_iter(q!(vec![80]));

thresholds.cross_product(readings);
// ERROR: expected `Process<'_, Server>`, found `Process<'_, Sensor>`
```

The fix is to explicitly move one side across the network first, such as `readings.send(&server, TCP.fail_stop().bincode())`. Networking APIs like `send` require you to pick a transport and serialization format, so every network boundary (a potential source of latency, reordering, and failure) is visible right in the code. The _effects_ of the network are surfaced in the types too: for example, messages arriving from many cluster members have no deterministic order across members, which is reflected in the type of the received collection (see [Live Collections and Correctness](./live-collections.md#live-collections-and-correctness)).

## Distributed Systems as Functions
Because locations are just values and live collections are just types, distributed patterns become ordinary Rust functions. A function can take locations and live collections as parameters and internally set up computation and networking that spans machines:

```rust,ignore
/// Broadcasts commands from the leader to all replicas, returning
/// the stream of commands received at each replica.
pub fn replicate<'a>(
    replicas: &Cluster<'a, Replica>,
    commands: Stream<Command, Process<'a, Leader>>,
) -> Stream<Command, Cluster<'a, Replica>> {
    commands.broadcast(replicas, TCP.fail_stop().bincode(), nondet!(/** membership is stable */))
}
```

Callers get distributed behavior, in this case a network broadcast from one machine to a whole cluster, just by calling a function. This is the foundation for building reusable distributed components, from heartbeats and timeouts to full protocols like two-phase commit and Paxos, that can be dropped into any Hydro application.

:::note

Location-oriented programming is closely related to [**choreographic programming**](https://en.wikipedia.org/wiki/Choreographic_programming), which also describes the behavior of many machines in a single program. The key difference is the execution model: choreographies are typically written as a _sequential_ program, where each step happens "after" the previous one across the whole system. Hydro instead uses an _asynchronous_ dataflow model, where each location processes its live collections independently and data flows between machines without global sequencing.

:::
