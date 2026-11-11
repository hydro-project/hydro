---
sidebar_position: 2
---

# Thinking in Hydro

Here are some tips for getting started working with Hydro. At the very highest level, Hydro's APIs steer you to write programs that behave the same regardless of network re-orderings and duplication. It does this by enforcing commutativity and idempotence when necessary. That mental model will help you understand Hydro's design.

1. Understand which parts of your distributed system are deliberately non-deterministic. Take 5-10 minutes and identify properties like the following:
   1. A client will choose a server depending on which one responds fastest
   2. The KVS will let arrival order dictate the value that wins the entry
   3. Duplicate messages will be allowed to double-increment a requests metric
   4. A client may time out and switch to a different server
2. Identify invariants. Here are examples that correspond to the cases above:
   1. The client will only connect to one of the servers
   2. All replicas of the KVS will hold the same eventual value
   3. The requests metric will account for connections that time out
   4. The client must enter an `Idle` state between the two connections
3. Turn these into [simulation tests](../reference/simulation/writing.mdx) for your system. Simulation tests are an excellent way to build the skeleton for your program. You can think of them as a bounded model checker for your program.
   1. In a simulation test, when you send multiple messages (including across different `Stream`s), the simulator may re-order them. But *when* is that ordering chosen? That is what the points you `.await` (such as `quiesce()` and its family) mark; see [Receiving Outputs and Quiescence](../reference/simulation/writing.mdx#receiving-outputs-and-quiescence).
   2. Be very thoughtful about where you `.await`. Avoid performing `.await` in helper functions, to give the simulator lots of room for interleaving messages.
   3. Use a happy-path test to naturally explore many orderings, instead of placing lots of `.await` points that constrain the orderings that may be explored.
4. In many programming models, servers are written as callbacks (e.g. `on_fetch_request`). Hydro requires you to acknowledge that these requests may have been re-ordered and/or duplicated on the network by modeling them as a `Stream<FetchRequest, _, _, NoOrder, AtLeastOnce>` (see [Streams](../reference/streaming-data/streams.md)).
   Create your function signatures by creating a parameter for each of these streams. Most likely, that will be `Stream<GetRequest>` and `Stream<PutRequest>` instead of `Stream<Request>`.
