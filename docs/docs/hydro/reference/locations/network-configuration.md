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

The `.bincode()` API configures the channel to use the [`bincode`](https://docs.rs/bincode) crate for serialization and deserialization. The types being sent must implement `Serialize` and `DeserializeOwned`. Bincode is currently the only built-in wire format — the one where Hydro performs the encoding and decoding for you — and it works on all deployment backends.

### Embedded

The `.embedded()` API configures the channel to leave serialization to code **outside of Hydro**. Instead of Hydro serializing elements to bytes, the raw element type `T` is exposed at the network boundary: the generated sender side hands you `T` values directly, and the receiver side accepts a stream of `T` values. You are then responsible for encoding, transporting, and decoding the elements yourself—for example with a custom wire format, a specialized transport like DPDK or shared memory, or an existing messaging layer.

```rust,no_run
# use hydro_lang::prelude::*;
let config = TCP.fail_stop().embedded().name("messages");
```

Embedded serialization works with any transport configuration, including [UDP](#udp):

```rust,no_run
# use hydro_lang::prelude::*;
let config = UDP.lossy_delayed_forever().embedded().name("messages");
```

Note that with embedded serialization, the transport and fault policy describe a **contract** rather than an implementation: since your external code carries the data, *you* must ensure your transport upholds the declared guarantees (e.g. in-order, prefix delivery for `TCP.fail_stop()`; for `UDP` channels, messages may be dropped or reordered, and Hydro's type system will treat the received stream accordingly, as `NoOrder`).

Because there is no transport managed by Hydro, the receiving side gets the raw payload with no transport `Result` to unwrap—your external code decides how to handle serialization and delivery faults before feeding elements into the receiver.

:::caution

Embedded serialization is only supported in [embedded deployments](../deploy/embedded.mdx) (where you wire up network channels manually) and the Hydro simulator (where raw values are carried directly through in-memory channels). Attempting to use `.embedded()` with other deployment backends, such as Hydro Deploy or Maelstrom, will panic at compile time; use `.bincode()` there instead.

:::

See [Embedded Mode: Embedded Serialization](../deploy/embedded.mdx#embedded-serialization) for details on the generated code.

## TCP

TCP is one of two available transport backends (see also [UDP](#udp)). When using `TCP`, you **must** choose a fault tolerance policy before configuring serialization. Calling `TCP.bincode()` directly will result in a compile error—you need to first call `.fail_stop()`, `.lossy_delayed_forever()`, or `.lossy()`.

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

### Lossy Delayed Forever

```rust,no_run
# use hydro_lang::prelude::*;
let config = TCP.lossy_delayed_forever().bincode();
```

With `lossy_delayed_forever`, messages may be **dropped**, but dropped messages are modeled as being **indefinitely delayed** rather than lost. This mode does **not** require a `nondet!` annotation because the output stream is unordered, so even if messages are lost the output will have a subset of the intended elements.

The tradeoff is that the output stream has a [`NoOrder`](rust:hydro_lang::live_collections::stream::NoOrder) guarantee, imposing stricter conditions on downstream consumers. For example, you cannot use order-dependent operators like `fold` without proving commutativity.

This is the **preferred** mode for protocols that tolerate message loss, because:
- It does not require `nondet!`, making it easier to reason about correctness.
- It can be easily simulated in exhaustive mode without running into fairness issues, so you can write simulator tests for your protocol.

:::note

When using `lossy_delayed_forever` in the Hydro simulator, you must call `.test_safety_only()` on the simulation:

```rust,ignore
flow.sim().test_safety_only().exhaustive(async || { /* ... */ });
```

This is required because the simulator will not actually drop packets—instead, it delays "dropped" messages until the end of the execution. This catches **safety** bugs (such as race conditions where a message arrives later than expected), but cannot test **liveness**: a message that is "delayed forever" may never arrive in a real deployment, so the simulator cannot guarantee that your program will eventually make progress—only that it won't produce incorrect results.

:::

:::caution

The `lossy_delayed_forever` fault model is currently available for [embedded deployments](../deploy/embedded.mdx) (the only production deployment option), Maelstrom testing, and the Hydro simulator (with `.test_safety_only()`). Support in Hydro Deploy will be available once TCP reconnect is implemented.

:::

This is appropriate for gossip protocols, retransmission-based protocols, or any system running under network partition testing (e.g. [Maelstrom](https://github.com/jepsen-io/maelstrom)).

### Lossy

```rust,no_run
# use hydro_lang::prelude::*;
let config = TCP.lossy(nondet!(/** messages may be dropped, explanation... */)).bincode();
```

With `lossy`, messages may be **arbitrarily dropped**. Unlike `fail_stop`, there is no guarantee that a prefix of messages is delivered—any individual message may be lost. But the network connection can still be used to send future messages, even after a message loss.

:::tip

In most cases, prefer [`lossy_delayed_forever`](#lossy-delayed-forever) over `lossy`. The `lossy_delayed_forever` mode does not require `nondet!` and can be simulated in exhaustive mode, making it much easier to test. Use `lossy` only if you specifically need to preserve the ordering guarantee of the input stream (since `lossy` preserves `TotalOrder` while `lossy_delayed_forever` weakens to `NoOrder`).

:::

:::caution

The `lossy` fault model is currently available for [embedded deployments](../deploy/embedded.mdx) (the only production deployment option) and Maelstrom testing. It is **not supported in the Hydro simulator**—use `lossy_delayed_forever` if you want to simulate message loss. Support in Hydro Deploy will be available in the near future.

:::

This is appropriate for protocols that are designed to tolerate message loss, such as gossip protocols or systems running under network partition testing (e.g. [Maelstrom](https://github.com/jepsen-io/maelstrom)).

Because message loss is non-deterministic, `lossy` requires a `nondet!` marker to make this explicit in your code. You should document why your protocol is correct despite potential message loss.

## UDP

UDP is a connectionless transport that guarantees **neither delivery nor ordering**. Output streams from a UDP channel always have a [`NoOrder`](rust:hydro_lang::live_collections::stream::NoOrder) guarantee, imposing stricter conditions on downstream consumers (e.g. you cannot use order-dependent operators like `fold` without proving commutativity).

Like `TCP`, you **must** choose a fault tolerance policy before configuring serialization. Because UDP is connectionless, there is no connection that can fail, so there is **no `fail_stop` policy**—only `.lossy_delayed_forever()` and `.lossy()` are available.

### Lossy Delayed Forever

```rust,no_run
# use hydro_lang::prelude::*;
let config = UDP.lossy_delayed_forever().bincode();
```

With `lossy_delayed_forever`, dropped messages are modeled as being **indefinitely delayed** rather than lost. Like the TCP mode of the same name, this does **not** require a `nondet!` annotation because the output stream is unordered, so even if messages are lost the output will have a subset of the intended elements.

This is the **preferred** UDP mode, for the same reasons as [TCP's `lossy_delayed_forever`](#lossy-delayed-forever): it does not require `nondet!` and can be simulated in exhaustive mode (with `.test_safety_only()`).

### Lossy

```rust,no_run
# use hydro_lang::prelude::*;
let config = UDP.lossy(nondet!(/** messages may be dropped, explanation... */)).bincode();
```

With `lossy`, messages may be **arbitrarily dropped and reordered**. Because message loss is non-deterministic, this requires a `nondet!` marker to make it explicit in your code.

Unlike TCP's `lossy` mode, UDP's `lossy` mode does **not** preserve the ordering of the input stream—the output is always `NoOrder`.

:::caution

UDP is **not yet available** in "deploy" deployment mode (via Hydro Deploy, including Docker and ECS deployments)—attempting to deploy a UDP channel there will panic at compile time.

Both UDP modes are available for [embedded deployments](../deploy/embedded.mdx) (the only production deployment option) and Maelstrom testing. In the Hydro simulator, only `lossy_delayed_forever` is supported, and it requires `.test_safety_only()`: the simulator will not actually drop packets—it delays "dropped" messages until the end of the execution, which catches safety bugs but cannot test liveness.

:::
