//! Networking APIs for [`Stream`].

use std::marker::PhantomData;

use serde::Serialize;
use serde::de::DeserializeOwned;
use stageleft::{q, quote_type};
use syn::parse_quote;

use crate::compile::ir::{DebugInstantiate, HydroIrOpMetadata, HydroNode, HydroRoot};
use crate::live_collections::boundedness::{Boundedness, Unbounded};
use crate::live_collections::keyed_singleton::KeyedSingleton;
use crate::live_collections::keyed_stream::KeyedStream;
use crate::live_collections::stream::{ExactlyOnce, Stream, TotalOrder};
#[cfg(stageleft_runtime)]
use crate::location::dynamic::DynLocation;
use crate::location::external_process::ExternalBincodeStream;
use crate::location::tick::NoAtomic;
use crate::location::{Cluster, External, Location, MemberId, MembershipEvent, NoTick, Process};
use crate::nondet::{NonDet, nondet};
use crate::staging_util::get_this_crate;

// same as the one in `hydro_std`, but internal use only
fn track_membership<'a, C, L: Location<'a> + NoTick + NoAtomic>(
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

#[expect(missing_docs, reason = "TODO")]
pub fn serialize_bincode_with_type(is_demux: bool, t_type: &syn::Type) -> syn::Expr {
    let root = get_this_crate();

    if is_demux {
        parse_quote! {
            ::#root::runtime_support::stageleft::runtime_support::fn1_type_hint::<(#root::location::MemberId<_>, #t_type), _>(
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

#[expect(missing_docs, reason = "TODO")]
pub fn serialize_bincode<T: Serialize>(is_demux: bool) -> syn::Expr {
    serialize_bincode_with_type(is_demux, &quote_type::<T>())
}

#[expect(missing_docs, reason = "TODO")]
pub fn deserialize_bincode_with_type(tagged: Option<&syn::Type>, t_type: &syn::Type) -> syn::Expr {
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

impl<'a, T, L, B: Boundedness, O, R> Stream<T, Process<'a, L>, B, O, R> {
    /// "Moves" elements of this stream to a new distributed location by sending them over the network,
    /// using [`bincode`] to serialize/deserialize messages. The returned stream captures the elements
    /// received at the destination, where values will asynchronously arrive over the network.
    ///
    /// Sending from a [`Process`] to another [`Process`] preserves ordering and retries guarantees by
    /// using a single TCP channel to send the values. The recipient is guaranteed to receive a _prefix_
    /// or the sent messages; if the TCP connection is dropped no further messages will be sent.
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
                metadata: other.new_node_metadata::<T>(),
            },
        )
    }

    /// Broadcasts elements of this stream to all members of a cluster by sending them over the network,
    /// using [`bincode`] to serialize/deserialize messages. Each element in the stream will be sent to
    /// **every** member of the cluster based on the latest membership information.
    ///
    /// This is a common pattern in distributed systems for broadcasting data to all nodes in a cluster.
    /// Unlike [`Stream::demux_bincode`], which requires `(MemberId, T)` tuples to target specific members,
    /// `broadcast_bincode` takes a stream of **only data elements** and sends each element to all cluster members.
    ///
    /// # Non-Determinism
    /// The set of cluster members may asynchronously change over time. Each element is only broadcast
    /// to the current cluster members _at that point in time_. Depending on when we are notified of
    /// membership changes, we will broadcast each element to different members, so each member may receive
    /// different elements.
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
    /// on_worker.send_bincode(&p2)
    /// # .entries()
    /// // if there are 4 members in the cluster, we should receive 4 elements
    /// // { MemberId::<()>(0): [123], MemberId::<()>(1): [123], MemberId::<()>(2): [123], MemberId::<()>(3): [123] }
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
        let current_members = ids.snapshot(&join_tick, nondet_membership).keys();

        current_members
            .weaker_retries()
            .assume_ordering::<TotalOrder>(
                nondet!(/** we send to each member independently, order does not matter */),
            )
            .cross_product_nested_loop(
                self.batch(&join_tick, nondet_membership)
                    .assume_ordering::<TotalOrder>(
                        nondet!(/** we weaken the ordering back later */),
                    ),
            )
            .assume_ordering::<O>(nondet!(/** strictly weaker than TotalOrder */))
            .all_ticks()
            .demux_bincode(other)
    }

    #[expect(missing_docs, reason = "TODO")]
    pub fn send_bincode_external<L2>(self, other: &External<L2>) -> ExternalBincodeStream<T>
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
            input: Box::new(HydroNode::Unpersist {
                inner: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata::<T>(),
            }),
            op_metadata: HydroIrOpMetadata::new(),
        });

        ExternalBincodeStream {
            process_id: other.id,
            port_id: external_key,
            _phantom: PhantomData,
        }
    }
}

#[expect(missing_docs, reason = "TODO")]
impl<'a, T, L, L2, B: Boundedness, O, R> Stream<(MemberId<L2>, T), Process<'a, L>, B, O, R> {
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

#[expect(missing_docs, reason = "TODO")]
impl<'a, T, L, B: Boundedness> Stream<T, Process<'a, L>, B, TotalOrder, ExactlyOnce> {
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
            .assume_ordering(
                nondet!(/** safe to assume ordering because each output is independent */),
            )
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

#[expect(missing_docs, reason = "TODO")]
impl<'a, T, L, B: Boundedness, O, R> Stream<T, Cluster<'a, L>, B, O, R> {
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
                metadata: other.new_node_metadata::<(MemberId<L>, T)>(),
            },
        );

        raw_stream.into_keyed()
    }

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
        let current_members = ids.snapshot(&join_tick, nondet_membership).keys();

        current_members
            .weaker_retries()
            .assume_ordering::<TotalOrder>(
                nondet!(/** we send to each member independently, order does not matter */),
            )
            .cross_product_nested_loop(
                self.batch(&join_tick, nondet_membership)
                    .assume_ordering::<TotalOrder>(
                        nondet!(/** we weaken the ordering back later */),
                    ),
            )
            .assume_ordering::<O>(nondet!(/** strictly weaker than TotalOrder */))
            .all_ticks()
            .demux_bincode(other)
    }
}

#[expect(missing_docs, reason = "TODO")]
impl<'a, T, L, L2, B: Boundedness, O, R> Stream<(MemberId<L2>, T), Cluster<'a, L>, B, O, R> {
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
