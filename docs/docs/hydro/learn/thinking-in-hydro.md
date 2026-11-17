---
sidebar_position: 2
---

# Thinking in Hydro

Here are some tips for getting started working with Hydro. At the very highest level, Hydro's APIs steer you to write programs that behave the same regardless of network re-orderings and duplication. It does this by enforcing commutativity and idempotence when necessary. That mental model will help you understand Hydro's design.

1. Understand which parts of your distributed system are deliberately non-deterministic. Take 5-10 minutes and identify properties like the following:
   1. A client will choose a server depending on which one responds fastest
   2. The KVS will let arrival order dictate the value that wins the entry
   3. A client may time out (slow network, etc.) and re-subscribe to a live stream
2. Identify invariants. Here are examples that correspond to the cases above:
   1. The client will only connect to one of the servers
   2. All replicas of the KVS will hold the same eventual value
   3. The client must not have any gaps in its log if catchup has concluded
3. Combine the non-determinism and invariants to create [simulation tests](../reference/simulation/writing.mdx) for your system.
   1. These might be as follows:
      1. Send the client a response from server 1 and from server 2. Quiesce, and assert that the client has sent a close to one server and an open to the other.
      2. Start up the KVS and allow it to elect a leader. Send the leader a series of PUT requests, quiesce, and then assert that every replica holds an identical value.
      3. Send messages for a few subscriptions (open, stream, and close messages), several clock ticks (some within, and some out of) the timeout interval(s), then quiesce and validate that the log stream that the client outputs has no gaps. See below about how to model clocks and timeouts.
   2. Simulation tests are an excellent way to build the skeleton for your program. You can think of them as a bounded model checker for your program.
   3. In a simulation test, when you send multiple messages (including across different `Stream`s), the simulator may re-order them. But *when* is that ordering chosen? That is what the points you `.await` (such as `quiesce()` and its family) mark; see [Receiving Outputs and Quiescence](../reference/simulation/writing.mdx#receiving-outputs-and-quiescence).
   4. Be very thoughtful about where you `.await`. Avoid performing `.await` in helper functions, to give the simulator lots of room for interleaving messages. Notice how in the examples, quiescence is used to introduce and set up the search space, but there is no quiescence through the points at which the invariants must hold.
   5. Use a happy-path test to naturally explore many orderings, instead of placing lots of `.await` points that constrain the orderings that may be explored.
4. In many programming models, servers are written as callbacks (e.g. `on_fetch_request`). Hydro requires you to acknowledge that these requests may have been re-ordered and/or duplicated on the network by modeling them as a `Stream<FetchRequest, _, _, NoOrder, AtLeastOnce>` (see [Streams](../reference/streaming-data/streams.md)).
   Create your function signatures by creating a parameter for each of these streams. Most likely, that will be `Stream<GetRequest>` and `Stream<PutRequest>` instead of `Stream<Request>`.
5. When using a clock in Hydro, prefer to model it as a `Stream<Timestamp, _>` (use `NoOrder` if a monotonic clock is not available, so that downstream logic cannot assume a later reading is a later time). It should be delivered as an argument from the [embedding code](../reference/deploy/embedded.mdx) outside your dataflow.

   Timers should be derived from this single source of time, as it allows for more effective simulation testing. Avoid the use of `source_interval` when you have multiple colocated timers because the simulator will not know that they are correlated. The timestamp at which the timer last fired would be stored as [`sliced!` state](../reference/state-management/slices.mdx#state-hooks).
