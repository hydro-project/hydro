use std::marker::PhantomData;

use serde::Serialize;
use serde::de::DeserializeOwned;
use stageleft::{q, quote_type};
use syn::parse_quote;

use crate::builder::ir::{DebugInstantiate, HydroIrOpMetadata, HydroNode, HydroRoot};
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

fn serialize_bincode<T: Serialize>(is_demux: bool) -> syn::Expr {
    serialize_bincode_with_type(is_demux, &quote_type::<T>())
}

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

impl<'a, T, L, L2, B: Boundedness, O, R> KeyedStream<MemberId<L2>, T, Process<'a, L>, B, O, R> {
    pub fn demux_bincode(
        self,
        other: &Cluster<'a, L2>,
    ) -> Stream<T, Cluster<'a, L2>, Unbounded, O, R>
    where
        T: Serialize + DeserializeOwned,
    {
        let serialize_pipeline = Some(serialize_bincode::<T>(true));

        let deserialize_pipeline = Some(deserialize_bincode::<T>(None));

        Stream::new(
            other.clone(),
            HydroNode::Network {
                serialize_fn: serialize_pipeline.map(|e| e.into()),
                instantiate_fn: DebugInstantiate::Building,
                deserialize_fn: deserialize_pipeline.map(|e| e.into()),
                input: Box::new(self.underlying.ir_node.into_inner()),
                metadata: other.new_node_metadata::<T>(),
            },
        )
    }
}

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

impl<'a, T, L, L2, B: Boundedness, O, R> KeyedStream<MemberId<L2>, T, Cluster<'a, L>, B, O, R> {
    pub fn demux_bincode(
        self,
        other: &Cluster<'a, L2>,
    ) -> KeyedStream<MemberId<L>, T, Cluster<'a, L2>, Unbounded, O, R>
    where
        T: Serialize + DeserializeOwned,
    {
        let serialize_pipeline = Some(serialize_bincode::<T>(true));

        let deserialize_pipeline = Some(deserialize_bincode::<T>(Some(&quote_type::<L>())));

        let raw_stream: Stream<(MemberId<L>, T), Cluster<'a, L2>, Unbounded, O, R> = Stream::new(
            other.clone(),
            HydroNode::Network {
                serialize_fn: serialize_pipeline.map(|e| e.into()),
                instantiate_fn: DebugInstantiate::Building,
                deserialize_fn: deserialize_pipeline.map(|e| e.into()),
                input: Box::new(self.underlying.ir_node.into_inner()),
                metadata: other.new_node_metadata::<(MemberId<L>, T)>(),
            },
        );

        raw_stream.into_keyed()
    }
}

impl<'a, T, L, B: Boundedness, O, R> Stream<T, Process<'a, L>, B, O, R> {
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

    pub fn send_bincode_external<L2>(self, other: &External<L2>) -> ExternalBincodeStream<T>
    where
        T: Serialize + DeserializeOwned,
    {
        let serialize_pipeline = Some(serialize_bincode::<T>(false));

        let mut flow_state_borrow = self.location.flow_state().borrow_mut();

        let external_key = flow_state_borrow.next_external_out;
        flow_state_borrow.next_external_out += 1;

        let roots = flow_state_borrow.roots.as_mut().expect("Attempted to add a root to a flow that has already been finalized. No roots can be added after the flow has been compiled()");

        roots.push(HydroRoot::SendExternal {
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
