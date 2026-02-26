---
sidebar_position: 3
---

# Network Configuration

When sending data between locations in Hydro (e.g. via `.send()`, `.broadcast()`, or `.demux()`), you must provide a networking configuration that specifies the transport, fault tolerance policy, and serialization format. These are configured by specifying a network protocol and using a builder-style API.

A typical networking configuration looks like:

```rust,no_run
# use hydro_lang::prelude::*;
# let mut flow = FlowBuilder::new();
# let p1 = flow.process::<()>();
# let p2 = flow.process::<()>();
let numbers: Stream<i32, Process<_>, Bounded> = p1.source_iter(q!(vec![1, 2, 3]));
let on_p2: Stream<i32, Process<_>, Unbounded> = numbers.send(&p2, TCP.fail_stop().bincode());
```

The networking config reads as: use TCP transport, with a fail-stop fault model, serialized with bincode.

## Naming Network Channels

Network channels can be configured with `.name("my_channel")`, which assigns a stable name to the network channel. This is required when you are **versioning** your Hydro service—named channels allow different versions of your code to communicate with each other, since the channel identity is tied to the name rather than to the compiled program structure. If you are not using versioning, naming is optional but can still be useful for debugging.

```rust,no_run
# use hydro_lang::prelude::*;
let config = TCP.fail_stop().name("heartbeat").bincode();
```

## Serialization

Serialization configures how data is encoded and decoded when sent over the network. The Hydro compiler will automatically generate the appropriate sender / receiver logic for your configured serialization format, so you don't need to worry about encoding details or making sure that types match on both sides of the network channel. However, you do need to make sure that the types being sent implement the appropriate traits for the chosen serialization format (e.g. `Serialize` and `DeserializeOwned` for `bincode`).

### Bincode

The `.bincode()` API configures the channel to use the [`bincode`](https://docs.rs/bincode) crate for serialization and deserialization. The types being sent must implement `Serialize` and `DeserializeOwned`. This is currently the only supported serialization backend.

## TCP

TCP is currently the only transport backend. When using `TCP`, you **must** choose a fault tolerance policy before configuring serialization. Calling `TCP.bincode()` directly will result in a compile error—you need to first call either `.fail_stop()` or `.lossy()`.

### Fail-Stop

```rust,no_run
# use hydro_lang::prelude::*;
let config = TCP.fail_stop().bincode();
```

With `fail_stop`, the channel guarantees that the recipient receives a **prefix** of the sent messages in order. If the TCP connection drops, no further messages will be delivered, but all messages received up to that point are valid and in the correct order.

This is the most common choice and is appropriate when your application can tolerate a connection permanently going down (e.g. a cluster member that is treated as permanently failed if any of its network channels are disconnected).

`fail_stop` is **deterministic** in the sense that the received messages are always a prefix of the sent messages—there are no reorderings or duplications. Hydro's type system prevents downstream users from blocking on network outputs (unless they explicitly use a `nondet!`), so network failures on a fail-stop connection are indistinguishable from a slow network.

:::note
The Hydro simulator will not simulate connection failures that block **liveness** (i.e., it won't cause a test to hang). However, it will still catch **safety** issues caused by connection failures, such as race conditions between a dropped connection and other messages.
:::

### Lossy

```rust,no_run
# use hydro_lang::prelude::*;
let config = TCP.lossy(nondet!(/** messages may be dropped, explanation... */)).bincode();
```

With `lossy`, messages may be **arbitrarily dropped**. Unlike `fail_stop`, there is no guarantee that a prefix of messages is delivered—any individual message may be lost. But the network connection can still be used to send future messages, even after a message loss.

:::caution

The `lossy` fault model is currently only available for [embedded deployments](../deploy/embedded.mdx) and Maelstrom testing. Support in Hydro Deploy will be available in the near future.

:::

This is appropriate for protocols that are designed to tolerate message loss, such as gossip protocols or systems running under network partition testing (e.g. [Maelstrom](https://github.com/jepsen-io/maelstrom)).

Because message loss is non-deterministic, `lossy` requires a `nondet!` marker to make this explicit in your code. You should document why your protocol is correct despite potential message loss.
