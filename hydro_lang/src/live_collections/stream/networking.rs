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
use crate::live_collections::sliced::sliced;
use crate::live_collections::stream::Retries;
#[cfg(stageleft_runtime)]
use crate::location::dynamic::DynLocation;
use crate::location::external_process::ExternalBincodeStream;
use crate::location::{Cluster, External, Location, MemberId, MembershipEvent, NoTick, Process};
use crate::networking::{NetworkFor, TCP};
use crate::nondet::NonDet;
#[cfg(feature = "sim")]
use crate::sim::SimReceiver;
use crate::staging_util::get_this_crate;

// same as the one in `hydro_std`, but internal use only
fn track_membership<'a, C, L: Location<'a> + NoTick>(
    membership: KeyedStream<MemberId<C>, MembershipEvent, L, Unbounded>,
) -> KeyedSingleton<MemberId<C>, bool, L, Unbounded> {
    membership.fold(
        q!(|| false),
        q!(|present, event| {
            match event {
                MembershipEvent::Joined => *present = true,
                MembershipEvent::Left => *present = false,
            }
        }),
    )
}

fn serialize_bincode_with_type(is_demux: bool, t_type: &syn::Type) -> syn::Expr {
    let root = get_this_crate();

    if is_demux {
        parse_quote! {
            ::#root::runtime_support::stageleft::runtime_support::fn1_type_hint::<(#root::__staged::location::MemberId<_>, #t_type), _>(
                |(id, data)| {
                    (id.into_tagless(), #root::runtime_support::bincode::serialize(&data).unwrap().into())
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
                (#root::__staged::location::MemberId::<#c_type>::from_tagless(id as #root::__staged::location::TaglessMemberId), #root::runtime_support::bincode::deserialize::<#t_type>(&b).unwrap())
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
    #[deprecated = "use Stream::send(..., TCP.bincode()) instead"]
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
    /// # #[cfg(feature = "deploy")] {
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
    /// # }
    /// ```
    pub fn send_bincode<L2>(
        self,
        other: &Process<'a, L2>,
    ) -> Stream<T, Process<'a, L2>, Unbounded, O, R>
    where
        T: Serialize + DeserializeOwned,
    {
        self.send(other, TCP.bincode())
    }

    /// "Moves" elements of this stream to a new distributed location by sending them over the network,
    /// using the configuration in `via` to set up the message transport.
    ///
    /// The returned stream captures the elements received at the destination, where values will
    /// asynchronously arrive over the network. Sending from a [`Process`] to another [`Process`]
    /// preserves ordering and retries guarantees when using a single TCP channel to send the values.
    /// The recipient is guaranteed to receive a _prefix_ or the sent messages; if the connection is
    /// dropped no further messages will be sent.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::multi_location_test(|flow, p_out| {
    /// let p1 = flow.process::<()>();
    /// let numbers: Stream<_, Process<_>, Unbounded> = p1.source_iter(q!(vec![1, 2, 3]));
    /// let p2 = flow.process::<()>();
    /// let on_p2: Stream<_, Process<_>, Unbounded> = numbers.send(&p2, TCP.bincode());
    /// // 1, 2, 3
    /// # on_p2.send(&p_out, TCP.bincode())
    /// # }, |mut stream| async move {
    /// # for w in 1..=3 {
    /// #     assert_eq!(stream.next().await, Some(w));
    /// # }
    /// # }));
    /// # }
    /// ```
    pub fn send<L2, N: NetworkFor<T>>(
        self,
        to: &Process<'a, L2>,
        via: N,
    ) -> Stream<T, Process<'a, L2>, Unbounded, O, R>
    where
        T: Serialize + DeserializeOwned,
    {
        let _ = via;
        let serialize_pipeline = Some(N::serialize_thunk(false));
        let deserialize_pipeline = Some(N::deserialize_thunk(None));

        Stream::new(
            to.clone(),
            HydroNode::Network {
                serialize_fn: serialize_pipeline.map(|e| e.into()),
                instantiate_fn: DebugInstantiate::Building,
                deserialize_fn: deserialize_pipeline.map(|e| e.into()),
                input: Box::new(self.ir_node.into_inner()),
                metadata: to.new_node_metadata(
                    Stream::<T, Process<'a, L2>, Unbounded, O, R>::collection_kind(),
                ),
            },
        )
    }

    #[deprecated = "use Stream::broadcast(..., TCP.bincode()) instead"]
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
    /// # #[cfg(feature = "deploy")] {
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
    /// # }
    /// ```
    pub fn broadcast_bincode<L2: 'a>(
        self,
        other: &Cluster<'a, L2>,
        nondet_membership: NonDet,
    ) -> Stream<T, Cluster<'a, L2>, Unbounded, O, R>
    where
        T: Clone + Serialize + DeserializeOwned,
    {
        self.broadcast(other, TCP.bincode(), nondet_membership)
    }

    /// Broadcasts elements of this stream to all members of a cluster by sending them over the network,
    /// using the configuration in `via` to set up the message transport.
    ///
    /// Each element in the stream will be sent to **every** member of the cluster based on the latest
    /// membership information. This is a common pattern in distributed systems for broadcasting data to
    /// all nodes in a cluster. Unlike [`Stream::demux`], which requires `(MemberId, T)` tuples to
    /// target specific members, `broadcast` takes a stream of **only data elements** and sends
    /// each element to all cluster members.
    ///
    /// # Non-Determinism
    /// The set of cluster members may asynchronously change over time. Each element is only broadcast
    /// to the current cluster members _at that point in time_. Depending on when we are notified of
    /// membership changes, we will broadcast each element to different members.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::multi_location_test(|flow, p2| {
    /// let p1 = flow.process::<()>();
    /// let workers: Cluster<()> = flow.cluster::<()>();
    /// let numbers: Stream<_, Process<_>, _> = p1.source_iter(q!(vec![123]));
    /// let on_worker: Stream<_, Cluster<_>, _> = numbers.broadcast(&workers, nondet!(/** assuming stable membership */), TCP.bincode());
    /// # on_worker.send(&p2, TCP.bincode()).entries()
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
    /// # }
    /// ```
    pub fn broadcast<L2: 'a, N: NetworkFor<T>>(
        self,
        to: &Cluster<'a, L2>,
        via: N,
        nondet_membership: NonDet,
    ) -> Stream<T, Cluster<'a, L2>, Unbounded, O, R>
    where
        T: Clone + Serialize + DeserializeOwned,
    {
        let ids = track_membership(self.location.source_cluster_members(to));
        sliced! {
            let members_snapshot = use(ids, nondet_membership);
            let elements = use(self, nondet_membership);

            let current_members = members_snapshot.filter(q!(|b| *b));
            elements.repeat_with_keys(current_members)
        }
        .demux(to, via)
    }

    /// Sends the elements of this stream to an external (non-Hydro) process, using [`bincode`]
    /// serialization. The external process can receive these elements by establishing a TCP
    /// connection and decoding using [`tokio_util::codec::LengthDelimitedCodec`].
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
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
    /// # }
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
            unpaired: true,
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

    #[cfg(feature = "sim")]
    /// Sets up a simulation output port for this stream, allowing test code to receive elements
    /// sent to this stream during simulation.
    pub fn sim_output(self) -> SimReceiver<T, O, R>
    where
        T: Serialize + DeserializeOwned,
    {
        let external_location: External<'a, ()> = External {
            id: 0,
            flow_state: self.location.flow_state().clone(),
            _phantom: PhantomData,
        };

        let external = self.send_bincode_external(&external_location);

        SimReceiver(external.port_id, PhantomData)
    }
}

impl<'a, T, L, L2, B: Boundedness, O: Ordering, R: Retries>
    Stream<(MemberId<L2>, T), Process<'a, L>, B, O, R>
{
    #[deprecated = "use Stream::demux(..., TCP.bincode()) instead"]
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
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::multi_location_test(|flow, p2| {
    /// let p1 = flow.process::<()>();
    /// let workers: Cluster<()> = flow.cluster::<()>();
    /// let numbers: Stream<_, Process<_>, _> = p1.source_iter(q!(vec![0, 1, 2, 3]));
    /// let on_worker: Stream<_, Cluster<_>, _> = numbers
    ///     .map(q!(|x| (hydro_lang::location::MemberId::from_raw_id(x), x)))
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
    /// # }
    /// ```
    pub fn demux_bincode(
        self,
        other: &Cluster<'a, L2>,
    ) -> Stream<T, Cluster<'a, L2>, Unbounded, O, R>
    where
        T: Serialize + DeserializeOwned,
    {
        self.demux(other, TCP.bincode())
    }

    /// Sends elements of this stream to specific members of a cluster, identified by a [`MemberId`],
    /// using the configuration in `via` to set up the message transport.
    ///
    /// Each element in the stream must be a tuple `(MemberId<L2>, T)` where the first element
    /// specifies which cluster member should receive the data. Unlike [`Stream::broadcast`],
    /// this API allows precise targeting of specific cluster members rather than broadcasting to
    /// all members.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::multi_location_test(|flow, p2| {
    /// let p1 = flow.process::<()>();
    /// let workers: Cluster<()> = flow.cluster::<()>();
    /// let numbers: Stream<_, Process<_>, _> = p1.source_iter(q!(vec![0, 1, 2, 3]));
    /// let on_worker: Stream<_, Cluster<_>, _> = numbers
    ///     .map(q!(|x| (hydro_lang::location::MemberId::from_raw_id(x), x)))
    ///     .demux(&workers, TCP.bincode());
    /// # on_worker.send(&p2, TCP.bincode()).entries()
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
    /// # }
    /// ```
    pub fn demux<N: NetworkFor<T>>(
        self,
        to: &Cluster<'a, L2>,
        via: N,
    ) -> Stream<T, Cluster<'a, L2>, Unbounded, O, R>
    where
        T: Serialize + DeserializeOwned,
    {
        self.into_keyed().demux(to, via)
    }
}

impl<'a, T, L, B: Boundedness> Stream<T, Process<'a, L>, B, TotalOrder, ExactlyOnce> {
    #[deprecated = "use Stream::round_robin(..., TCP.bincode()) instead"]
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
    /// # #[cfg(feature = "deploy")] {
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
    /// # }
    /// ```
    pub fn round_robin_bincode<L2: 'a>(
        self,
        other: &Cluster<'a, L2>,
        nondet_membership: NonDet,
    ) -> Stream<T, Cluster<'a, L2>, Unbounded, TotalOrder, ExactlyOnce>
    where
        T: Serialize + DeserializeOwned,
    {
        self.round_robin(other, TCP.bincode(), nondet_membership)
    }

    /// Distributes elements of this stream to cluster members in a round-robin fashion, using
    /// the configuration in `via` to set up the message transport.
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
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use hydro_lang::live_collections::stream::{TotalOrder, ExactlyOnce};
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::multi_location_test(|flow, p2| {
    /// let p1 = flow.process::<()>();
    /// let workers: Cluster<()> = flow.cluster::<()>();
    /// let numbers: Stream<_, Process<_>, _, TotalOrder, ExactlyOnce> = p1.source_iter(q!(vec![1, 2, 3, 4]));
    /// let on_worker: Stream<_, Cluster<_>, _> = numbers.round_robin(&workers, TCP.bincode(), nondet!(/** assuming stable membership */));
    /// on_worker.send(&p2, TCP.bincode())
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
    /// # }
    /// ```
    pub fn round_robin<L2: 'a, N: NetworkFor<T>>(
        self,
        to: &Cluster<'a, L2>,
        via: N,
        nondet_membership: NonDet,
    ) -> Stream<T, Cluster<'a, L2>, Unbounded, TotalOrder, ExactlyOnce>
    where
        T: Serialize + DeserializeOwned,
    {
        let ids = track_membership(self.location.source_cluster_members(to));
        sliced! {
            let members_snapshot = use(ids, nondet_membership);
            let elements = use(self.enumerate(), nondet_membership);

            let current_members = members_snapshot
                .filter(q!(|b| *b))
                .keys()
                .assume_ordering(nondet_membership)
                .collect_vec();

            elements
                .cross_singleton(current_members)
                .map(q!(|(data, members)| (
                    members[data.0 % members.len()].clone(),
                    data.1
                )))
        }
        .demux(to, via)
    }
}

impl<'a, T, L, B: Boundedness> Stream<T, Cluster<'a, L>, B, TotalOrder, ExactlyOnce> {
    #[deprecated = "use Stream::round_robin(..., TCP.bincode()) instead"]
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
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use hydro_lang::live_collections::stream::{TotalOrder, ExactlyOnce, NoOrder};
    /// # use hydro_lang::location::MemberId;
    /// # use futures::StreamExt;
    /// # std::thread::spawn(|| {
    /// # tokio_test::block_on(hydro_lang::test_util::multi_location_test(|flow, p2| {
    /// let p1 = flow.process::<()>();
    /// let workers1: Cluster<()> = flow.cluster::<()>();
    /// let workers2: Cluster<()> = flow.cluster::<()>();
    /// let numbers: Stream<_, Process<_>, _, TotalOrder, ExactlyOnce> = p1.source_iter(q!(0..=16));
    /// let on_worker1: Stream<_, Cluster<_>, _> = numbers.round_robin_bincode(&workers1, nondet!(/** assuming stable membership */));
    /// let on_worker2: Stream<_, Cluster<_>, _> = on_worker1.round_robin_bincode(&workers2, nondet!(/** assuming stable membership */)).entries().assume_ordering(nondet!(/** assuming stable membership */));
    /// on_worker2.send_bincode(&p2)
    /// # .entries()
    /// # .map(q!(|(w2, (w1, v))| ((w2, w1), v)))
    /// # }, |mut stream| async move {
    /// # let mut results = Vec::new();
    /// # let mut locations = std::collections::HashSet::new();
    /// # for w in 0..=16 {
    /// #     let (location, v) = stream.next().await.unwrap();
    /// #     locations.insert(location);
    /// #     results.push(v);
    /// # }
    /// # results.sort();
    /// # assert_eq!(results, (0..=16).collect::<Vec<_>>());
    /// # assert_eq!(locations.len(), 16);
    /// # }));
    /// # }).join().unwrap();
    /// # }
    /// ```
    pub fn round_robin_bincode<L2: 'a>(
        self,
        other: &Cluster<'a, L2>,
        nondet_membership: NonDet,
    ) -> KeyedStream<MemberId<L>, T, Cluster<'a, L2>, Unbounded, TotalOrder, ExactlyOnce>
    where
        T: Serialize + DeserializeOwned,
    {
        self.round_robin(other, TCP.bincode(), nondet_membership)
    }

    /// Distributes elements of this stream to cluster members in a round-robin fashion, using
    /// the configuration in `via` to set up the message transport.
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
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use hydro_lang::live_collections::stream::{TotalOrder, ExactlyOnce, NoOrder};
    /// # use hydro_lang::location::MemberId;
    /// # use futures::StreamExt;
    /// # std::thread::spawn(|| {
    /// # tokio_test::block_on(hydro_lang::test_util::multi_location_test(|flow, p2| {
    /// let p1 = flow.process::<()>();
    /// let workers1: Cluster<()> = flow.cluster::<()>();
    /// let workers2: Cluster<()> = flow.cluster::<()>();
    /// let numbers: Stream<_, Process<_>, _, TotalOrder, ExactlyOnce> = p1.source_iter(q!(0..=16));
    /// let on_worker1: Stream<_, Cluster<_>, _> = numbers.round_robin(&workers1, TCP.bincode(), nondet!(/** assuming stable membership */));
    /// let on_worker2: Stream<_, Cluster<_>, _> = on_worker1.round_robin(&workers2, TCP.bincode(), nondet!(/** assuming stable membership */)).entries().assume_ordering(nondet!(/** assuming stable membership */));
    /// on_worker2.send(&p2, TCP.bincode())
    /// # .entries()
    /// # .map(q!(|(w2, (w1, v))| ((w2, w1), v)))
    /// # }, |mut stream| async move {
    /// # let mut results = Vec::new();
    /// # let mut locations = std::collections::HashSet::new();
    /// # for w in 0..=16 {
    /// #     let (location, v) = stream.next().await.unwrap();
    /// #     locations.insert(location);
    /// #     results.push(v);
    /// # }
    /// # results.sort();
    /// # assert_eq!(results, (0..=16).collect::<Vec<_>>());
    /// # assert_eq!(locations.len(), 16);
    /// # }));
    /// # }).join().unwrap();
    /// # }
    /// ```
    pub fn round_robin<L2: 'a, N: NetworkFor<T>>(
        self,
        to: &Cluster<'a, L2>,
        via: N,
        nondet_membership: NonDet,
    ) -> KeyedStream<MemberId<L>, T, Cluster<'a, L2>, Unbounded, TotalOrder, ExactlyOnce>
    where
        T: Serialize + DeserializeOwned,
    {
        let ids = track_membership(self.location.source_cluster_members(to));
        sliced! {
            let members_snapshot = use(ids, nondet_membership);
            let elements = use(self.enumerate(), nondet_membership);

            let current_members = members_snapshot
                .filter(q!(|b| *b))
                .keys()
                .assume_ordering(nondet_membership)
                .collect_vec();

            elements
                .cross_singleton(current_members)
                .map(q!(|(data, members)| (
                    members[data.0 % members.len()].clone(),
                    data.1
                )))
        }
        .demux(to, via)
    }
}

impl<'a, T, L, B: Boundedness, O: Ordering, R: Retries> Stream<T, Cluster<'a, L>, B, O, R> {
    #[deprecated = "use Stream::send(..., TCP.bincode()) instead"]
    /// "Moves" elements of this stream from a cluster to a process by sending them over the network,
    /// using [`bincode`] to serialize/deserialize messages.
    ///
    /// Each cluster member sends its local stream elements, and they are collected at the destination
    /// as a [`KeyedStream`] where keys identify the source cluster member.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
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
    /// # }
    /// ```
    ///
    /// If you don't need to know the source for each element, you can use `.values()`
    /// to get just the data:
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
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
    /// # }
    /// ```
    pub fn send_bincode<L2>(
        self,
        other: &Process<'a, L2>,
    ) -> KeyedStream<MemberId<L>, T, Process<'a, L2>, Unbounded, O, R>
    where
        T: Serialize + DeserializeOwned,
    {
        self.send(other, TCP.bincode())
    }

    /// "Moves" elements of this stream from a cluster to a process by sending them over the network,
    /// using the configuration in `via` to set up the message transport.
    ///
    /// Each cluster member sends its local stream elements, and they are collected at the destination
    /// as a [`KeyedStream`] where keys identify the source cluster member.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::multi_location_test(|flow, process| {
    /// let workers: Cluster<()> = flow.cluster::<()>();
    /// let numbers: Stream<_, Cluster<_>, _> = workers.source_iter(q!(vec![1]));
    /// let all_received = numbers.send(&process, TCP.bincode()); // KeyedStream<MemberId<()>, i32, ...>
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
    /// # }
    /// ```
    ///
    /// If you don't need to know the source for each element, you can use `.values()`
    /// to get just the data:
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use hydro_lang::live_collections::stream::NoOrder;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::multi_location_test(|flow, process| {
    /// # let workers: Cluster<()> = flow.cluster::<()>();
    /// # let numbers: Stream<_, Cluster<_>, _> = workers.source_iter(q!(vec![1]));
    /// let values: Stream<i32, _, _, NoOrder> = numbers.send(&process, TCP.bincode()).values();
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
    /// # }
    /// ```
    pub fn send<L2, N: NetworkFor<T>>(
        self,
        to: &Process<'a, L2>,
        via: N,
    ) -> KeyedStream<MemberId<L>, T, Process<'a, L2>, Unbounded, O, R>
    where
        T: Serialize + DeserializeOwned,
    {
        let _ = via;
        let serialize_pipeline = Some(N::serialize_thunk(false));

        let deserialize_pipeline = Some(N::deserialize_thunk(Some(&quote_type::<L>())));

        let raw_stream: Stream<(MemberId<L>, T), Process<'a, L2>, Unbounded, O, R> = Stream::new(
            to.clone(),
            HydroNode::Network {
                serialize_fn: serialize_pipeline.map(|e| e.into()),
                instantiate_fn: DebugInstantiate::Building,
                deserialize_fn: deserialize_pipeline.map(|e| e.into()),
                input: Box::new(self.ir_node.into_inner()),
                metadata: to.new_node_metadata(Stream::<
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

    #[deprecated = "use Stream::broadcast(..., TCP.bincode()) instead"]
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
    /// # #[cfg(feature = "deploy")] {
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
    /// # }
    /// ```
    pub fn broadcast_bincode<L2: 'a>(
        self,
        other: &Cluster<'a, L2>,
        nondet_membership: NonDet,
    ) -> KeyedStream<MemberId<L>, T, Cluster<'a, L2>, Unbounded, O, R>
    where
        T: Clone + Serialize + DeserializeOwned,
    {
        self.broadcast(other, TCP.bincode(), nondet_membership)
    }

    /// Broadcasts elements of this stream at each source member to all members of a destination
    /// cluster, using the configuration in `via` to set up the message transport.
    ///
    /// Each source member sends each of its stream elements to **every** member of the cluster
    /// based on its latest membership information. Unlike [`Stream::demux`], which requires
    /// `(MemberId, T)` tuples to target specific members, `broadcast` takes a stream of
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
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use hydro_lang::location::MemberId;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::multi_location_test(|flow, p2| {
    /// # type Source = ();
    /// # type Destination = ();
    /// let source: Cluster<Source> = flow.cluster::<Source>();
    /// let numbers: Stream<_, Cluster<Source>, _> = source.source_iter(q!(vec![123]));
    /// let destination: Cluster<Destination> = flow.cluster::<Destination>();
    /// let on_destination: KeyedStream<MemberId<Source>, _, Cluster<Destination>, _> = numbers.broadcast(&destination, nondet!(/** assuming stable membership */), TCP.bincode());
    /// # on_destination.entries().send(&p2, TCP.bincode()).entries()
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
    /// # }
    /// ```
    pub fn broadcast<L2: 'a, N: NetworkFor<T>>(
        self,
        to: &Cluster<'a, L2>,
        via: N,
        nondet_membership: NonDet,
    ) -> KeyedStream<MemberId<L>, T, Cluster<'a, L2>, Unbounded, O, R>
    where
        T: Clone + Serialize + DeserializeOwned,
    {
        let ids = track_membership(self.location.source_cluster_members(to));
        sliced! {
            let members_snapshot = use(ids, nondet_membership);
            let elements = use(self, nondet_membership);

            let current_members = members_snapshot.filter(q!(|b| *b));
            elements.repeat_with_keys(current_members)
        }
        .demux(to, via)
    }
}

impl<'a, T, L, L2, B: Boundedness, O: Ordering, R: Retries>
    Stream<(MemberId<L2>, T), Cluster<'a, L>, B, O, R>
{
    #[deprecated = "use Stream::demux(..., TCP.bincode()) instead"]
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
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::multi_location_test(|flow, p2| {
    /// # type Source = ();
    /// # type Destination = ();
    /// let source: Cluster<Source> = flow.cluster::<Source>();
    /// let to_send: Stream<_, Cluster<_>, _> = source
    ///     .source_iter(q!(vec![0, 1, 2, 3]))
    ///     .map(q!(|x| (hydro_lang::location::MemberId::from_raw_id(x), x)));
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
    /// # }
    /// ```
    pub fn demux_bincode(
        self,
        other: &Cluster<'a, L2>,
    ) -> KeyedStream<MemberId<L>, T, Cluster<'a, L2>, Unbounded, O, R>
    where
        T: Serialize + DeserializeOwned,
    {
        self.demux(other, TCP.bincode())
    }

    /// Sends elements of this stream at each source member to specific members of a destination
    /// cluster, identified by a [`MemberId`], using the configuration in `via` to set up the
    /// message transport.
    ///
    /// Each element in the stream must be a tuple `(MemberId<L2>, T)` where the first element
    /// specifies which cluster member should receive the data. Unlike [`Stream::broadcast`],
    /// this API allows precise targeting of specific cluster members rather than broadcasting to
    /// all members.
    ///
    /// Each cluster member sends its local stream elements, and they are collected at each
    /// destination member as a [`KeyedStream`] where keys identify the source cluster member.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::multi_location_test(|flow, p2| {
    /// # type Source = ();
    /// # type Destination = ();
    /// let source: Cluster<Source> = flow.cluster::<Source>();
    /// let to_send: Stream<_, Cluster<_>, _> = source
    ///     .source_iter(q!(vec![0, 1, 2, 3]))
    ///     .map(q!(|x| (hydro_lang::location::MemberId::from_raw_id(x), x)));
    /// let destination: Cluster<Destination> = flow.cluster::<Destination>();
    /// let all_received = to_send.demux(&destination, TCP.bincode()); // KeyedStream<MemberId<Source>, i32, ...>
    /// # all_received.entries().send(&p2, TCP.bincode()).entries()
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
    /// # }
    /// ```
    pub fn demux<N: NetworkFor<T>>(
        self,
        to: &Cluster<'a, L2>,
        via: N,
    ) -> KeyedStream<MemberId<L>, T, Cluster<'a, L2>, Unbounded, O, R>
    where
        T: Serialize + DeserializeOwned,
    {
        self.into_keyed().demux(to, via)
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "sim")]
    use stageleft::q;

    #[cfg(feature = "sim")]
    use crate::location::{Location, MemberId};
    #[cfg(feature = "sim")]
    use crate::networking::TCP;
    #[cfg(feature = "sim")]
    use crate::nondet::nondet;
    #[cfg(feature = "sim")]
    use crate::prelude::FlowBuilder;

    #[cfg(feature = "sim")]
    #[test]
    fn sim_send_bincode_o2o() {
        use crate::networking::TCP;

        let flow = FlowBuilder::new();
        let node = flow.process::<()>();
        let node2 = flow.process::<()>();

        let (in_send, input) = node.sim_input();

        let out_recv = input
            .send(&node2, TCP.bincode())
            .batch(&node2.tick(), nondet!(/** test */))
            .count()
            .all_ticks()
            .sim_output();

        let instances = flow.sim().exhaustive(async || {
            in_send.send(());
            in_send.send(());
            in_send.send(());

            let received = out_recv.collect::<Vec<_>>().await;
            assert!(received.into_iter().sum::<usize>() == 3);
        });

        assert_eq!(instances, 4); // 2^{3 - 1}
    }

    #[cfg(feature = "sim")]
    #[test]
    fn sim_send_bincode_m2o() {
        let flow = FlowBuilder::new();
        let cluster = flow.cluster::<()>();
        let node = flow.process::<()>();

        let input = cluster.source_iter(q!(vec![1]));

        let out_recv = input
            .send(&node, TCP.bincode())
            .entries()
            .batch(&node.tick(), nondet!(/** test */))
            .all_ticks()
            .sim_output();

        let instances = flow
            .sim()
            .with_cluster_size(&cluster, 4)
            .exhaustive(async || {
                out_recv
                    .assert_yields_only_unordered(vec![
                        (MemberId::from_raw_id(0), 1),
                        (MemberId::from_raw_id(1), 1),
                        (MemberId::from_raw_id(2), 1),
                        (MemberId::from_raw_id(3), 1),
                    ])
                    .await
            });

        assert_eq!(instances, 75); // ∑ (k=1 to 4) S(4,k) × k! = 75
    }

    #[cfg(feature = "sim")]
    #[test]
    fn sim_send_bincode_multiple_m2o() {
        let flow = FlowBuilder::new();
        let cluster1 = flow.cluster::<()>();
        let cluster2 = flow.cluster::<()>();
        let node = flow.process::<()>();

        let out_recv_1 = cluster1
            .source_iter(q!(vec![1]))
            .send(&node, TCP.bincode())
            .entries()
            .sim_output();

        let out_recv_2 = cluster2
            .source_iter(q!(vec![2]))
            .send(&node, TCP.bincode())
            .entries()
            .sim_output();

        let instances = flow
            .sim()
            .with_cluster_size(&cluster1, 3)
            .with_cluster_size(&cluster2, 4)
            .exhaustive(async || {
                out_recv_1
                    .assert_yields_only_unordered(vec![
                        (MemberId::from_raw_id(0), 1),
                        (MemberId::from_raw_id(1), 1),
                        (MemberId::from_raw_id(2), 1),
                    ])
                    .await;

                out_recv_2
                    .assert_yields_only_unordered(vec![
                        (MemberId::from_raw_id(0), 2),
                        (MemberId::from_raw_id(1), 2),
                        (MemberId::from_raw_id(2), 2),
                        (MemberId::from_raw_id(3), 2),
                    ])
                    .await;
            });

        assert_eq!(instances, 1);
    }

    #[cfg(feature = "sim")]
    #[test]
    fn sim_send_bincode_o2m() {
        let flow = FlowBuilder::new();
        let cluster = flow.cluster::<()>();
        let node = flow.process::<()>();

        let input = node.source_iter(q!(vec![
            (MemberId::from_raw_id(0), 123),
            (MemberId::from_raw_id(1), 456),
        ]));

        let out_recv = input
            .demux(&cluster, TCP.bincode())
            .map(q!(|x| x + 1))
            .send(&node, TCP.bincode())
            .entries()
            .sim_output();

        flow.sim()
            .with_cluster_size(&cluster, 4)
            .exhaustive(async || {
                out_recv
                    .assert_yields_only_unordered(vec![
                        (MemberId::from_raw_id(0), 124),
                        (MemberId::from_raw_id(1), 457),
                    ])
                    .await
            });
    }

    #[cfg(feature = "sim")]
    #[test]
    fn sim_broadcast_bincode_o2m() {
        let flow = FlowBuilder::new();
        let cluster = flow.cluster::<()>();
        let node = flow.process::<()>();

        let input = node.source_iter(q!(vec![123, 456]));

        let out_recv = input
            .broadcast(&cluster, TCP.bincode(), nondet!(/** test */))
            .map(q!(|x| x + 1))
            .send(&node, TCP.bincode())
            .entries()
            .sim_output();

        let mut c_1_produced = false;
        let mut c_2_produced = false;

        flow.sim()
            .with_cluster_size(&cluster, 2)
            .exhaustive(async || {
                let all_out = out_recv.collect_sorted::<Vec<_>>().await;

                // check that order is preserved
                if all_out.contains(&(MemberId::from_raw_id(0), 124)) {
                    assert!(all_out.contains(&(MemberId::from_raw_id(0), 457)));
                    c_1_produced = true;
                }

                if all_out.contains(&(MemberId::from_raw_id(1), 124)) {
                    assert!(all_out.contains(&(MemberId::from_raw_id(1), 457)));
                    c_2_produced = true;
                }
            });

        assert!(c_1_produced && c_2_produced); // in at least one execution each, the cluster member received both messages
    }

    #[cfg(feature = "sim")]
    #[test]
    fn sim_send_bincode_m2m() {
        let flow = FlowBuilder::new();
        let cluster = flow.cluster::<()>();
        let node = flow.process::<()>();

        let input = node.source_iter(q!(vec![
            (MemberId::from_raw_id(0), 123),
            (MemberId::from_raw_id(1), 456),
        ]));

        let out_recv = input
            .demux(&cluster, TCP.bincode())
            .map(q!(|x| x + 1))
            .flat_map_ordered(q!(|x| vec![
                (MemberId::from_raw_id(0), x),
                (MemberId::from_raw_id(1), x),
            ]))
            .demux(&cluster, TCP.bincode())
            .entries()
            .send(&node, TCP.bincode())
            .entries()
            .sim_output();

        flow.sim()
            .with_cluster_size(&cluster, 4)
            .exhaustive(async || {
                out_recv
                    .assert_yields_only_unordered(vec![
                        (MemberId::from_raw_id(0), (MemberId::from_raw_id(0), 124)),
                        (MemberId::from_raw_id(0), (MemberId::from_raw_id(1), 457)),
                        (MemberId::from_raw_id(1), (MemberId::from_raw_id(0), 124)),
                        (MemberId::from_raw_id(1), (MemberId::from_raw_id(1), 457)),
                    ])
                    .await
            });
    }
}
