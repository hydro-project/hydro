//! Type definitions for distributed locations, which specify where pieces of a Hydro
//! program will be executed.
//!
//! Hydro is a **global**, **distributed** programming model. This means that the data
//! and computation in a Hydro program can be spread across multiple machines, data
//! centers, and even continents. To achieve this, Hydro uses the concept of
//! **locations** to keep track of _where_ data is located and computation is executed.
//!
//! Each live collection type (in [`crate::live_collections`]) has a type parameter `L`
//! which will always be a type that implements the [`Location`] trait (e.g. [`Process`]
//! and [`Cluster`]). To create distributed programs, Hydro provides a variety of APIs
//! to allow live collections to be _moved_ between locations via network send/receive.
//!
//! See [the Hydro docs](https://hydro.run/docs/hydro/locations/) for more information.

use std::fmt::Debug;
use std::marker::PhantomData;
use std::time::Duration;

use bytes::BytesMut;
use futures::stream::Stream as FuturesStream;
use proc_macro2::Span;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use stageleft::{QuotedWithContext, q, quote_type};
use syn::parse_quote;
use tokio_util::codec::{Decoder, Encoder, LengthDelimitedCodec};

use crate::compile::ir::{DebugInstantiate, HydroIrOpMetadata, HydroNode, HydroRoot, HydroSource};
use crate::forward_handle::ForwardRef;
#[cfg(stageleft_runtime)]
use crate::forward_handle::{CycleCollection, ForwardHandle};
use crate::live_collections::boundedness::Unbounded;
use crate::live_collections::keyed_stream::KeyedStream;
use crate::live_collections::singleton::Singleton;
use crate::live_collections::stream::{
    ExactlyOnce, NoOrder, Ordering, Retries, Stream, TotalOrder,
};
use crate::location::cluster::ClusterIds;
use crate::location::dynamic::LocationId;
use crate::location::external_process::{
    ExternalBincodeBidi, ExternalBincodeSink, ExternalBytesPort, Many, NotMany,
};
use crate::nondet::NonDet;
use crate::staging_util::get_this_crate;

pub mod dynamic;

#[expect(missing_docs, reason = "TODO")]
pub mod external_process;
pub use external_process::External;

#[expect(missing_docs, reason = "TODO")]
pub mod process;
pub use process::Process;

#[expect(missing_docs, reason = "TODO")]
pub mod cluster;
pub use cluster::Cluster;

#[expect(missing_docs, reason = "TODO")]
pub mod member_id;
pub use member_id::MemberId;

#[expect(missing_docs, reason = "TODO")]
pub mod tick;
pub use tick::{Atomic, NoTick, Tick};

#[expect(missing_docs, reason = "TODO")]
#[derive(PartialEq, Eq, Clone, Debug, Hash, Serialize, Deserialize)]
pub enum MembershipEvent {
    Joined,
    Left,
}

#[expect(missing_docs, reason = "TODO")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NetworkHint {
    Auto,
    TcpPort(Option<u16>),
}

pub(crate) fn check_matching_location<'a, L: Location<'a>>(l1: &L, l2: &L) {
    assert_eq!(Location::id(l1), Location::id(l2), "locations do not match");
}

#[expect(missing_docs, reason = "TODO")]
#[expect(
    private_bounds,
    reason = "only internal Hydro code can define location types"
)]
pub trait Location<'a>: dynamic::DynLocation {
    type Root: Location<'a>;

    fn root(&self) -> Self::Root;

    fn try_tick(&self) -> Option<Tick<Self>> {
        if Self::is_top_level() {
            let next_id = self.flow_state().borrow_mut().next_clock_id;
            self.flow_state().borrow_mut().next_clock_id += 1;
            Some(Tick {
                id: next_id,
                l: self.clone(),
            })
        } else {
            None
        }
    }

    fn id(&self) -> LocationId {
        dynamic::DynLocation::id(self)
    }

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

    fn spin(&self) -> Stream<(), Self, Unbounded, TotalOrder, ExactlyOnce>
    where
        Self: Sized + NoTick,
    {
        Stream::new(
            self.clone(),
            HydroNode::Source {
                source: HydroSource::Spin(),
                metadata: self.new_node_metadata(Stream::<
                    (),
                    Self,
                    Unbounded,
                    TotalOrder,
                    ExactlyOnce,
                >::collection_kind()),
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
            HydroNode::Source {
                source: HydroSource::Stream(e.into()),
                metadata: self.new_node_metadata(Stream::<
                    T,
                    Self,
                    Unbounded,
                    TotalOrder,
                    ExactlyOnce,
                >::collection_kind()),
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
        let e = e.splice_typed_ctx(self);

        Stream::new(
            self.clone(),
            HydroNode::Source {
                source: HydroSource::Iter(e.into()),
                metadata: self.new_node_metadata(Stream::<
                    T,
                    Self,
                    Unbounded,
                    TotalOrder,
                    ExactlyOnce,
                >::collection_kind()),
            },
        )
    }

    fn source_cluster_members<C: 'a>(
        &self,
        cluster: &Cluster<'a, C>,
    ) -> KeyedStream<MemberId<C>, MembershipEvent, Self, Unbounded>
    where
        Self: Sized + NoTick,
    {
        let underlying_memberids: ClusterIds<'a, C> = ClusterIds {
            id: cluster.id,
            _phantom: PhantomData,
        };

        self.source_iter(q!(underlying_memberids))
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
                HydroNode::ExternalInput {
                    from_external_id: from.id,
                    from_key: next_external_port_id,
                    from_many: false,
                    codec_type: quote_type::<LengthDelimitedCodec>().into(),
                    port_hint: NetworkHint::Auto,
                    instantiate_fn: DebugInstantiate::Building,
                    deserialize_fn: None,
                    metadata: self.new_node_metadata(Stream::<
                        std::io::Result<BytesMut>,
                        Self,
                        Unbounded,
                        TotalOrder,
                        ExactlyOnce,
                    >::collection_kind()),
                },
            ),
        )
    }

    #[expect(clippy::type_complexity, reason = "stream markers")]
    fn source_external_bincode<L, T, O: Ordering, R: Retries>(
        &self,
        from: &External<L>,
    ) -> (
        ExternalBincodeSink<T, NotMany, O, R>,
        Stream<T, Self, Unbounded, O, R>,
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
                HydroNode::ExternalInput {
                    from_external_id: from.id,
                    from_key: next_external_port_id,
                    from_many: false,
                    codec_type: quote_type::<LengthDelimitedCodec>().into(),
                    port_hint: NetworkHint::Auto,
                    instantiate_fn: DebugInstantiate::Building,
                    deserialize_fn: Some(
                        crate::live_collections::stream::networking::deserialize_bincode::<T>(None)
                            .into(),
                    ),
                    metadata: self
                        .new_node_metadata(Stream::<T, Self, Unbounded, O, R>::collection_kind()),
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
        ForwardHandle<'a, KeyedStream<u64, T, Self, Unbounded, NoOrder, ExactlyOnce>>,
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

        flow_state_borrow.push_root(HydroRoot::SendExternal {
            to_external_id: from.id,
            to_key: next_external_port_id,
            to_many: true,
            serialize_fn: None,
            instantiate_fn: DebugInstantiate::Building,
            input: Box::new(to_sink.entries().ir_node.into_inner()),
            op_metadata: HydroIrOpMetadata::new(),
        });

        let raw_stream: Stream<
            Result<(u64, <Codec as Decoder>::Item), <Codec as Decoder>::Error>,
            Self,
            Unbounded,
            TotalOrder,
            ExactlyOnce,
        > = Stream::new(
            self.clone(),
            HydroNode::ExternalInput {
                from_external_id: from.id,
                from_key: next_external_port_id,
                from_many: true,
                codec_type: quote_type::<Codec>().into(),
                port_hint,
                instantiate_fn: DebugInstantiate::Building,
                deserialize_fn: None,
                metadata: self.new_node_metadata(Stream::<
                    Result<(u64, <Codec as Decoder>::Item), <Codec as Decoder>::Error>,
                    Self,
                    Unbounded,
                    TotalOrder,
                    ExactlyOnce,
                >::collection_kind()),
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
        let raw_membership_stream: KeyedStream<
            u64,
            bool,
            Self,
            Unbounded,
            TotalOrder,
            ExactlyOnce,
        > = KeyedStream::new(
            self.clone(),
            HydroNode::Source {
                source: HydroSource::Stream(membership_stream_expr.into()),
                metadata: self.new_node_metadata(KeyedStream::<
                    u64,
                    bool,
                    Self,
                    Unbounded,
                    TotalOrder,
                    ExactlyOnce,
                >::collection_kind()),
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
                .into_keyed(),
            raw_membership_stream.map(q!(|join| {
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
        ForwardHandle<'a, KeyedStream<u64, OutT, Self, Unbounded, NoOrder, ExactlyOnce>>,
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

        let out_t_type = quote_type::<OutT>();
        let ser_fn: syn::Expr = syn::parse_quote! {
            ::#root::runtime_support::stageleft::runtime_support::fn1_type_hint::<(u64, #out_t_type), _>(
                |(id, b)| (id, #root::runtime_support::bincode::serialize(&b).unwrap().into())
            )
        };

        flow_state_borrow.push_root(HydroRoot::SendExternal {
            to_external_id: from.id,
            to_key: next_external_port_id,
            to_many: true,
            serialize_fn: Some(ser_fn.into()),
            instantiate_fn: DebugInstantiate::Building,
            input: Box::new(to_sink.entries().ir_node.into_inner()),
            op_metadata: HydroIrOpMetadata::new(),
        });

        let in_t_type = quote_type::<InT>();

        let deser_fn: syn::Expr = syn::parse_quote! {
            |res| {
                let (id, b) = res.unwrap();
                (id, #root::runtime_support::bincode::deserialize::<#in_t_type>(&b).unwrap())
            }
        };

        let raw_stream: KeyedStream<u64, InT, Self, Unbounded, TotalOrder, ExactlyOnce> =
            KeyedStream::new(
                self.clone(),
                HydroNode::ExternalInput {
                    from_external_id: from.id,
                    from_key: next_external_port_id,
                    from_many: true,
                    codec_type: quote_type::<LengthDelimitedCodec>().into(),
                    port_hint: NetworkHint::Auto,
                    instantiate_fn: DebugInstantiate::Building,
                    deserialize_fn: Some(deser_fn.into()),
                    metadata: self.new_node_metadata(KeyedStream::<
                        u64,
                        InT,
                        Self,
                        Unbounded,
                        TotalOrder,
                        ExactlyOnce,
                    >::collection_kind()),
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
        let raw_membership_stream: KeyedStream<
            u64,
            bool,
            Self,
            Unbounded,
            TotalOrder,
            ExactlyOnce,
        > = KeyedStream::new(
            self.clone(),
            HydroNode::Source {
                source: HydroSource::Stream(membership_stream_expr.into()),
                metadata: self.new_node_metadata(KeyedStream::<
                    u64,
                    bool,
                    Self,
                    Unbounded,
                    TotalOrder,
                    ExactlyOnce,
                >::collection_kind()),
            },
        );

        (
            ExternalBincodeBidi {
                process_id: from.id,
                port_id: next_external_port_id,
                _phantom: PhantomData,
            },
            raw_stream,
            raw_membership_stream.map(q!(|join| {
                if join {
                    MembershipEvent::Joined
                } else {
                    MembershipEvent::Left
                }
            })),
            fwd_ref,
        )
    }

    /// Constructs a [`Singleton`] materialized at this location with the given static value.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let singleton = tick.singleton(q!(5));
    /// # singleton.all_ticks()
    /// # }, |mut stream| async move {
    /// // 5
    /// # assert_eq!(stream.next().await.unwrap(), 5);
    /// # }));
    /// ```
    fn singleton<T>(&self, e: impl QuotedWithContext<'a, T, Self>) -> Singleton<T, Self, Unbounded>
    where
        T: Clone,
        Self: Sized,
    {
        // TODO(shadaj): we mark this as unbounded because we do not yet have a representation
        // for bounded top-level singletons, and this is the only way to generate one

        let e = e.splice_untyped_ctx(self);

        Singleton::new(
            self.clone(),
            HydroNode::SingletonSource {
                value: e.into(),
                metadata: self
                    .new_node_metadata(Singleton::<T, Self, Unbounded>::collection_kind()),
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

    fn forward_ref<S>(&self) -> (ForwardHandle<'a, S>, S)
    where
        S: CycleCollection<'a, ForwardRef, Location = Self>,
        Self: NoTick,
    {
        let next_id = self.flow_state().borrow_mut().next_cycle_id();
        let ident = syn::Ident::new(&format!("cycle_{}", next_id), Span::call_site());

        (
            ForwardHandle {
                completed: false,
                ident: ident.clone(),
                expected_location: Location::id(self),
                _phantom: PhantomData,
            },
            S::create_source(ident, self.clone()),
        )
    }
}

#[cfg(feature = "deploy")]
#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use futures::{SinkExt, StreamExt};
    use hydro_deploy::Deployment;
    use stageleft::q;
    use tokio_util::codec::LengthDelimitedCodec;

    use crate::compile::builder::FlowBuilder;
    use crate::live_collections::stream::{ExactlyOnce, TotalOrder};
    use crate::location::{Location, NetworkHint};
    use crate::nondet::nondet;

    #[tokio::test]
    async fn top_level_singleton_replay_cardinality() {
        let mut deployment = Deployment::new();

        let flow = FlowBuilder::new();
        let node = flow.process::<()>();
        let external = flow.external::<()>();

        let (in_port, input) =
            node.source_external_bincode::<_, _, TotalOrder, ExactlyOnce>(&external);
        let singleton = node.singleton(q!(123));
        let tick = node.tick();
        let out = input
            .batch(&tick, nondet!(/** test */))
            .cross_singleton(singleton.clone().snapshot(&tick, nondet!(/** test */)))
            .cross_singleton(
                singleton
                    .snapshot(&tick, nondet!(/** test */))
                    .into_stream()
                    .count(),
            )
            .all_ticks()
            .send_bincode_external(&external);

        let nodes = flow
            .with_process(&node, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let mut external_in = nodes.connect(in_port).await;
        let mut external_out = nodes.connect(out).await;

        deployment.start().await.unwrap();

        external_in.send(1).await.unwrap();
        assert_eq!(external_out.next().await.unwrap(), ((1, 123), 1));

        external_in.send(2).await.unwrap();
        assert_eq!(external_out.next().await.unwrap(), ((2, 123), 1));
    }

    #[tokio::test]
    async fn tick_singleton_replay_cardinality() {
        let mut deployment = Deployment::new();

        let flow = FlowBuilder::new();
        let node = flow.process::<()>();
        let external = flow.external::<()>();

        let (in_port, input) =
            node.source_external_bincode::<_, _, TotalOrder, ExactlyOnce>(&external);
        let tick = node.tick();
        let singleton = tick.singleton(q!(123));
        let out = input
            .batch(&tick, nondet!(/** test */))
            .cross_singleton(singleton.clone())
            .cross_singleton(singleton.into_stream().count())
            .all_ticks()
            .send_bincode_external(&external);

        let nodes = flow
            .with_process(&node, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let mut external_in = nodes.connect(in_port).await;
        let mut external_out = nodes.connect(out).await;

        deployment.start().await.unwrap();

        external_in.send(1).await.unwrap();
        assert_eq!(external_out.next().await.unwrap(), ((1, 123), 1));

        external_in.send(2).await.unwrap();
        assert_eq!(external_out.next().await.unwrap(), ((2, 123), 1));
    }

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

        let mut external_in = nodes.connect(in_port).await.1;
        let mut external_out = nodes.connect(out).await;

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
        let external_out = nodes.connect(out).await;

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
        let mut external_out = nodes.connect(out).await;

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

        let mut external_in_1 = nodes.connect(in_port.clone()).await.1;
        let mut external_in_2 = nodes.connect(in_port).await.1;
        let external_out = nodes.connect(out).await;

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

        let (mut external_out_1, mut external_in_1) = nodes.connect(port.clone()).await;
        let (mut external_out_2, mut external_in_2) = nodes.connect(port).await;

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
