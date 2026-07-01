---
slug: multi-version-simulation-testing
title: Multi-version simulation testing
tags: [simulation, testing, distributed-systems]
---

Hydro can now catch mixed-version deployment bugs before they ship.

<!-- truncate -->

## Introduction

One practical consideration for a distributed system is updating the individual components of it. Usually these kinds of systems are meant to be highly available, so taking down the whole system to do an atomic deployment is not an option. This necessarily means that during a deployment the system will have multiple versions of the code running side by side. This mixed-version configuration is difficult to reason about and is often difficult to test. Various frameworks and conventions[^1] have evolved to provide developers with support in dealing with these issues but these frameworks and conventions are narrow in scope and only address some of the challenges that developers may face. In general, gaining confidence that a change is not going to break anything is difficult. Hydro now supports multi-version simulation testing which will allow you to verify that all of your desired system invariants hold, even when multiple versions are live at once.

[^1]: One example is grpc/protobufs, there are specific rules around modifying a .proto file, such as you must not re-use a field number, you must not arbitrarily change a field number, and so on. As long as you follow these rules you will be protected from a number of specific forward and backward compatibility issues.

## Simulation testing

The Hydro framework comes with a deterministic simulation tester. This provides you with a systematic way to explore all the possible states your system can reach and assert that invariants are held in all of those states. The deterministic simulation tester looks for all places in your Hydro program that are non-deterministic and schedules a number of test runs where that non-determinism plays out in every possible way. Let's look at a quick example:

```rust
struct Request {
    value: u32,
}

let mut flow = FlowBuilder::new();
let clients = flow.cluster::<Client>();
let servers = flow.cluster::<Server>();

let output = clients
    .source_iter(q!([Request { value: 0 }]))
    .broadcast(
        &servers,
        TCP.fail_stop().bincode().name("requests"),
        nondet!(/** cluster membership discovery is non-deterministic */),
    )
    .map(q!(|message| message.value ^ 1))
    .values()
    .sim_cluster_output();

flow.sim()
    .with_cluster_size(&clients, 1)
    .with_cluster_size(&servers, 2)
    .exhaustive(async || {
        let c1 = output.collect_sorted::<Vec<_>>(0).await; // collect from the first server in our cluster of 2 servers.
        match c1.as_slice() {
            [] => { /* Message was dropped because Client did not know about Server at the time of broadcast */ }
            [1] => { /* Expected result */ }
            _ => panic!("unexpected output: {:?}", c1),
        }
    });
```

Within the `exhaustive` block we can assert various invariants that will be checked against all (because of `.exhaustive()`) the possible orderings. In the above example, there is a case where the client goes to send the message but has not yet learned about the first server, so the request is not broadcast there and the server produces an empty output. The test passes without hitting the explicit panic, so we know that the only two things that this program is going to do are either emit nothing or emit `[1]`.

## Channels and cross-version communication

In Hydro, cross-location communication is performed over named channels. Taking a look at the previous example program we can see the following snippet:

```rust
TCP.fail_stop().bincode().name("requests"),
```

This line of code configures the channel and also provides it with a user-specified name that is stable across versions of the code. Previously in Hydro there was no need to name the cross-location communication channels, they essentially had automatic identifiers that were generated at compile time. In order to support upgrading a deployed Hydro system, these channels needed to have stable identifiers, so that different versions of the code can point to the same logical endpoint. All cross-location communication channels are now required to be named. The channel name is fairly analogous to the RPC name in a protobuf service definition. For example, the above Hydro program could be re-imagined as the following protobuf service definition:

```proto
message Request {
  uint32 value = 1;
}

service Server {
  rpc requests (Request) returns (???);
}
```

Hydro is not an RPC based system and so there's no way to clearly map the return type, but otherwise the analogy is hopefully illuminating.

## Making changes to a distributed system

Let's consider the example program above and make some changes to it. The difference between the two versions is that in version one `clients` sends a `0` and `servers` xors what it receives with `1` before asserting the result is `1`. Version two flips that, where `clients` will instead send a `1` and `servers` will instead xor the incoming value with a `0` before again asserting that the result is `1`.

```rust {10,16}
struct Request {
    value: u32,
}

let mut flow = FlowBuilder::new();
let clients = flow.cluster::<Client>();
let servers = flow.cluster::<Server>();

let output = clients
    .source_iter(q!([Request { value: 1 }]))
    .broadcast(
        &servers,
        TCP.fail_stop().bincode().name("requests"),
        nondet!(/** cluster membership discovery is non-deterministic */),
    )
    .map(q!(|message| message.value ^ 0))
    .values()
    .sim_cluster_output();

flow.sim()
    .with_cluster_size(&clients, 1)
    .with_cluster_size(&servers, 2)
    .exhaustive(async || {
        let c1 = output.collect_sorted::<Vec<_>>(0).await; // collect from the first server in our cluster of 2 servers.
        match c1.as_slice() {
            [] => { /* Message was dropped because Client did not know about Server at the time of broadcast */ }
            [1] => { /* Expected result */ }
            _ => panic!("unexpected output: {:?}", c1),
        }
    });
```

After this change has been made we run the simulation test and we note that it passes. This is great, the change works. We go to deploy it and suddenly it starts crashing. What went wrong? Unlike a unit test, a real deployment rolling out does not instantly swap all nodes from version one code to version two code. There is likely to be a period where old code and new code are communicating with each other, and moreover, the exact state that the system will be in and the order in which each node will discover their new neighbors is generally non-deterministic and hard to reason about. In this instance, the reason it is crashing is because the new code for both the Client and the Server is backward and forward incompatible. If an old Client communicates with a new Server, or an old Server communicates with a new Client, an incorrect output is going to be produced (a value of 0). This is a contrived example but it is a good motivating example of functional but incompatible change.

## Multi-version simulation tests

What if instead of directly editing the old code, we introduced the new version using the new multi-version simulation testing APIs? We would get a program that looks as follows:

```rust {20,21}
struct Request {
    value: u32,
}

let mut flow = FlowBuilder::new();
let clients = flow.cluster::<Client>();
let servers = flow.cluster::<Server>();

let output = clients
    .source_iter(q!([Request { value: 0 }])) // Old version sends 0
    .broadcast(
        &servers,
        TCP.fail_stop().bincode().name("requests"),
        nondet!(/** cluster membership discovery is non-deterministic */),
    )
    .map(q!(|message| message.value ^ 1)) // Old version xors against 1
    .values()
    .sim_cluster_output();

let clients_v2 = flow.next_version(&clients);
let servers_v2 = flow.next_version(&servers);

clients_v2
    .source_iter(q!([Request { value: 1 }])) // New version sends 1
    .broadcast(
        &servers_v2,
        TCP.fail_stop().bincode().name("requests"),
        nondet!(/** cluster membership discovery is non-deterministic */),
    )
    .map(q!(|message| message.value ^ 0)) // New version xors against 0
    .values()
    .sim_cluster_output();

flow.sim()
    .with_cluster_size(&clients, 0) // No old clients
    .with_cluster_size(&clients_v2, 1) // Single new client
    .with_cluster_size(&servers, 1) // One old server
    .with_cluster_size(&servers_v2, 1) // One new server
    .exhaustive(async || {
        let c1 = output.collect_sorted::<Vec<_>>(0).await; // collect from the old-version server, which received the new client's message.
        match c1.as_slice() {
            [] => { /* Message was dropped because Client did not know about Server at the time of broadcast */ }
            [1] => { /* Expected result */ }
            _ => panic!("unexpected output: {:?}", c1),
        }
    });
```

The only API that has been introduced is `next_version`. It is used to associate two locations together so that the simulator understands that they are different versions of the same code. There are no restrictions on modifications to the topology, because the actual communication paths are stitched together based on matching channel names. Everything else about the simulator remains unchanged. When we run this test now it fails by producing the expected failure:

```
unexpected output: [0]
```

The simulator also produces a trace of decisions it made to show you how it came to that output, so it is easy to debug the behavior exactly.

## Conclusion

During deployments to distributed systems, multiple versions of your code will be running alongside each other. This mixed-version configuration is a big source of difficult-to-reproduce bugs. Hydro now has support for simulating exactly these kinds of system configurations and exploring them systematically, which will allow you to build confidence that what you deploy will actually work.