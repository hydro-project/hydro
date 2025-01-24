---
sidebar_position: 0
---

# Processes
The simplest type of location in Hydro is a process. A process represents a single machine running a piece of a Hydro program. When creating a process, you can pass in a type parameter that will be used as a marker to distinguish that process from others (and will also be used to mark logs originating at that process). For example, you can create a process with a marker of `Leader` to represent a leader in a distributed system:

```rust,no_run
# use hydro_lang::*;
struct Leader {}

let flow = FlowBuilder::new();
let leader: Process<Leader> = flow.process::<Leader>();
```

Once we have a process, we can create live collections on that process (see [Live Collections](../live-collections/index.md) for more details). For example, we can create a stream of integers on the leader process:

```rust,no_run
# use hydro_lang::*;
# struct Leader {}
# let flow = FlowBuilder::new();
# let leader: Process<Leader> = flow.process::<Leader>();
let numbers = leader.source_iter(q!(vec![1, 2, 3, 4]));
```

## Networking
Because a process represents a single machine, it is straightforward to send data to and from a process. For example, we can send a stream of integers from the leader process to another process using the `send_bincode` method (which uses [bincode](https://docs.rs/bincode/latest/bincode/) as a serialization format). This automatically sets up network senders and receivers on the two processes.

```rust,no_run
# use hydro_lang::*;
# struct Leader {}
# let flow = FlowBuilder::new();
# let leader: Process<Leader> = flow.process::<Leader>();
let numbers = leader.source_iter(q!(vec![1, 2, 3, 4]));
let process2: Process<()> = flow.process::<()>();
let on_p2: Stream<_, Process<()>, _> = numbers.send_bincode(&process2);
```
