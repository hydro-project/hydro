//! Networking APIs for [`Stream`].

use std::marker::PhantomData;

use serde::Serialize;
use serde::de::DeserializeOwned;
use stageleft::{q, quote_type};
use syn::parse_quote;

use super::{ExactlyOnce, Ordering, Stream, TotalOrder};
use crate::compile::ir::{DebugInstantiate, HydroIrOpMetadata, HydroNode, HydroRoot};
use crate::live_collections::boundedness::{Boundedness, Unbounded};
use crate::live_collections::keyed_singleton::KeyedSingleton;
use crate::live_collections::keyed_stream::KeyedStream;
use crate::live_collections::stream::Retries;
#[cfg(stageleft_runtime)]
use crate::location::dynamic::DynLocation;
use crate::location::external_process::ExternalBincodeStream;
use crate::location::{Cluster, External, Location, MemberId, MembershipEvent, NoTick, Process};
use crate::nondet::NonDet;
use crate::staging_util::get_this_crate;

// same as the one in `hydro_std`, but internal use only
fn track_membership<'a, C, L: Location<'a> + NoTick>(
    membership: KeyedStream<MemberId<C>, MembershipEvent, L, Unbounded>,
) -> KeyedSingleton<MemberId<C>, (), L, Unbounded> {
    membership
        .fold(
            q!(|| false),
            q!(|present, event| {
                match event {
                    MembershipEvent::Joined => *present = true,
                    MembershipEvent::Left => *present = false,
                }
            }),
        )
        .filter_map(q!(|v| if v { Some(()) } else { None }))
}

fn serialize_bincode_with_type(is_demux: bool, t_type: &syn::Type) -> syn::Expr {
    let root = get_this_crate();

    if is_demux {
        parse_quote! {
            ::#root::runtime_support::stageleft::runtime_support::fn1_type_hint::<(#root::__staged::location::MemberId<_>, #t_type), _>(
                |(id, data)| {
                    (id.raw_id, #root::runtime_support::bincode::serialize(&data).unwrap().into())
                }
            )
        }
    } else {
        parse_quote! {
            ::#root::runtime_support::stageleft::runtime_support::fn1_type_hint::<#t_type, _>(
                |data| {
                    #root::runtime_support::bincode::serialize(&data).unwrap().into()
                }
            )
        }
    }
}

pub(crate) fn serialize_bincode<T: Serialize>(is_demux: bool) -> syn::Expr {
    serialize_bincode_with_type(is_demux, &quote_type::<T>())
}

fn deserialize_bincode_with_type(tagged: Option<&syn::Type>, t_type: &syn::Type) -> syn::Expr {
    let root = get_this_crate();

    if let Some(c_type) = tagged {
        parse_quote! {
            |res| {
                let (id, b) = res.unwrap();
                (#root::location::MemberId::<#c_type>::from_raw(id), #root::runtime_support::bincode::deserialize::<#t_type>(&b).unwrap())
            }
        }
    } else {
        parse_quote! {
            |res| {
                #root::runtime_support::bincode::deserialize::<#t_type>(&res.unwrap()).unwrap()
            }
        }
    }
}

pub(crate) fn deserialize_bincode<T: DeserializeOwned>(tagged: Option<&syn::Type>) -> syn::Expr {
    deserialize_bincode_with_type(tagged, &quote_type::<T>())
}

impl<'a, T, L, B: Boundedness, O: Ordering, R: Retries> Stream<T, Process<'a, L>, B, O, R> {
    /// "Moves" elements of this stream to a new distributed location by sending them over the network,
    /// using [`bincode`] to serialize/deserialize messages.
    ///
    /// The returned stream captures the elements received at the destination, where values will
    /// asynchronously arrive over the network. Sending from a [`Process`] to another [`Process`]
    /// preserves ordering and retries guarantees by using a single TCP channel to send the values. The
    /// recipient is guaranteed to receive a _prefix_ or the sent messages; if the TCP connection is
    /// dropped no further messages will be sent.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::multi_location_test(|flow, p_out| {
    /// let p1 = flow.process::<()>();
    /// let numbers: Stream<_, Process<_>, Unbounded> = p1.source_iter(q!(vec![1, 2, 3]));
    /// let p2 = flow.process::<()>();
    /// let on_p2: Stream<_, Process<_>, Unbounded> = numbers.send_bincode(&p2);
    /// // 1, 2, 3
    /// # on_p2.send_bincode(&p_out)
    /// # }, |mut stream| async move {
    /// # for w in 1..=3 {
    /// #     assert_eq!(stream.next().await, Some(w));
    /// # }
    /// # }));
    /// ```
    pub fn send_bincode<L2>(
        self,
        other: &Process<'a, L2>,
    ) -> Stream<T, Process<'a, L2>, Unbounded, O, R>
    where
        T: Serialize + DeserializeOwned,
    {
        let serialize_pipeline = Some(serialize_bincode::<T>(false));

        let deserialize_pipeline = Some(deserialize_bincode::<T>(None));

        Stream::new(
            other.clone(),
            HydroNode::Network {
                serialize_fn: serialize_pipeline.map(|e| e.into()),
                instantiate_fn: DebugInstantiate::Building,
                deserialize_fn: deserialize_pipeline.map(|e| e.into()),
                input: Box::new(self.ir_node.into_inner()),
                metadata: other.new_node_metadata(
                    Stream::<T, Process<'a, L2>, Unbounded, O, R>::collection_kind(),
                ),
            },
        )
    }

    /// Broadcasts elements of this stream to all members of a cluster by sending them over the network,
    /// using [`bincode`] to serialize/deserialize messages.
    ///
    /// Each element in the stream will be sent to **every** member of the cluster based on the latest
    /// membership information. This is a common pattern in distributed systems for broadcasting data to
    /// all nodes in a cluster. Unlike [`Stream::demux_bincode`], which requires `(MemberId, T)` tuples to
    /// target specific members, `broadcast_bincode` takes a stream of **only data elements** and sends
    /// each element to all cluster members.
    ///
    /// # Non-Determinism
    /// The set of cluster members may asynchronously change over time. Each element is only broadcast
    /// to the current cluster members _at that point in time_. Depending on when we are notified of
    /// membership changes, we will broadcast each element to different members.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::multi_location_test(|flow, p2| {
    /// let p1 = flow.process::<()>();
    /// let workers: Cluster<()> = flow.cluster::<()>();
    /// let numbers: Stream<_, Process<_>, _> = p1.source_iter(q!(vec![123]));
    /// let on_worker: Stream<_, Cluster<_>, _> = numbers.broadcast_bincode(&workers, nondet!(/** assuming stable membership */));
    /// # on_worker.send_bincode(&p2).entries()
    /// // if there are 4 members in the cluster, each receives one element
    /// // - MemberId::<()>(0): [123]
    /// // - MemberId::<()>(1): [123]
    /// // - MemberId::<()>(2): [123]
    /// // - MemberId::<()>(3): [123]
    /// # }, |mut stream| async move {
    /// # let mut results = Vec::new();
    /// # for w in 0..4 {
    /// #     results.push(format!("{:?}", stream.next().await.unwrap()));
    /// # }
    /// # results.sort();
    /// # assert_eq!(results, vec!["(MemberId::<()>(0), 123)", "(MemberId::<()>(1), 123)", "(MemberId::<()>(2), 123)", "(MemberId::<()>(3), 123)"]);
    /// # }));
    /// ```
    pub fn broadcast_bincode<L2: 'a>(
        self,
        other: &Cluster<'a, L2>,
        nondet_membership: NonDet,
    ) -> Stream<T, Cluster<'a, L2>, Unbounded, O, R>
    where
        T: Clone + Serialize + DeserializeOwned,
    {
        let ids = track_membership(self.location.source_cluster_members(other));
        let join_tick = self.location.tick();
        let current_members = ids.snapshot(&join_tick, nondet_membership);

        self.batch(&join_tick, nondet_membership)
            .repeat_with_keys(current_members)
            .all_ticks()
            .demux_bincode(other)
    }

    /// Sends the elements of this stream to an external (non-Hydro) process, using [`bincode`]
    /// serialization. The external process can receive these elements by establishing a TCP
    /// connection and decoding using [`tokio_util::codec::LengthDelimitedCodec`].
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(async move {
    /// let flow = FlowBuilder::new();
    /// let process = flow.process::<()>();
    /// let numbers: Stream<_, Process<_>, Unbounded> = process.source_iter(q!(vec![1, 2, 3]));
    /// let external = flow.external::<()>();
    /// let external_handle = numbers.send_bincode_external(&external);
    ///
    /// let mut deployment = hydro_deploy::Deployment::new();
    /// let nodes = flow
    ///     .with_process(&process, deployment.Localhost())
    ///     .with_external(&external, deployment.Localhost())
    ///     .deploy(&mut deployment);
    ///
    /// deployment.deploy().await.unwrap();
    /// // establish the TCP connection
    /// let mut external_recv_stream = nodes.connect(external_handle).await;
    /// deployment.start().await.unwrap();
    ///
    /// for w in 1..=3 {
    ///     assert_eq!(external_recv_stream.next().await, Some(w));
    /// }
    /// # });
    /// ```
    pub fn send_bincode_external<L2>(self, other: &External<L2>) -> ExternalBincodeStream<T, O, R>
    where
        T: Serialize + DeserializeOwned,
    {
        let serialize_pipeline = Some(serialize_bincode::<T>(false));

        let mut flow_state_borrow = self.location.flow_state().borrow_mut();

        let external_key = flow_state_borrow.next_external_out;
        flow_state_borrow.next_external_out += 1;

        flow_state_borrow.push_root(HydroRoot::SendExternal {
            to_external_id: other.id,
            to_key: external_key,
            to_many: false,
            serialize_fn: serialize_pipeline.map(|e| e.into()),
            instantiate_fn: DebugInstantiate::Building,
            input: Box::new(self.ir_node.into_inner()),
            op_metadata: HydroIrOpMetadata::new(),
        });

        ExternalBincodeStream {
            process_id: other.id,
            port_id: external_key,
            _phantom: PhantomData,
        }
    }
}

impl<'a, T, L, L2, B: Boundedness, O: Ordering, R: Retries>
    Stream<(MemberId<L2>, T), Process<'a, L>, B, O, R>
{
    /// Sends elements of this stream to specific members of a cluster, identified by a [`MemberId`],
    /// using [`bincode`] to serialize/deserialize messages.
    ///
    /// Each element in the stream must be a tuple `(MemberId<L2>, T)` where the first element
    /// specifies which cluster member should receive the data. Unlike [`Stream::broadcast_bincode`],
    /// this API allows precise targeting of specific cluster members rather than broadcasting to
    /// all members.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::multi_location_test(|flow, p2| {
    /// let p1 = flow.process::<()>();
    /// let workers: Cluster<()> = flow.cluster::<()>();
    /// let numbers: Stream<_, Process<_>, _> = p1.source_iter(q!(vec![0, 1, 2, 3]));
    /// let on_worker: Stream<_, Cluster<_>, _> = numbers
    ///     .map(q!(|x| (hydro_lang::location::MemberId::from_raw(x), x)))
    ///     .demux_bincode(&workers);
    /// # on_worker.send_bincode(&p2).entries()
    /// // if there are 4 members in the cluster, each receives one element
    /// // - MemberId::<()>(0): [0]
    /// // - MemberId::<()>(1): [1]
    /// // - MemberId::<()>(2): [2]
    /// // - MemberId::<()>(3): [3]
    /// # }, |mut stream| async move {
    /// # let mut results = Vec::new();
    /// # for w in 0..4 {
    /// #     results.push(format!("{:?}", stream.next().await.unwrap()));
    /// # }
    /// # results.sort();
    /// # assert_eq!(results, vec!["(MemberId::<()>(0), 0)", "(MemberId::<()>(1), 1)", "(MemberId::<()>(2), 2)", "(MemberId::<()>(3), 3)"]);
    /// # }));
    /// ```
    pub fn demux_bincode(
        self,
        other: &Cluster<'a, L2>,
    ) -> Stream<T, Cluster<'a, L2>, Unbounded, O, R>
    where
        T: Serialize + DeserializeOwned,
    {
        self.into_keyed().demux_bincode(other)
    }
}

impl<'a, T, L, B: Boundedness> Stream<T, Process<'a, L>, B, TotalOrder, ExactlyOnce> {
    /// Distributes elements of this stream to cluster members in a round-robin fashion, using
    /// [`bincode`] to serialize/deserialize messages.
    ///
    /// This provides load balancing by evenly distributing work across cluster members. The
    /// distribution is deterministic based on element order - the first element goes to member 0,
    /// the second to member 1, and so on, wrapping around when reaching the end of the member list.
    ///
    /// # Non-Determinism
    /// The set of cluster members may asynchronously change over time. Each element is distributed
    /// based on the current cluster membership _at that point in time_. Depending on when cluster
    /// members join and leave, the round-robin pattern will change. Furthermore, even when the
    /// membership is stable, the order of members in the round-robin pattern may change across runs.
    ///
    /// # Ordering Requirements
    /// This method is only available on streams with [`TotalOrder`] and [`ExactlyOnce`], since the
    /// order of messages and retries affects the round-robin pattern.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use hydro_lang::live_collections::stream::{TotalOrder, ExactlyOnce};
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::multi_location_test(|flow, p2| {
    /// let p1 = flow.process::<()>();
    /// let workers: Cluster<()> = flow.cluster::<()>();
    /// let numbers: Stream<_, Process<_>, _, TotalOrder, ExactlyOnce> = p1.source_iter(q!(vec![1, 2, 3, 4]));
    /// let on_worker: Stream<_, Cluster<_>, _> = numbers.round_robin_bincode(&workers, nondet!(/** assuming stable membership */));
    /// on_worker.send_bincode(&p2)
    /// # .first().values() // we use first to assert that each member gets one element
    /// // with 4 cluster members, elements are distributed (with a non-deterministic round-robin order):
    /// // - MemberId::<()>(?): [1]
    /// // - MemberId::<()>(?): [2]
    /// // - MemberId::<()>(?): [3]
    /// // - MemberId::<()>(?): [4]
    /// # }, |mut stream| async move {
    /// # let mut results = Vec::new();
    /// # for w in 0..4 {
    /// #     results.push(stream.next().await.unwrap());
    /// # }
    /// # results.sort();
    /// # assert_eq!(results, vec![1, 2, 3, 4]);
    /// # }));
    /// ```
    pub fn round_robin_bincode<L2: 'a>(
        self,
        other: &Cluster<'a, L2>,
        nondet_membership: NonDet,
    ) -> Stream<T, Cluster<'a, L2>, Unbounded, TotalOrder, ExactlyOnce>
    where
        T: Serialize + DeserializeOwned,
    {
        let ids = track_membership(self.location.source_cluster_members(other));
        let join_tick = self.location.tick();
        let current_members = ids
            .snapshot(&join_tick, nondet_membership)
            .keys()
            .assume_ordering(nondet_membership)
            .collect_vec();

        self.enumerate()
            .batch(&join_tick, nondet_membership)
            .cross_singleton(current_members)
            .map(q!(|(data, members)| (
                members[data.0 % members.len()],
                data.1
            )))
            .all_ticks()
            .demux_bincode(other)
    }
}

impl<'a, T, L, B: Boundedness, O: Ordering, R: Retries> Stream<T, Cluster<'a, L>, B, O, R> {
    /// "Moves" elements of this stream from a cluster to a process by sending them over the network,
    /// using [`bincode`] to serialize/deserialize messages.
    ///
    /// Each cluster member sends its local stream elements, and they are collected at the destination
    /// as a [`KeyedStream`] where keys identify the source cluster member.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::multi_location_test(|flow, process| {
    /// let workers: Cluster<()> = flow.cluster::<()>();
    /// let numbers: Stream<_, Cluster<_>, _> = workers.source_iter(q!(vec![1]));
    /// let all_received = numbers.send_bincode(&process); // KeyedStream<MemberId<()>, i32, ...>
    /// # all_received.entries()
    /// # }, |mut stream| async move {
    /// // if there are 4 members in the cluster, we should receive 4 elements
    /// // { MemberId::<()>(0): [1], MemberId::<()>(1): [1], MemberId::<()>(2): [1], MemberId::<()>(3): [1] }
    /// # let mut results = Vec::new();
    /// # for w in 0..4 {
    /// #     results.push(format!("{:?}", stream.next().await.unwrap()));
    /// # }
    /// # results.sort();
    /// # assert_eq!(results, vec!["(MemberId::<()>(0), 1)", "(MemberId::<()>(1), 1)", "(MemberId::<()>(2), 1)", "(MemberId::<()>(3), 1)"]);
    /// # }));
    /// ```
    ///
    /// If you don't need to know the source for each element, you can use `.values()`
    /// to get just the data:
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use hydro_lang::live_collections::stream::NoOrder;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::multi_location_test(|flow, process| {
    /// # let workers: Cluster<()> = flow.cluster::<()>();
    /// # let numbers: Stream<_, Cluster<_>, _> = workers.source_iter(q!(vec![1]));
    /// let values: Stream<i32, _, _, NoOrder> = numbers.send_bincode(&process).values();
    /// # values
    /// # }, |mut stream| async move {
    /// # let mut results = Vec::new();
    /// # for w in 0..4 {
    /// #     results.push(format!("{:?}", stream.next().await.unwrap()));
    /// # }
    /// # results.sort();
    /// // if there are 4 members in the cluster, we should receive 4 elements
    /// // 1, 1, 1, 1
    /// # assert_eq!(results, vec!["1", "1", "1", "1"]);
    /// # }));
    /// ```
    pub fn send_bincode<L2>(
        self,
        other: &Process<'a, L2>,
    ) -> KeyedStream<MemberId<L>, T, Process<'a, L2>, Unbounded, O, R>
    where
        T: Serialize + DeserializeOwned,
    {
        let serialize_pipeline = Some(serialize_bincode::<T>(false));

        let deserialize_pipeline = Some(deserialize_bincode::<T>(Some(&quote_type::<L>())));

        let raw_stream: Stream<(MemberId<L>, T), Process<'a, L2>, Unbounded, O, R> = Stream::new(
            other.clone(),
            HydroNode::Network {
                serialize_fn: serialize_pipeline.map(|e| e.into()),
                instantiate_fn: DebugInstantiate::Building,
                deserialize_fn: deserialize_pipeline.map(|e| e.into()),
                input: Box::new(self.ir_node.into_inner()),
                metadata: other.new_node_metadata(Stream::<
                    (MemberId<L>, T),
                    Process<'a, L2>,
                    Unbounded,
                    O,
                    R,
                >::collection_kind()),
            },
        );

        raw_stream.into_keyed()
    }

    /// Broadcasts elements of this stream at each source member to all members of a destination
    /// cluster, using [`bincode`] to serialize/deserialize messages.
    ///
    /// Each source member sends each of its stream elements to **every** member of the cluster
    /// based on its latest membership information. Unlike [`Stream::demux_bincode`], which requires
    /// `(MemberId, T)` tuples to target specific members, `broadcast_bincode` takes a stream of
    /// **only data elements** and sends each element to all cluster members.
    ///
    /// # Non-Determinism
    /// The set of cluster members may asynchronously change over time. Each element is only broadcast
    /// to the current cluster members known _at that point in time_ at the source member. Depending
    /// on when each source member is notified of membership changes, it will broadcast each element
    /// to different members.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use hydro_lang::location::MemberId;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::multi_location_test(|flow, p2| {
    /// # type Source = ();
    /// # type Destination = ();
    /// let source: Cluster<Source> = flow.cluster::<Source>();
    /// let numbers: Stream<_, Cluster<Source>, _> = source.source_iter(q!(vec![123]));
    /// let destination: Cluster<Destination> = flow.cluster::<Destination>();
    /// let on_destination: KeyedStream<MemberId<Source>, _, Cluster<Destination>, _> = numbers.broadcast_bincode(&destination, nondet!(/** assuming stable membership */));
    /// # on_destination.entries().send_bincode(&p2).entries()
    /// // if there are 4 members in the desination, each receives one element from each source member
    /// // - Destination(0): { Source(0): [123], Source(1): [123], ... }
    /// // - Destination(1): { Source(0): [123], Source(1): [123], ... }
    /// // - ...
    /// # }, |mut stream| async move {
    /// # let mut results = Vec::new();
    /// # for w in 0..16 {
    /// #     results.push(format!("{:?}", stream.next().await.unwrap()));
    /// # }
    /// # results.sort();
    /// # assert_eq!(results, vec![
    /// #   "(MemberId::<()>(0), (MemberId::<()>(0), 123))", "(MemberId::<()>(0), (MemberId::<()>(1), 123))", "(MemberId::<()>(0), (MemberId::<()>(2), 123))", "(MemberId::<()>(0), (MemberId::<()>(3), 123))",
    /// #   "(MemberId::<()>(1), (MemberId::<()>(0), 123))", "(MemberId::<()>(1), (MemberId::<()>(1), 123))", "(MemberId::<()>(1), (MemberId::<()>(2), 123))", "(MemberId::<()>(1), (MemberId::<()>(3), 123))",
    /// #   "(MemberId::<()>(2), (MemberId::<()>(0), 123))", "(MemberId::<()>(2), (MemberId::<()>(1), 123))", "(MemberId::<()>(2), (MemberId::<()>(2), 123))", "(MemberId::<()>(2), (MemberId::<()>(3), 123))",
    /// #   "(MemberId::<()>(3), (MemberId::<()>(0), 123))", "(MemberId::<()>(3), (MemberId::<()>(1), 123))", "(MemberId::<()>(3), (MemberId::<()>(2), 123))", "(MemberId::<()>(3), (MemberId::<()>(3), 123))"
    /// # ]);
    /// # }));
    /// ```
    pub fn broadcast_bincode<L2: 'a>(
        self,
        other: &Cluster<'a, L2>,
        nondet_membership: NonDet,
    ) -> KeyedStream<MemberId<L>, T, Cluster<'a, L2>, Unbounded, O, R>
    where
        T: Clone + Serialize + DeserializeOwned,
    {
        let ids = track_membership(self.location.source_cluster_members(other));
        let join_tick = self.location.tick();
        let current_members = ids.snapshot(&join_tick, nondet_membership);

        self.batch(&join_tick, nondet_membership)
            .repeat_with_keys(current_members)
            .all_ticks()
            .demux_bincode(other)
    }
}

impl<'a, T, L, L2, B: Boundedness, O: Ordering, R: Retries>
    Stream<(MemberId<L2>, T), Cluster<'a, L>, B, O, R>
{
    /// Sends elements of this stream at each source member to specific members of a destination
    /// cluster, identified by a [`MemberId`], using [`bincode`] to serialize/deserialize messages.
    ///
    /// Each element in the stream must be a tuple `(MemberId<L2>, T)` where the first element
    /// specifies which cluster member should receive the data. Unlike [`Stream::broadcast_bincode`],
    /// this API allows precise targeting of specific cluster members rather than broadcasting to
    /// all members.
    ///
    /// Each cluster member sends its local stream elements, and they are collected at each
    /// destination member as a [`KeyedStream`] where keys identify the source cluster member.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::multi_location_test(|flow, p2| {
    /// # type Source = ();
    /// # type Destination = ();
    /// let source: Cluster<Source> = flow.cluster::<Source>();
    /// let to_send: Stream<_, Cluster<_>, _> = source
    ///     .source_iter(q!(vec![0, 1, 2, 3]))
    ///     .map(q!(|x| (hydro_lang::location::MemberId::from_raw(x), x)));
    /// let destination: Cluster<Destination> = flow.cluster::<Destination>();
    /// let all_received = to_send.demux_bincode(&destination); // KeyedStream<MemberId<Source>, i32, ...>
    /// # all_received.entries().send_bincode(&p2).entries()
    /// # }, |mut stream| async move {
    /// // if there are 4 members in the destination cluster, each receives one message from each source member
    /// // - Destination(0): { Source(0): [0], Source(1): [0], ... }
    /// // - Destination(1): { Source(0): [1], Source(1): [1], ... }
    /// // - ...
    /// # let mut results = Vec::new();
    /// # for w in 0..16 {
    /// #     results.push(format!("{:?}", stream.next().await.unwrap()));
    /// # }
    /// # results.sort();
    /// # assert_eq!(results, vec![
    /// #   "(MemberId::<()>(0), (MemberId::<()>(0), 0))", "(MemberId::<()>(0), (MemberId::<()>(1), 0))", "(MemberId::<()>(0), (MemberId::<()>(2), 0))", "(MemberId::<()>(0), (MemberId::<()>(3), 0))",
    /// #   "(MemberId::<()>(1), (MemberId::<()>(0), 1))", "(MemberId::<()>(1), (MemberId::<()>(1), 1))", "(MemberId::<()>(1), (MemberId::<()>(2), 1))", "(MemberId::<()>(1), (MemberId::<()>(3), 1))",
    /// #   "(MemberId::<()>(2), (MemberId::<()>(0), 2))", "(MemberId::<()>(2), (MemberId::<()>(1), 2))", "(MemberId::<()>(2), (MemberId::<()>(2), 2))", "(MemberId::<()>(2), (MemberId::<()>(3), 2))",
    /// #   "(MemberId::<()>(3), (MemberId::<()>(0), 3))", "(MemberId::<()>(3), (MemberId::<()>(1), 3))", "(MemberId::<()>(3), (MemberId::<()>(2), 3))", "(MemberId::<()>(3), (MemberId::<()>(3), 3))"
    /// # ]);
    /// # }));
    /// ```
    pub fn demux_bincode(
        self,
        other: &Cluster<'a, L2>,
    ) -> KeyedStream<MemberId<L>, T, Cluster<'a, L2>, Unbounded, O, R>
    where
        T: Serialize + DeserializeOwned,
    {
        self.into_keyed().demux_bincode(other)
    }
}

#[cfg(test)]
mod tests {
    use stageleft::q;

    use crate::location::{Location, MemberId};
    use crate::nondet::nondet;
    use crate::prelude::FlowBuilder;

    #[test]
    fn sim_send_bincode_o2o() {
        let flow = FlowBuilder::new();
        let external = flow.external::<()>();
        let node = flow.process::<()>();
        let node2 = flow.process::<()>();

        let (port, input) = node.source_external_bincode(&external);

        let out_port = input
            .send_bincode(&node2)
            .batch(&node2.tick(), nondet!(/** test */))
            .count()
            .all_ticks()
            .send_bincode_external(&external);

        let instances = flow.sim().exhaustive(async |mut compiled| {
            let in_send = compiled.connect(&port);
            let out_recv = compiled.connect(&out_port);
            compiled.launch();

            in_send.send(()).unwrap();
            in_send.send(()).unwrap();
            in_send.send(()).unwrap();

            let received = out_recv.collect::<Vec<_>>().await;
            assert!(received.into_iter().sum::<usize>() == 3);
        });

        assert_eq!(instances, 4); // 2^{3 - 1}
    }

    #[test]
    fn sim_send_bincode_m2o() {
        let flow = FlowBuilder::new();
        let external = flow.external::<()>();
        let cluster = flow.cluster::<()>();
        let node = flow.process::<()>();

        let input = cluster.source_iter(q!(vec![1]));

        let out_port = input
            .send_bincode(&node)
            .entries()
            .batch(&node.tick(), nondet!(/** test */))
            .all_ticks()
            .send_bincode_external(&external);

        let instances =
            flow.sim()
                .with_cluster_size(&cluster, 4)
                .exhaustive(async |mut compiled| {
                    let out_recv = compiled.connect(&out_port);
                    compiled.launch();

                    out_recv
                        .assert_yields_only_unordered(vec![
                            (MemberId::from_raw(0), 1),
                            (MemberId::from_raw(1), 1),
                            (MemberId::from_raw(2), 1),
                            (MemberId::from_raw(3), 1),
                        ])
                        .await
                });

        assert_eq!(instances, 75); // ∑ (k=1 to 4) S(4,k) × k! = 75
    }

    #[test]
    fn sim_send_bincode_multiple_m2o() {
        let flow = FlowBuilder::new();
        let external = flow.external::<()>();
        let cluster1 = flow.cluster::<()>();
        let cluster2 = flow.cluster::<()>();
        let node = flow.process::<()>();

        let out_port_1 = cluster1
            .source_iter(q!(vec![1]))
            .send_bincode(&node)
            .entries()
            .send_bincode_external(&external);

        let out_port_2 = cluster2
            .source_iter(q!(vec![2]))
            .send_bincode(&node)
            .entries()
            .send_bincode_external(&external);

        let instances = flow
            .sim()
            .with_cluster_size(&cluster1, 3)
            .with_cluster_size(&cluster2, 4)
            .exhaustive(async |mut compiled| {
                let out_recv_1 = compiled.connect(&out_port_1);
                let out_recv_2 = compiled.connect(&out_port_2);
                compiled.launch();

                out_recv_1
                    .assert_yields_only_unordered(vec![
                        (MemberId::from_raw(0), 1),
                        (MemberId::from_raw(1), 1),
                        (MemberId::from_raw(2), 1),
                    ])
                    .await;

                out_recv_2
                    .assert_yields_only_unordered(vec![
                        (MemberId::from_raw(0), 2),
                        (MemberId::from_raw(1), 2),
                        (MemberId::from_raw(2), 2),
                        (MemberId::from_raw(3), 2),
                    ])
                    .await;
            });

        assert_eq!(instances, 1);
    }

    #[test]
    fn sim_send_bincode_o2m() {
        let flow = FlowBuilder::new();
        let external = flow.external::<()>();
        let cluster = flow.cluster::<()>();
        let node = flow.process::<()>();

        let input = node.source_iter(q!(vec![
            (MemberId::from_raw(0), 123),
            (MemberId::from_raw(1), 456),
        ]));

        let out_port = input
            .demux_bincode(&cluster)
            .map(q!(|x| x + 1))
            .send_bincode(&node)
            .entries()
            .send_bincode_external(&external);

        flow.sim()
            .with_cluster_size(&cluster, 4)
            .exhaustive(async |mut compiled| {
                let out_recv = compiled.connect(&out_port);
                compiled.launch();

                out_recv
                    .assert_yields_only_unordered(vec![
                        (MemberId::from_raw(0), 124),
                        (MemberId::from_raw(1), 457),
                    ])
                    .await
            });
    }

    #[test]
    fn sim_send_bincode_m2m() {
        let flow = FlowBuilder::new();
        let external = flow.external::<()>();
        let cluster = flow.cluster::<()>();
        let node = flow.process::<()>();

        let input = node.source_iter(q!(vec![
            (MemberId::from_raw(0), 123),
            (MemberId::from_raw(1), 456),
        ]));

        let out_port = input
            .demux_bincode(&cluster)
            .map(q!(|x| x + 1))
            .flat_map_ordered(q!(|x| vec![
                (MemberId::from_raw(0), x),
                (MemberId::from_raw(1), x),
            ]))
            .demux_bincode(&cluster)
            .entries()
            .send_bincode(&node)
            .entries()
            .send_bincode_external(&external);

        flow.sim()
            .with_cluster_size(&cluster, 4)
            .exhaustive(async |mut compiled| {
                let out_recv = compiled.connect(&out_port);
                compiled.launch();

                out_recv
                    .assert_yields_only_unordered(vec![
                        (MemberId::from_raw(0), (MemberId::from_raw(0), 124)),
                        (MemberId::from_raw(0), (MemberId::from_raw(1), 457)),
                        (MemberId::from_raw(1), (MemberId::from_raw(0), 124)),
                        (MemberId::from_raw(1), (MemberId::from_raw(1), 457)),
                    ])
                    .await
            });
    }
}
