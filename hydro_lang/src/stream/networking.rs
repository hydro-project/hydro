use std::marker::PhantomData;

use bytes::Bytes;
use serde::Serialize;
use serde::de::DeserializeOwned;
use stageleft::{q, quote_type};
use syn::parse_quote;

use crate::ir::{DebugInstantiate, HydroLeaf, HydroNode};
use crate::keyed_stream::KeyedStream;
use crate::location::external_process::{ExternalBincodeStream, ExternalBytesPort};
use crate::location::{CanSend, NoTick};
use crate::staging_util::get_this_crate;
use crate::stream::{ExactlyOnce, MinOrder};
use crate::{Cluster, ClusterId, External, Location, Process, Stream, TotalOrder, Unbounded};

pub fn serialize_bincode_with_type(is_demux: bool, t_type: &syn::Type) -> syn::Expr {
    let root = get_this_crate();

    if is_demux {
        parse_quote! {
            ::#root::runtime_support::stageleft::runtime_support::fn1_type_hint::<(#root::ClusterId<_>, #t_type), _>(
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
                (#root::ClusterId::<#c_type>::from_raw(id), #root::runtime_support::bincode::deserialize::<#t_type>(&b).unwrap())
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

impl<'a, T, L, B, O, R> Stream<T, Cluster<'a, L>, B, O, R> {
    pub fn send_bincode<L2>(
        self,
        other: &Process<'a, L2>,
    ) -> KeyedStream<ClusterId<L>, T, Process<'a, L2>, Unbounded, O, R>
    where
        T: Serialize + DeserializeOwned,
    {
        let serialize_pipeline = Some(serialize_bincode::<T>(false));

        let deserialize_pipeline = Some(deserialize_bincode::<T>(Some(&quote_type::<L>())));

        let raw_stream: Stream<(ClusterId<L>, T), Process<'a, L2>, Unbounded, O, R> = Stream::new(
            other.clone(),
            HydroNode::Network {
                serialize_fn: serialize_pipeline.map(|e| e.into()),
                instantiate_fn: DebugInstantiate::Building,
                deserialize_fn: deserialize_pipeline.map(|e| e.into()),
                input: Box::new(self.ir_node.into_inner()),
                metadata: other.new_node_metadata::<(ClusterId<L>, T)>(),
            },
        );

        raw_stream.into_keyed()
    }

    pub fn broadcast_bincode<L2: 'a>(
        self,
        other: &Cluster<'a, L2>,
    ) -> KeyedStream<ClusterId<L>, T, Cluster<'a, L2>, Unbounded, O, R>
    where
        T: Clone + Serialize + DeserializeOwned,
    {
        let ids = other.members();
        self.flat_map_ordered(q!(|v| { ids.iter().map(move |id| (*id, v.clone())) }))
            .demux_bincode(other)
    }
}

impl<'a, T, L, L2, B, O, R> Stream<(ClusterId<L2>, T), Process<'a, L>, B, O, R> {
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
                input: Box::new(self.ir_node.into_inner()),
                metadata: other.new_node_metadata::<T>(),
            },
        )
    }
}

impl<'a, T, L, B> Stream<T, Process<'a, L>, B, TotalOrder, ExactlyOnce> {
    pub fn round_robin_bincode<L2: 'a>(
        self,
        other: &Cluster<'a, L2>,
    ) -> Stream<T, Cluster<'a, L2>, Unbounded, TotalOrder, ExactlyOnce>
    where
        T: Serialize + DeserializeOwned,
    {
        let ids = other.members();

        self.enumerate()
            .map(q!(|(i, w)| (ids[i % ids.len()], w)))
            .demux_bincode(other)
    }
}

impl<'a, T, L, L2, B, O, R> Stream<(ClusterId<L2>, T), Cluster<'a, L>, B, O, R> {
    pub fn demux_bincode(
        self,
        other: &Cluster<'a, L2>,
    ) -> KeyedStream<ClusterId<L>, T, Cluster<'a, L2>, Unbounded, O, R>
    where
        T: Serialize + DeserializeOwned,
    {
        let serialize_pipeline = Some(serialize_bincode::<T>(true));

        let deserialize_pipeline = Some(deserialize_bincode::<T>(Some(&quote_type::<L>())));

        let raw_stream: Stream<(ClusterId<L>, T), Cluster<'a, L2>, Unbounded, O, R> = Stream::new(
            other.clone(),
            HydroNode::Network {
                serialize_fn: serialize_pipeline.map(|e| e.into()),
                instantiate_fn: DebugInstantiate::Building,
                deserialize_fn: deserialize_pipeline.map(|e| e.into()),
                input: Box::new(self.ir_node.into_inner()),
                metadata: other.new_node_metadata::<(ClusterId<L>, T)>(),
            },
        );

        raw_stream.into_keyed()
    }
}

impl<'a, T, L, B, O, R> Stream<T, Process<'a, L>, B, O, R> {
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
    ) -> Stream<T, Cluster<'a, L2>, Unbounded, O, R>
    where
        T: Clone + Serialize + DeserializeOwned,
    {
        let ids = other.members();
        self.flat_map_ordered(q!(|v| { ids.iter().map(move |id| (*id, v.clone())) }))
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

        let leaves = flow_state_borrow.leaves.as_mut().expect("Attempted to add a leaf to a flow that has already been finalized. No leaves can be added after the flow has been compiled()");

        leaves.push(HydroLeaf::SendExternal {
            to_external_id: other.id,
            to_key: external_key,
            to_many: false,
            serialize_fn: serialize_pipeline.map(|e| e.into()),
            instantiate_fn: DebugInstantiate::Building,
            input: Box::new(HydroNode::Unpersist {
                inner: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata::<T>(),
            }),
        });

        ExternalBincodeStream {
            process_id: other.id,
            port_id: external_key,
            _phantom: PhantomData,
        }
    }
}

impl<'a, T, L, B, O, R> Stream<T, L, B, O, R>
where
    L: Location<'a> + NoTick,
{
    #[expect(
        clippy::type_complexity,
        reason = "Complex signatures for CanSend trait"
    )]
    pub fn send_bytes<L2>(
        self,
        other: &L2,
    ) -> Stream<<L::Root as CanSend<'a, L2>>::Out<Bytes>, L2, Unbounded, O::Min, R>
    where
        L2: Location<'a>,
        L::Root: CanSend<'a, L2, In<Bytes> = T>,
        O: MinOrder<<L::Root as CanSend<'a, L2>>::OutStrongestOrder<O>>,
    {
        let root = get_this_crate();
        Stream::new(
            other.clone(),
            HydroNode::Network {
                serialize_fn: None,
                instantiate_fn: DebugInstantiate::Building,
                deserialize_fn: if let Some(c_type) = L::Root::tagged_type() {
                    let expr: syn::Expr = parse_quote!(|(id, b)| (#root::ClusterId<#c_type>::from_raw(id), b.unwrap().freeze()));
                    Some(expr.into())
                } else {
                    let expr: syn::Expr = parse_quote!(|b| b.unwrap().freeze());
                    Some(expr.into())
                },
                input: Box::new(self.ir_node.into_inner()),
                metadata: other.new_node_metadata::<Bytes>(),
            },
        )
    }

    pub fn send_bytes_external<L2>(self, other: &External<L2>) -> ExternalBytesPort
    where
        L2: 'a,
        L::Root: CanSend<'a, External<'a, L2>, In<Bytes> = T, Out<Bytes> = Bytes>,
    {
        let mut flow_state_borrow = self.location.flow_state().borrow_mut();
        let external_key = flow_state_borrow.next_external_out;
        flow_state_borrow.next_external_out += 1;

        let leaves = flow_state_borrow.leaves.as_mut().expect("Attempted to add a leaf to a flow that has already been finalized. No leaves can be added after the flow has been compiled()");

        leaves.push(HydroLeaf::SendExternal {
            to_external_id: other.id,
            to_key: external_key,
            to_many: false,
            serialize_fn: None,
            instantiate_fn: DebugInstantiate::Building,
            input: Box::new(HydroNode::Unpersist {
                inner: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata::<T>(),
            }),
        });

        ExternalBytesPort {
            process_id: other.id,
            port_id: external_key,
            _phantom: Default::default(),
        }
    }

    #[expect(clippy::type_complexity, reason = "ordering semantics for broadcast")]
    pub fn broadcast_bytes<C2>(
        self,
        other: &Cluster<'a, C2>,
    ) -> Stream<
        <L::Root as CanSend<'a, Cluster<'a, C2>>>::Out<Bytes>,
        Cluster<'a, C2>,
        Unbounded,
        O::Min,
        R,
    >
    where
        C2: 'a,
        L::Root: CanSend<'a, Cluster<'a, C2>, In<Bytes> = (ClusterId<C2>, T)>,
        T: Clone,
        O: MinOrder<<L::Root as CanSend<'a, Cluster<'a, C2>>>::OutStrongestOrder<O>>,
    {
        let ids = other.members();

        self.flat_map_ordered(q!(|b| ids.iter().map(move |id| (
            ::std::clone::Clone::clone(id),
            ::std::clone::Clone::clone(&b)
        ))))
        .send_bytes(other)
    }
}
