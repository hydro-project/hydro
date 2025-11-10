---
sidebar_position: 1
---

# Clusters
When building scalable distributed systems in Hydro, you'll often need to use **clusters**, which represent groups of threads all running the _same_ piece of your program (Single-Program-Multiple-Data, or "SPMD"). Hydro clusters can be used to implement scale-out systems using techniques such as sharding or replication. Unlike processes, the number of threads in a cluster does not need to be static, and can be chosen during deployment.

Like when creating a process, you can pass in a type parameter to a cluster to distinguish it from other clusters. For example, you can create a cluster with a marker of `Worker` to represent a pool of workers in a distributed system:

```rust,no_run
# use hydro_lang::prelude::*;
struct Worker {}

let flow = FlowBuilder::new();
let workers: Cluster<Worker> = flow.cluster::<Worker>();
```

You can then instantiate a live collection on the cluster using the same APIs as for processes. For example, you can create a stream of integers on the worker cluster. If you launch this program, **each** member of the cluster will create a stream containing the elements 1, 2, 3, and 4:

```rust,no_run
# use hydro_lang::prelude::*;
# struct Worker {}
# let flow = FlowBuilder::new();
# let workers: Cluster<Worker> = flow.cluster::<Worker>();
let numbers = workers.source_iter(q!(vec![1, 2, 3, 4]));
```

## Networking
When sending a live collection from a cluster to another location, **each** member of the cluster will send its local collection. On the receiver side, these collections will be joined together into a **keyed stream** of with `ID` keys and groups of `Data` values where the ID uniquely identifies which member of the cluster the data came from. For example, you can send a stream from the worker cluster to another process using the `send_bincode` method:

```rust
# use hydro_lang::prelude::*;
# use futures::StreamExt;
# tokio_test::block_on(hydro_lang::test_util::multi_location_test(|flow, process| {
# let workers: Cluster<()> = flow.cluster::<()>();
let numbers: Stream<_, Cluster<_>, _> = workers.source_iter(q!(vec![1]));
numbers.send_bincode(&process) // KeyedStream<MemberId<()>, i32, ...>
# .entries()
# }, |mut stream| async move {
// if there are 4 members in the cluster, we should receive 4 elements
// { MemberId::<Worker>(0): [1], MemberId::<Worker>(1): [1], MemberId::<Worker>(2): [1], MemberId::<Worker>(3): [1] }
# let mut results = Vec::new();
# for w in 0..4 {
#     results.push(format!("{:?}", stream.next().await.unwrap()));
# }
# results.sort();
# assert_eq!(results, vec!["(MemberId::<()>(0), 1)", "(MemberId::<()>(1), 1)", "(MemberId::<()>(2), 1)", "(MemberId::<()>(3), 1)"]);
# }));
```

:::tip

If you do not need to know _which_ member of the cluster the data came from, you can use the `values()` method on the keyed stream, which will drop the IDs at the receiver:

```rust
# use hydro_lang::prelude::*;
# use futures::StreamExt;
# tokio_test::block_on(hydro_lang::test_util::multi_location_test(|flow, process| {
# let workers: Cluster<()> = flow.cluster::<()>();
let numbers: Stream<_, Cluster<_>, _> = workers.source_iter(q!(vec![1]));
numbers.send_bincode(&process).values()
# }, |mut stream| async move {
// if there are 4 members in the cluster, we should receive 4 elements
// 1, 1, 1, 1
# let mut results = Vec::new();
# for w in 0..4 {
#     results.push(format!("{:?}", stream.next().await.unwrap()));
# }
# results.sort();
# assert_eq!(results, vec!["1", "1", "1", "1"]);
# }));
```

:::

In the reverse direction, when sending a stream _to_ a cluster, the sender must prepare `(ID, Data)` tuples, where the ID uniquely identifies which member of the cluster the data is intended for. Then, we can send a stream from a process to the worker cluster using the `demux_bincode` method:

```rust
# use hydro_lang::prelude::*;
# use futures::StreamExt;
# tokio_test::block_on(hydro_lang::test_util::multi_location_test(|flow, p2| {
# let p1 = flow.process::<()>();
# let workers: Cluster<()> = flow.cluster::<()>();
let numbers: Stream<_, Process<_>, _> = p1.source_iter(q!(vec![0, 1, 2, 3]));
let on_worker: Stream<_, Cluster<_>, _> = numbers
    .map(q!(|x| (hydro_lang::location::MemberId::from_raw_id(x), x)))
    .demux_bincode(&workers);
on_worker.send_bincode(&p2)
# .entries()
// if there are 4 members in the cluster, we should receive 4 elements
// { MemberId::<Worker>(0): [0], MemberId::<Worker>(1): [1], MemberId::<Worker>(2): [2], MemberId::<Worker>(3): [3] }
# }, |mut stream| async move {
# let mut results = Vec::new();
# for w in 0..4 {
#     results.push(format!("{:?}", stream.next().await.unwrap()));
# }
# results.sort();
# assert_eq!(results, vec!["(MemberId::<()>(0), 0)", "(MemberId::<()>(1), 1)", "(MemberId::<()>(2), 2)", "(MemberId::<()>(3), 3)"]);
# }));
```

## Broadcasting and Membership Lists
A common pattern in distributed systems is to broadcast data to all members of a cluster. In Hydro, this can be achieved using `broadcast_bincode`, which takes in a stream of **only data elements** and broadcasts them to all members of the cluster. For example, we can broadcast a stream of integers to the worker cluster:

```rust
# use hydro_lang::prelude::*;
# use futures::StreamExt;
# tokio_test::block_on(hydro_lang::test_util::multi_location_test(|flow, p2| {
# let p1 = flow.process::<()>();
# let workers: Cluster<()> = flow.cluster::<()>();
let numbers: Stream<_, Process<_>, _> = p1.source_iter(q!(vec![123]));
let on_worker: Stream<_, Cluster<_>, _> = numbers.broadcast_bincode(&workers, nondet!(/** assuming stable membership */));
on_worker.send_bincode(&p2)
# .entries()
// if there are 4 members in the cluster, we should receive 4 elements
// { MemberId::<Worker>(0): [123], MemberId::<Worker>(1): [123], MemberId::<Worker>(2): [123, MemberId::<Worker>(3): [123] }
# }, |mut stream| async move {
# let mut results = Vec::new();
# for w in 0..4 {
#     results.push(format!("{:?}", stream.next().await.unwrap()));
# }
# results.sort();
# assert_eq!(results, vec!["(MemberId::<()>(0), 123)", "(MemberId::<()>(1), 123)", "(MemberId::<()>(2), 123)", "(MemberId::<()>(3), 123)"]);
# }));
```

This API requires a [non-determinism guard](../live-collections/determinism.md#unsafe-operations-in-hydro), because the set of cluster members may asynchronously change over time. Depending on when we are notified of membership changes, we will broadcast to different members. Under the hood, the `broadcast_bincode` API uses a list of members of the cluster provided by the deployment system. To manually access this list, you can use the `source_cluster_members` method to get a stream of membership events (cluster members joining or leaving):

```rust
# use hydro_lang::prelude::*;
# use futures::StreamExt;
# tokio_test::block_on(hydro_lang::test_util::multi_location_test(|flow, p2| {
let p1 = flow.process::<()>();
let workers: Cluster<()> = flow.cluster::<()>();
# // do nothing on each worker
# workers.source_iter(q!(vec![])).for_each(q!(|_: ()| {}));
let cluster_members = p1.source_cluster_members(&workers);
# cluster_members.entries().send_bincode(&p2)
// if there are 4 members in the cluster, we would see a join event for each
// { MemberId::<Worker>(0): [MembershipEvent::Join], MemberId::<Worker>(2): [MembershipEvent::Join], ... }
# }, |mut stream| async move {
# let mut results = Vec::new();
# for w in 0..4 {
#     results.push(format!("{:?}", stream.next().await.unwrap()));
# }
# results.sort();
# assert_eq!(results, vec!["(MemberId::<()>(0), Joined)", "(MemberId::<()>(1), Joined)", "(MemberId::<()>(2), Joined)", "(MemberId::<()>(3), Joined)"]);
# }));
```

## Self-Identification
In some programs, it may be necessary for cluster members to know their own ID (for example, to construct a ballot in Paxos). In Hydro, this can be achieved by using the `CLUSTER_SELF_ID` constant, which can be used inside `q!(...)` blocks to get the current cluster member's ID:

```rust
# use hydro_lang::prelude::*;
# use futures::StreamExt;
# tokio_test::block_on(hydro_lang::test_util::multi_location_test(|flow, process| {
use hydro_lang::location::cluster::CLUSTER_SELF_ID;

let workers: Cluster<()> = flow.cluster::<()>();
let self_id_stream = workers.source_iter(q!([CLUSTER_SELF_ID]));
self_id_stream
    .filter(q!(|x| x.get_raw_id() % 2 == 0))
    .map(q!(|x| format!("hello from {}", x.get_raw_id())))
    .send_bincode(&process)
    .values()
// if there are 4 members in the cluster, we should receive 2 elements
// "hello from 0", "hello from 2"
# }, |mut stream| async move {
# let mut results = Vec::new();
# for w in 0..2 {
#     results.push(stream.next().await.unwrap());
# }
# results.sort();
# assert_eq!(results, vec!["hello from 0", "hello from 2"]);
# }));
```

:::info

You can only use `CLUSTER_SELF_ID` in code that will run on a `Cluster<_>`, such as when calling `Stream::map` when that stream is on a cluster. If you try to use it in code that will run on a `Process<_>`, you'll get a compile-time error:

```compile_fail
# use hydro_lang::prelude::*;
# let flow = FlowBuilder::new();
let process: Process<()> = flow.process::<()>();
process.source_iter(q!([CLUSTER_SELF_ID]));
// error[E0277]: the trait bound `ClusterSelfId<'_>: FreeVariableWithContext<hydro_lang::Process<'_>>` is not satisfied
```

:::
