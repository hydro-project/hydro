use std::fmt::Debug;
use std::marker::PhantomData;
use std::time::Duration;

use bytes::{Bytes, BytesMut};
use futures::stream::Stream as FuturesStream;
use proc_macro2::Span;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use stageleft::{QuotedWithContext, q, quote_type};
use syn::parse_quote;
use tokio_util::codec::{Decoder, Encoder, LengthDelimitedCodec};

use super::builder::FlowState;
use crate::backtrace::get_backtrace;
use crate::cycle::{CycleCollection, ForwardRef, ForwardRefMarker};
use crate::ir::{DebugInstantiate, HydroIrMetadata, HydroLeaf, HydroNode, HydroSource};
use crate::keyed_stream::KeyedStream;
use crate::location::cluster::ClusterIds;
use crate::location::external_process::{
    ExternalBincodeBidi, ExternalBincodeSink, ExternalBytesPort, Many,
};
use crate::staging_util::get_this_crate;
use crate::stream::ExactlyOnce;
use crate::unsafety::NonDet;
use crate::{NoOrder, Singleton, Stream, TotalOrder, Unbounded, nondet};

pub mod external_process;
pub use external_process::External;

pub mod process;
pub use process::Process;

pub mod cluster;
pub use cluster::{Cluster, ClusterId};

pub mod tick;
pub use tick::{Atomic, NoTick, Tick};

#[derive(PartialEq, Eq, Clone, Debug, Hash, Serialize, Deserialize)]
pub enum LocationId {
    Process(usize),
    Cluster(usize),
    Tick(usize, Box<LocationId>),
}

#[derive(PartialEq, Eq, Clone, Debug, Hash, Serialize, Deserialize)]
pub enum MembershipEvent {
    Joined,
    Left,
}

impl LocationId {
    pub fn root(&self) -> &LocationId {
        match self {
            LocationId::Process(_) => self,
            LocationId::Cluster(_) => self,
            LocationId::Tick(_, id) => id.root(),
        }
    }

    pub fn is_root(&self) -> bool {
        match self {
            LocationId::Process(_) | LocationId::Cluster(_) => true,
            LocationId::Tick(_, _) => false,
        }
    }

    pub fn raw_id(&self) -> usize {
        match self {
            LocationId::Process(id) => *id,
            LocationId::Cluster(id) => *id,
            LocationId::Tick(_, _) => panic!("cannot get raw id for tick"),
        }
    }

    pub fn swap_root(&mut self, new_root: LocationId) {
        match self {
            LocationId::Tick(_, id) => {
                id.swap_root(new_root);
            }
            _ => {
                assert!(new_root.is_root());
                *self = new_root;
            }
        }
    }
}

pub fn check_matching_location<'a, L: Location<'a>>(l1: &L, l2: &L) {
    assert_eq!(l1.id(), l2.id(), "locations do not match");
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NetworkHint {
    Auto,
    TcpPort(Option<u16>),
}

pub trait Location<'a>: Clone {
    type Root: Location<'a>;

    fn root(&self) -> Self::Root;

    fn id(&self) -> LocationId;

    fn flow_state(&self) -> &FlowState;

    fn is_top_level() -> bool;

    fn tick(&self) -> Tick<Self>
    where
        Self: NoTick,
    {
        let next_id = self.flow_state().borrow_mut().next_clock_id;
        self.flow_state().borrow_mut().next_clock_id += 1;
        Tick {
            id: next_id,
            l: self.clone(),
        }
    }

    fn next_node_id(&self) -> usize {
        let next_id = self.flow_state().borrow_mut().next_node_id;
        self.flow_state().borrow_mut().next_node_id += 1;
        next_id
    }

    #[inline(never)]
    fn new_node_metadata<T>(&self) -> HydroIrMetadata {
        HydroIrMetadata {
            location_kind: self.id(),
            backtrace: get_backtrace(2),
            output_type: Some(quote_type::<T>().into()),
            cardinality: None,
            cpu_usage: None,
            network_recv_cpu_usage: None,
            id: None,
            tag: None,
        }
    }

    fn spin(&self) -> Stream<(), Self, Unbounded, TotalOrder, ExactlyOnce>
    where
        Self: Sized + NoTick,
    {
        Stream::new(
            self.clone(),
            HydroNode::Persist {
                inner: Box::new(HydroNode::Source {
                    source: HydroSource::Spin(),
                    metadata: self.new_node_metadata::<()>(),
                }),
                metadata: self.new_node_metadata::<()>(),
            },
        )
    }

    fn source_stream<T, E>(
        &self,
        e: impl QuotedWithContext<'a, E, Self>,
    ) -> Stream<T, Self, Unbounded, TotalOrder, ExactlyOnce>
    where
        E: FuturesStream<Item = T> + Unpin,
        Self: Sized + NoTick,
    {
        let e = e.splice_untyped_ctx(self);

        Stream::new(
            self.clone(),
            HydroNode::Persist {
                inner: Box::new(HydroNode::Source {
                    source: HydroSource::Stream(e.into()),
                    metadata: self.new_node_metadata::<T>(),
                }),
                metadata: self.new_node_metadata::<T>(),
            },
        )
    }

    fn source_iter<T, E>(
        &self,
        e: impl QuotedWithContext<'a, E, Self>,
    ) -> Stream<T, Self, Unbounded, TotalOrder, ExactlyOnce>
    where
        E: IntoIterator<Item = T>,
        Self: Sized + NoTick,
    {
        // TODO(shadaj): we mark this as unbounded because we do not yet have a representation
        // for bounded top-level streams, and this is the only way to generate one
        let e = e.splice_untyped_ctx(self);

        Stream::new(
            self.clone(),
            HydroNode::Persist {
                inner: Box::new(HydroNode::Source {
                    source: HydroSource::Iter(e.into()),
                    metadata: self.new_node_metadata::<T>(),
                }),
                metadata: self.new_node_metadata::<T>(),
            },
        )
    }

    fn source_cluster_members<C: 'a>(
        &self,
        cluster: &Cluster<'a, C>,
    ) -> KeyedStream<ClusterId<C>, MembershipEvent, Self, Unbounded>
    where
        Self: Sized + NoTick,
    {
        let underlying_clusterids: ClusterIds<'a, C> = ClusterIds {
            id: cluster.id,
            _phantom: PhantomData,
        };

        self.source_iter(q!(underlying_clusterids))
            .map(q!(|id| (*id, MembershipEvent::Joined)))
            .into_keyed()
    }

    fn source_external_bytes<L>(
        &self,
        from: &External<L>,
    ) -> (
        ExternalBytesPort,
        Stream<std::io::Result<BytesMut>, Self, Unbounded, TotalOrder, ExactlyOnce>,
    )
    where
        Self: Sized + NoTick,
    {
        let next_external_port_id = {
            let mut flow_state = from.flow_state.borrow_mut();
            let id = flow_state.next_external_out;
            flow_state.next_external_out += 1;
            id
        };

        (
            ExternalBytesPort {
                process_id: from.id,
                port_id: next_external_port_id,
                _phantom: Default::default(),
            },
            Stream::new(
                self.clone(),
                HydroNode::Persist {
                    inner: Box::new(HydroNode::ExternalInput {
                        from_external_id: from.id,
                        from_key: next_external_port_id,
                        from_many: false,
                        codec_type: quote_type::<LengthDelimitedCodec>().into(),
                        port_hint: NetworkHint::Auto,
                        instantiate_fn: DebugInstantiate::Building,
                        deserialize_fn: None,
                        metadata: self.new_node_metadata::<std::io::Result<BytesMut>>(),
                    }),
                    metadata: self.new_node_metadata::<std::io::Result<BytesMut>>(),
                },
            ),
        )
    }

    fn source_external_bincode<L, T>(
        &self,
        from: &External<L>,
    ) -> (
        ExternalBincodeSink<T>,
        Stream<T, Self, Unbounded, TotalOrder, ExactlyOnce>,
    )
    where
        Self: Sized + NoTick,
        T: Serialize + DeserializeOwned,
    {
        let next_external_port_id = {
            let mut flow_state = from.flow_state.borrow_mut();
            let id = flow_state.next_external_out;
            flow_state.next_external_out += 1;
            id
        };

        (
            ExternalBincodeSink {
                process_id: from.id,
                port_id: next_external_port_id,
                _phantom: PhantomData,
            },
            Stream::new(
                self.clone(),
                HydroNode::Persist {
                    inner: Box::new(HydroNode::ExternalInput {
                        from_external_id: from.id,
                        from_key: next_external_port_id,
                        from_many: false,
                        codec_type: quote_type::<LengthDelimitedCodec>().into(),
                        port_hint: NetworkHint::Auto,
                        instantiate_fn: DebugInstantiate::Building,
                        deserialize_fn: Some(
                            crate::stream::networking::deserialize_bincode::<T>(None).into(),
                        ),
                        metadata: self.new_node_metadata::<T>(),
                    }),
                    metadata: self.new_node_metadata::<T>(),
                },
            ),
        )
    }

    #[expect(clippy::type_complexity, reason = "stream markers")]
    fn bidi_external_many_bytes<L, T, Codec: Encoder<T> + Decoder>(
        &self,
        from: &External<L>,
        port_hint: NetworkHint,
    ) -> (
        ExternalBytesPort<Many>,
        KeyedStream<u64, <Codec as Decoder>::Item, Self, Unbounded, TotalOrder, ExactlyOnce>,
        KeyedStream<u64, MembershipEvent, Self, Unbounded, TotalOrder, ExactlyOnce>,
        ForwardRef<'a, KeyedStream<u64, T, Self, Unbounded, NoOrder, ExactlyOnce>>,
    )
    where
        Self: Sized + NoTick,
    {
        let next_external_port_id = {
            let mut flow_state = from.flow_state.borrow_mut();
            let id = flow_state.next_external_out;
            flow_state.next_external_out += 1;
            id
        };

        let (fwd_ref, to_sink) =
            self.forward_ref::<KeyedStream<u64, T, Self, Unbounded, NoOrder, ExactlyOnce>>();
        let mut flow_state_borrow = self.flow_state().borrow_mut();

        let leaves = flow_state_borrow.leaves.as_mut().expect("Attempted to add a leaf to a flow that has already been finalized. No leaves can be added after the flow has been compiled()");

        leaves.push(HydroLeaf::SendExternal {
            to_external_id: from.id,
            to_key: next_external_port_id,
            to_many: true,
            serialize_fn: None,
            instantiate_fn: DebugInstantiate::Building,
            input: Box::new(HydroNode::Unpersist {
                inner: Box::new(to_sink.entries().ir_node.into_inner()),
                metadata: self.new_node_metadata::<(u64, T)>(),
            }),
        });

        let raw_stream: Stream<
            Result<(u64, <Codec as Decoder>::Item), <Codec as Decoder>::Error>,
            Self,
            Unbounded,
            NoOrder,
            ExactlyOnce,
        > = Stream::new(
            self.clone(),
            HydroNode::Persist {
                inner: Box::new(HydroNode::ExternalInput {
                    from_external_id: from.id,
                    from_key: next_external_port_id,
                    from_many: true,
                    codec_type: quote_type::<Codec>().into(),
                    port_hint,
                    instantiate_fn: DebugInstantiate::Building,
                    deserialize_fn: None,
                    metadata: self
                        .new_node_metadata::<std::io::Result<(u64, <Codec as Decoder>::Item)>>(),
                }),
                metadata: self
                    .new_node_metadata::<std::io::Result<(u64, <Codec as Decoder>::Item)>>(),
            },
        );

        let membership_stream_ident = syn::Ident::new(
            &format!(
                "__hydro_deploy_many_{}_{}_membership",
                from.id, next_external_port_id
            ),
            Span::call_site(),
        );
        let membership_stream_expr: syn::Expr = parse_quote!(#membership_stream_ident);
        let raw_membership_stream: Stream<(u64, bool), Self, Unbounded, TotalOrder, ExactlyOnce> =
            Stream::new(
                self.clone(),
                HydroNode::Persist {
                    inner: Box::new(HydroNode::Source {
                        source: HydroSource::Stream(membership_stream_expr.into()),
                        metadata: self.new_node_metadata::<(u64, bool)>(),
                    }),
                    metadata: self.new_node_metadata::<(u64, bool)>(),
                },
            );

        (
            ExternalBytesPort {
                process_id: from.id,
                port_id: next_external_port_id,
                _phantom: PhantomData,
            },
            raw_stream
                .flatten_ordered() // TODO(shadaj): this silently drops framing errors, decide on right defaults
                .into_keyed()
                .assume_ordering::<TotalOrder>(
                    nondet!(/** order of messages is deterministic within each key due to TCP */)
                ),
            raw_membership_stream
                .into_keyed()
                .assume_ordering::<TotalOrder>(
                    nondet!(/** membership events are ordered within each key */),
                )
                .map(q!(|join| {
                    if join {
                        MembershipEvent::Joined
                    } else {
                        MembershipEvent::Left
                    }
                })),
            fwd_ref,
        )
    }

    #[expect(clippy::type_complexity, reason = "stream markers")]
    fn bidi_external_many_bincode<L, InT: DeserializeOwned, OutT: Serialize>(
        &self,
        from: &External<L>,
    ) -> (
        ExternalBincodeBidi<InT, OutT, Many>,
        KeyedStream<u64, InT, Self, Unbounded, TotalOrder, ExactlyOnce>,
        KeyedStream<u64, MembershipEvent, Self, Unbounded, TotalOrder, ExactlyOnce>,
        ForwardRef<'a, KeyedStream<u64, OutT, Self, Unbounded, NoOrder, ExactlyOnce>>,
    )
    where
        Self: Sized + NoTick,
    {
        let next_external_port_id = {
            let mut flow_state = from.flow_state.borrow_mut();
            let id = flow_state.next_external_out;
            flow_state.next_external_out += 1;
            id
        };

        let root = get_this_crate();

        let (fwd_ref, to_sink) =
            self.forward_ref::<KeyedStream<u64, OutT, Self, Unbounded, NoOrder, ExactlyOnce>>();
        let mut flow_state_borrow = self.flow_state().borrow_mut();

        let leaves = flow_state_borrow.leaves.as_mut().expect("Attempted to add a leaf to a flow that has already been finalized. No leaves can be added after the flow has been compiled()");

        let out_t_type = quote_type::<OutT>();
        let ser_fn: syn::Expr = syn::parse_quote! {
            ::#root::runtime_support::stageleft::runtime_support::fn1_type_hint::<(u64, #out_t_type), _>(
                |(id, b)| (id, #root::runtime_support::bincode::serialize(&b).unwrap().into())
            )
        };

        leaves.push(HydroLeaf::SendExternal {
            to_external_id: from.id,
            to_key: next_external_port_id,
            to_many: true,
            serialize_fn: Some(ser_fn.into()),
            instantiate_fn: DebugInstantiate::Building,
            input: Box::new(HydroNode::Unpersist {
                inner: Box::new(to_sink.entries().ir_node.into_inner()),
                metadata: self.new_node_metadata::<(u64, Bytes)>(),
            }),
        });

        let in_t_type = quote_type::<InT>();

        let deser_fn: syn::Expr = syn::parse_quote! {
            |res| {
                let (id, b) = res.unwrap();
                (id, #root::runtime_support::bincode::deserialize::<#in_t_type>(&b).unwrap())
            }
        };

        let raw_stream: Stream<(u64, InT), Self, Unbounded, NoOrder, ExactlyOnce> = Stream::new(
            self.clone(),
            HydroNode::Persist {
                inner: Box::new(HydroNode::ExternalInput {
                    from_external_id: from.id,
                    from_key: next_external_port_id,
                    from_many: true,
                    codec_type: quote_type::<LengthDelimitedCodec>().into(),
                    port_hint: NetworkHint::Auto,
                    instantiate_fn: DebugInstantiate::Building,
                    deserialize_fn: Some(deser_fn.into()),
                    metadata: self.new_node_metadata::<(u64, InT)>(),
                }),
                metadata: self.new_node_metadata::<(u64, InT)>(),
            },
        );

        let membership_stream_ident = syn::Ident::new(
            &format!(
                "__hydro_deploy_many_{}_{}_membership",
                from.id, next_external_port_id
            ),
            Span::call_site(),
        );
        let membership_stream_expr: syn::Expr = parse_quote!(#membership_stream_ident);
        let raw_membership_stream: Stream<(u64, bool), Self, Unbounded, NoOrder, ExactlyOnce> =
            Stream::new(
                self.clone(),
                HydroNode::Persist {
                    inner: Box::new(HydroNode::Source {
                        source: HydroSource::Stream(membership_stream_expr.into()),
                        metadata: self.new_node_metadata::<(u64, bool)>(),
                    }),
                    metadata: self.new_node_metadata::<(u64, bool)>(),
                },
            );

        (
            ExternalBincodeBidi {
                process_id: from.id,
                port_id: next_external_port_id,
                _phantom: PhantomData,
            },
            raw_stream.into_keyed().assume_ordering::<TotalOrder>(
                nondet!(/** order of messages is deterministic within each key due to TCP */),
            ),
            raw_membership_stream
                .into_keyed()
                .assume_ordering::<TotalOrder>(
                    nondet!(/** membership events are ordered within each key */),
                )
                .map(q!(|join| {
                    if join {
                        MembershipEvent::Joined
                    } else {
                        MembershipEvent::Left
                    }
                })),
            fwd_ref,
        )
    }

    fn singleton<T>(&self, e: impl QuotedWithContext<'a, T, Self>) -> Singleton<T, Self, Unbounded>
    where
        T: Clone,
        Self: Sized + NoTick,
    {
        // TODO(shadaj): we mark this as unbounded because we do not yet have a representation
        // for bounded top-level singletons, and this is the only way to generate one

        let e_arr = q!([e]);
        let e = e_arr.splice_untyped_ctx(self);

        // we do a double persist here because if the singleton shows up on every tick,
        // we first persist the source so that we store that value and then persist again
        // so that it grows every tick
        Singleton::new(
            self.clone(),
            HydroNode::Persist {
                inner: Box::new(HydroNode::Persist {
                    inner: Box::new(HydroNode::Source {
                        source: HydroSource::Iter(e.into()),
                        metadata: self.new_node_metadata::<T>(),
                    }),
                    metadata: self.new_node_metadata::<T>(),
                }),
                metadata: self.new_node_metadata::<T>(),
            },
        )
    }

    /// Generates a stream with values emitted at a fixed interval, with
    /// each value being the current time (as an [`tokio::time::Instant`]).
    ///
    /// The clock source used is monotonic, so elements will be emitted in
    /// increasing order.
    ///
    /// # Non-Determinism
    /// Because this stream is generated by an OS timer, it will be
    /// non-deterministic because each timestamp will be arbitrary.
    fn source_interval(
        &self,
        interval: impl QuotedWithContext<'a, Duration, Self> + Copy + 'a,
        _nondet: NonDet,
    ) -> Stream<tokio::time::Instant, Self, Unbounded, TotalOrder, ExactlyOnce>
    where
        Self: Sized + NoTick,
    {
        self.source_stream(q!(tokio_stream::wrappers::IntervalStream::new(
            tokio::time::interval(interval)
        )))
    }

    /// Generates a stream with values emitted at a fixed interval (with an
    /// initial delay), with each value being the current time
    /// (as an [`tokio::time::Instant`]).
    ///
    /// The clock source used is monotonic, so elements will be emitted in
    /// increasing order.
    ///
    /// # Non-Determinism
    /// Because this stream is generated by an OS timer, it will be
    /// non-deterministic because each timestamp will be arbitrary.
    fn source_interval_delayed(
        &self,
        delay: impl QuotedWithContext<'a, Duration, Self> + Copy + 'a,
        interval: impl QuotedWithContext<'a, Duration, Self> + Copy + 'a,
        _nondet: NonDet,
    ) -> Stream<tokio::time::Instant, Self, Unbounded, TotalOrder, ExactlyOnce>
    where
        Self: Sized + NoTick,
    {
        self.source_stream(q!(tokio_stream::wrappers::IntervalStream::new(
            tokio::time::interval_at(tokio::time::Instant::now() + delay, interval)
        )))
    }

    fn forward_ref<S>(&self) -> (ForwardRef<'a, S>, S)
    where
        S: CycleCollection<'a, ForwardRefMarker, Location = Self>,
        Self: NoTick,
    {
        let next_id = self.flow_state().borrow_mut().next_cycle_id();
        let ident = syn::Ident::new(&format!("cycle_{}", next_id), Span::call_site());

        (
            ForwardRef {
                completed: false,
                ident: ident.clone(),
                expected_location: self.id(),
                _phantom: PhantomData,
            },
            S::create_source(ident, self.clone()),
        )
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use futures::{SinkExt, StreamExt};
    use hydro_deploy::Deployment;
    use stageleft::q;
    use tokio_util::codec::LengthDelimitedCodec;

    use crate::{FlowBuilder, Location, NetworkHint};

    #[tokio::test]
    async fn external_bytes() {
        let mut deployment = Deployment::new();

        let flow = FlowBuilder::new();
        let first_node = flow.process::<()>();
        let external = flow.external::<()>();

        let (in_port, input) = first_node.source_external_bytes(&external);
        let out = input
            .map(q!(|r| r.unwrap()))
            .send_bincode_external(&external);

        let nodes = flow
            .with_process(&first_node, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let mut external_in = nodes.connect_sink_bytes(in_port).await;
        let mut external_out = nodes.connect_source_bincode(out).await;

        deployment.start().await.unwrap();

        external_in.send(vec![1, 2, 3].into()).await.unwrap();

        assert_eq!(external_out.next().await.unwrap(), vec![1, 2, 3]);
    }

    #[tokio::test]
    async fn multi_external_source() {
        let mut deployment = Deployment::new();

        let flow = FlowBuilder::new();
        let first_node = flow.process::<()>();
        let external = flow.external::<()>();

        let (in_port, input, _membership, complete_sink) =
            first_node.bidi_external_many_bincode(&external);
        let out = input.entries().send_bincode_external(&external);
        complete_sink.complete(first_node.source_iter::<(u64, ()), _>(q!([])).into_keyed());

        let nodes = flow
            .with_process(&first_node, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let (_, mut external_in_1) = nodes.connect_bincode(in_port.clone()).await;
        let (_, mut external_in_2) = nodes.connect_bincode(in_port).await;
        let external_out = nodes.connect_source_bincode(out).await;

        deployment.start().await.unwrap();

        external_in_1.send(123).await.unwrap();
        external_in_2.send(456).await.unwrap();

        assert_eq!(
            external_out.take(2).collect::<HashSet<_>>().await,
            vec![(0, 123), (1, 456)].into_iter().collect()
        );
    }

    #[tokio::test]
    async fn second_connection_only_multi_source() {
        let mut deployment = Deployment::new();

        let flow = FlowBuilder::new();
        let first_node = flow.process::<()>();
        let external = flow.external::<()>();

        let (in_port, input, _membership, complete_sink) =
            first_node.bidi_external_many_bincode(&external);
        let out = input.entries().send_bincode_external(&external);
        complete_sink.complete(first_node.source_iter::<(u64, ()), _>(q!([])).into_keyed());

        let nodes = flow
            .with_process(&first_node, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        // intentionally skipped to test stream waking logic
        let (_, mut _external_in_1) = nodes.connect_bincode(in_port.clone()).await;
        let (_, mut external_in_2) = nodes.connect_bincode(in_port).await;
        let mut external_out = nodes.connect_source_bincode(out).await;

        deployment.start().await.unwrap();

        external_in_2.send(456).await.unwrap();

        assert_eq!(external_out.next().await.unwrap(), (1, 456));
    }

    #[tokio::test]
    async fn multi_external_bytes() {
        let mut deployment = Deployment::new();

        let flow = FlowBuilder::new();
        let first_node = flow.process::<()>();
        let external = flow.external::<()>();

        let (in_port, input, _membership, complete_sink) = first_node
            .bidi_external_many_bytes::<_, _, LengthDelimitedCodec>(&external, NetworkHint::Auto);
        let out = input.entries().send_bincode_external(&external);
        complete_sink.complete(first_node.source_iter(q!([])).into_keyed());

        let nodes = flow
            .with_process(&first_node, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let mut external_in_1 = nodes.connect_sink_bytes(in_port.clone()).await;
        let mut external_in_2 = nodes.connect_sink_bytes(in_port).await;
        let external_out = nodes.connect_source_bincode(out).await;

        deployment.start().await.unwrap();

        external_in_1.send(vec![1, 2, 3].into()).await.unwrap();
        external_in_2.send(vec![4, 5].into()).await.unwrap();

        assert_eq!(
            external_out.take(2).collect::<HashSet<_>>().await,
            vec![
                (0, (&[1u8, 2, 3] as &[u8]).into()),
                (1, (&[4u8, 5] as &[u8]).into())
            ]
            .into_iter()
            .collect()
        );
    }

    #[tokio::test]
    async fn echo_external_bytes() {
        let mut deployment = Deployment::new();

        let flow = FlowBuilder::new();
        let first_node = flow.process::<()>();
        let external = flow.external::<()>();

        let (port, input, _membership, complete_sink) = first_node
            .bidi_external_many_bytes::<_, _, LengthDelimitedCodec>(&external, NetworkHint::Auto);
        complete_sink
            .complete(input.map(q!(|bytes| { bytes.into_iter().map(|x| x + 1).collect() })));

        let nodes = flow
            .with_process(&first_node, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let (mut external_out_1, mut external_in_1) = nodes.connect_bytes(port.clone()).await;
        let (mut external_out_2, mut external_in_2) = nodes.connect_bytes(port).await;

        deployment.start().await.unwrap();

        external_in_1.send(vec![1, 2, 3].into()).await.unwrap();
        external_in_2.send(vec![4, 5].into()).await.unwrap();

        assert_eq!(external_out_1.next().await.unwrap().unwrap(), vec![2, 3, 4]);
        assert_eq!(external_out_2.next().await.unwrap().unwrap(), vec![5, 6]);
    }

    #[tokio::test]
    async fn echo_external_bincode() {
        let mut deployment = Deployment::new();

        let flow = FlowBuilder::new();
        let first_node = flow.process::<()>();
        let external = flow.external::<()>();

        let (port, input, _membership, complete_sink) =
            first_node.bidi_external_many_bincode(&external);
        complete_sink.complete(input.map(q!(|text: String| { text.to_uppercase() })));

        let nodes = flow
            .with_process(&first_node, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let (mut external_out_1, mut external_in_1) = nodes.connect_bincode(port.clone()).await;
        let (mut external_out_2, mut external_in_2) = nodes.connect_bincode(port).await;

        deployment.start().await.unwrap();

        external_in_1.send("hi".to_string()).await.unwrap();
        external_in_2.send("hello".to_string()).await.unwrap();

        assert_eq!(external_out_1.next().await.unwrap(), "HI");
        assert_eq!(external_out_2.next().await.unwrap(), "HELLO");
    }
}
