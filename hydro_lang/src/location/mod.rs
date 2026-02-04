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
//! See [the Hydro docs](https://hydro.run/docs/hydro/reference/locations/) for more information.

use std::fmt::Debug;
use std::marker::PhantomData;
use std::num::ParseIntError;
use std::time::Duration;

use bytes::{Bytes, BytesMut};
use futures::stream::Stream as FuturesStream;
use proc_macro2::Span;
use quote::quote;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use slotmap::{Key, new_key_type};
use stageleft::runtime_support::{FreeVariableWithContextWithProps, QuoteTokens};
use stageleft::{QuotedWithContext, q, quote_type};
use syn::parse_quote;
use tokio_util::codec::{Decoder, Encoder, LengthDelimitedCodec};

use crate::compile::ir::{DebugInstantiate, HydroIrOpMetadata, HydroNode, HydroRoot, HydroSource};
use crate::forward_handle::ForwardRef;
#[cfg(stageleft_runtime)]
use crate::forward_handle::{CycleCollection, ForwardHandle};
use crate::live_collections::boundedness::{Bounded, Unbounded};
use crate::live_collections::keyed_stream::KeyedStream;
use crate::live_collections::singleton::Singleton;
use crate::live_collections::stream::{
    ExactlyOnce, NoOrder, Ordering, Retries, Stream, TotalOrder,
};
use crate::location::dynamic::LocationId;
use crate::location::external_process::{
    ExternalBincodeBidi, ExternalBincodeSink, ExternalBytesPort, Many, NotMany,
};
use crate::nondet::NonDet;
#[cfg(feature = "sim")]
use crate::sim::SimSender;
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
pub use member_id::{MemberId, TaglessMemberId};

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

#[stageleft::export(LocationKey)]
new_key_type! {
    /// A unique identifier for a clock tick.
    pub struct LocationKey;
}

impl std::fmt::Display for LocationKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "loc{:?}", self.data()) // `"loc1v1"``
    }
}

/// This is used for the ECS membership stream.
/// TODO(mingwei): Make this more robust?
impl std::str::FromStr for LocationKey {
    type Err = Option<ParseIntError>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let nvn = s.strip_prefix("loc").ok_or(None)?;
        let (idx, ver) = nvn.split_once("v").ok_or(None)?;
        let idx: u64 = idx.parse()?;
        let ver: u64 = ver.parse()?;
        Ok(slotmap::KeyData::from_ffi((ver << 32) | idx).into())
    }
}

impl LocationKey {
    /// TODO(minwgei): Remove this and avoid magic key for simulator external.
    /// The first location key, used by the simulator as the default external location.
    pub const FIRST: Self = Self(slotmap::KeyData::from_ffi(0x0000000100000001)); // `1v1`

    /// A key for testing with index 1.
    #[cfg(test)]
    pub const TEST_KEY_1: Self = Self(slotmap::KeyData::from_ffi(0x000000ff00000001)); // `1v255`

    /// A key for testing with index 2.
    #[cfg(test)]
    pub const TEST_KEY_2: Self = Self(slotmap::KeyData::from_ffi(0x000000ff00000002)); // `2v255`
}

/// This is used within `q!` code in docker and ECS.
impl<Ctx> FreeVariableWithContextWithProps<Ctx, ()> for LocationKey {
    type O = LocationKey;

    fn to_tokens(self, _ctx: &Ctx) -> (QuoteTokens, ())
    where
        Self: Sized,
    {
        let root = get_this_crate();
        let n = Key::data(&self).as_ffi();
        (
            QuoteTokens {
                prelude: None,
                expr: Some(quote! {
                    #root::location::LocationKey::from(#root::runtime_support::slotmap::KeyData::from_ffi(#n))
                }),
            },
            (),
        )
    }
}

/// A simple enum for the type of a root location.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize)]
pub enum LocationType {
    /// A process (single node).
    Process,
    /// A cluster (multiple nodes).
    Cluster,
    /// An external client.
    External,
}

/// A location where data can be materialized and computation can be executed.
///
/// Hydro is a **global**, **distributed** programming model. This means that the data
/// and computation in a Hydro program can be spread across multiple machines, data
/// centers, and even continents. To achieve this, Hydro uses the concept of
/// **locations** to keep track of _where_ data is located and computation is executed.
///
/// Each live collection type (in [`crate::live_collections`]) has a type parameter `L`
/// which will always be a type that implements the [`Location`] trait (e.g. [`Process`]
/// and [`Cluster`]). To create distributed programs, Hydro provides a variety of APIs
/// to allow live collections to be _moved_ between locations via network send/receive.
///
/// See [the Hydro docs](https://hydro.run/docs/hydro/reference/locations/) for more information.
#[expect(
    private_bounds,
    reason = "only internal Hydro code can define location types"
)]
pub trait Location<'a>: dynamic::DynLocation {
    /// The root location type for this location.
    ///
    /// For top-level locations like [`Process`] and [`Cluster`], this is `Self`.
    /// For nested locations like [`Tick`], this is the root location that contains it.
    type Root: Location<'a>;

    /// Returns the root location for this location.
    ///
    /// For top-level locations like [`Process`] and [`Cluster`], this returns `self`.
    /// For nested locations like [`Tick`], this returns the root location that contains it.
    fn root(&self) -> Self::Root;

    /// Attempts to create a new [`Tick`] clock domain at this location.
    ///
    /// Returns `Some(Tick)` if this is a top-level location (like [`Process`] or [`Cluster`]),
    /// or `None` if this location is already inside a tick (nested ticks are not supported).
    ///
    /// Prefer using [`Location::tick`] when you know the location is top-level.
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

    /// Returns the unique identifier for this location.
    fn id(&self) -> LocationId {
        dynamic::DynLocation::id(self)
    }

    /// Creates a new [`Tick`] clock domain at this location.
    ///
    /// A tick represents a logical clock that can be used to batch streaming data
    /// into discrete time steps. This is useful for implementing iterative algorithms
    /// or for synchronizing data across multiple streams.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let inside_tick = process
    ///     .source_iter(q!(vec![1, 2, 3, 4]))
    ///     .batch(&tick, nondet!(/** test */));
    /// inside_tick.all_ticks()
    /// # }, |mut stream| async move {
    /// // 1, 2, 3, 4
    /// # for w in vec![1, 2, 3, 4] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// # }
    /// ```
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

    /// Creates an unbounded stream that continuously emits unit values `()`.
    ///
    /// This is useful for driving computations that need to run continuously,
    /// such as polling or heartbeat mechanisms.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// process.spin()
    ///     .batch(&tick, nondet!(/** test */))
    ///     .map(q!(|_| 42))
    ///     .all_ticks()
    /// # }, |mut stream| async move {
    /// // 42, 42, 42, ...
    /// # assert_eq!(stream.next().await.unwrap(), 42);
    /// # assert_eq!(stream.next().await.unwrap(), 42);
    /// # assert_eq!(stream.next().await.unwrap(), 42);
    /// # }));
    /// # }
    /// ```
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

    /// Creates a stream from an async [`FuturesStream`].
    ///
    /// This is useful for integrating with external async data sources,
    /// such as network connections or file readers.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// process.source_stream(q!(futures::stream::iter(vec![1, 2, 3])))
    /// # }, |mut stream| async move {
    /// // 1, 2, 3
    /// # for w in vec![1, 2, 3] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// # }
    /// ```
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

    /// Creates a bounded stream from an iterator.
    ///
    /// The iterator is evaluated once at runtime, and all elements are emitted
    /// in order. This is useful for creating streams from static data or
    /// for testing.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// process.source_iter(q!(vec![1, 2, 3, 4]))
    /// # }, |mut stream| async move {
    /// // 1, 2, 3, 4
    /// # for w in vec![1, 2, 3, 4] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// # }
    /// ```
    fn source_iter<T, E>(
        &self,
        e: impl QuotedWithContext<'a, E, Self>,
    ) -> Stream<T, Self, Bounded, TotalOrder, ExactlyOnce>
    where
        E: IntoIterator<Item = T>,
        Self: Sized + NoTick,
    {
        let e = e.splice_typed_ctx(self);

        Stream::new(
            self.clone(),
            HydroNode::Source {
                source: HydroSource::Iter(e.into()),
                metadata: self.new_node_metadata(
                    Stream::<T, Self, Bounded, TotalOrder, ExactlyOnce>::collection_kind(),
                ),
            },
        )
    }

    /// Creates a stream of membership events for a cluster.
    ///
    /// This stream emits [`MembershipEvent::Joined`] when a cluster member joins
    /// and [`MembershipEvent::Left`] when a cluster member leaves. The stream is
    /// keyed by the [`MemberId`] of the cluster member.
    ///
    /// This is useful for implementing protocols that need to track cluster membership,
    /// such as broadcasting to all members or detecting failures.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::multi_location_test(|flow, p2| {
    /// let p1 = flow.process::<()>();
    /// let workers: Cluster<()> = flow.cluster::<()>();
    /// # // do nothing on each worker
    /// # workers.source_iter(q!(vec![])).for_each(q!(|_: ()| {}));
    /// let cluster_members = p1.source_cluster_members(&workers);
    /// # cluster_members.entries().send(&p2, TCP.bincode())
    /// // if there are 4 members in the cluster, we would see a join event for each
    /// // { MemberId::<Worker>(0): [MembershipEvent::Join], MemberId::<Worker>(2): [MembershipEvent::Join], ... }
    /// # }, |mut stream| async move {
    /// # let mut results = Vec::new();
    /// # for w in 0..4 {
    /// #     results.push(format!("{:?}", stream.next().await.unwrap()));
    /// # }
    /// # results.sort();
    /// # assert_eq!(results, vec!["(MemberId::<()>(0), Joined)", "(MemberId::<()>(1), Joined)", "(MemberId::<()>(2), Joined)", "(MemberId::<()>(3), Joined)"]);
    /// # }));
    /// # }
    /// ```
    fn source_cluster_members<C: 'a>(
        &self,
        cluster: &Cluster<'a, C>,
    ) -> KeyedStream<MemberId<C>, MembershipEvent, Self, Unbounded>
    where
        Self: Sized + NoTick,
    {
        Stream::new(
            self.clone(),
            HydroNode::Source {
                source: HydroSource::ClusterMembers(cluster.id()),
                metadata: self.new_node_metadata(Stream::<
                    (TaglessMemberId, MembershipEvent),
                    Self,
                    Unbounded,
                    TotalOrder,
                    ExactlyOnce,
                >::collection_kind()),
            },
        )
        .map(q!(|(k, v)| (MemberId::from_tagless(k), v)))
        .into_keyed()
    }

    /// Creates a one-way connection from an external process to receive raw bytes.
    ///
    /// Returns a port handle for the external process to connect to, and a stream
    /// of received byte buffers.
    ///
    /// For bidirectional communication or typed data, see [`Location::bind_single_client`]
    /// or [`Location::source_external_bincode`].
    fn source_external_bytes<L>(
        &self,
        from: &External<L>,
    ) -> (
        ExternalBytesPort,
        Stream<BytesMut, Self, Unbounded, TotalOrder, ExactlyOnce>,
    )
    where
        Self: Sized + NoTick,
    {
        let (port, stream, sink) =
            self.bind_single_client::<_, Bytes, LengthDelimitedCodec>(from, NetworkHint::Auto);

        sink.complete(self.source_iter(q!([])));

        (port, stream)
    }

    /// Creates a one-way connection from an external process to receive bincode-serialized data.
    ///
    /// Returns a sink handle for the external process to send data to, and a stream
    /// of received values.
    ///
    /// For bidirectional communication, see [`Location::bind_single_client_bincode`].
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
        let (port, stream, sink) = self.bind_single_client_bincode::<_, T, ()>(from);
        sink.complete(self.source_iter(q!([])));

        (
            ExternalBincodeSink {
                process_key: from.key,
                port_id: port.port_id,
                _phantom: PhantomData,
            },
            stream.weaken_ordering().weaken_retries(),
        )
    }

    /// Sets up a simulated input port on this location for testing.
    ///
    /// Returns a handle to send messages to the location as well as a stream
    /// of received messages. This is only available when the `sim` feature is enabled.
    #[cfg(feature = "sim")]
    #[expect(clippy::type_complexity, reason = "stream markers")]
    fn sim_input<T, O: Ordering, R: Retries>(
        &self,
    ) -> (SimSender<T, O, R>, Stream<T, Self, Unbounded, O, R>)
    where
        Self: Sized + NoTick,
        T: Serialize + DeserializeOwned,
    {
        let external_location: External<'a, ()> = External {
            key: LocationKey::FIRST,
            flow_state: self.flow_state().clone(),
            _phantom: PhantomData,
        };

        let (external, stream) = self.source_external_bincode(&external_location);

        (SimSender(external.port_id, PhantomData), stream)
    }

    /// Establishes a server on this location to receive a bidirectional connection from a single
    /// client, identified by the given `External` handle. Returns a port handle for the external
    /// process to connect to, a stream of incoming messages, and a handle to send outgoing
    /// messages.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use hydro_deploy::Deployment;
    /// # use futures::{SinkExt, StreamExt};
    /// # tokio_test::block_on(async {
    /// # use bytes::Bytes;
    /// # use hydro_lang::location::NetworkHint;
    /// # use tokio_util::codec::LengthDelimitedCodec;
    /// # let mut flow = FlowBuilder::new();
    /// let node = flow.process::<()>();
    /// let external = flow.external::<()>();
    /// let (port, incoming, outgoing) =
    ///     node.bind_single_client::<_, Bytes, LengthDelimitedCodec>(&external, NetworkHint::Auto);
    /// outgoing.complete(incoming.map(q!(|data /* : Bytes */| {
    ///     let mut resp: Vec<u8> = data.into();
    ///     resp.push(42);
    ///     resp.into() // : Bytes
    /// })));
    ///
    /// # let mut deployment = Deployment::new();
    /// let nodes = flow // ... with_process and with_external
    /// #     .with_process(&node, deployment.Localhost())
    /// #     .with_external(&external, deployment.Localhost())
    /// #     .deploy(&mut deployment);
    ///
    /// deployment.deploy().await.unwrap();
    /// deployment.start().await.unwrap();
    ///
    /// let (mut external_out, mut external_in) = nodes.connect(port).await;
    /// external_in.send(vec![1, 2, 3].into()).await.unwrap();
    /// assert_eq!(
    ///     external_out.next().await.unwrap().unwrap(),
    ///     vec![1, 2, 3, 42]
    /// );
    /// # });
    /// # }
    /// ```
    #[expect(clippy::type_complexity, reason = "stream markers")]
    fn bind_single_client<L, T, Codec: Encoder<T> + Decoder>(
        &self,
        from: &External<L>,
        port_hint: NetworkHint,
    ) -> (
        ExternalBytesPort<NotMany>,
        Stream<<Codec as Decoder>::Item, Self, Unbounded, TotalOrder, ExactlyOnce>,
        ForwardHandle<'a, Stream<T, Self, Unbounded, TotalOrder, ExactlyOnce>>,
    )
    where
        Self: Sized + NoTick,
    {
        let next_external_port_id = from
            .flow_state
            .borrow_mut()
            .next_external_port
            .get_and_increment();

        let (fwd_ref, to_sink) =
            self.forward_ref::<Stream<T, Self, Unbounded, TotalOrder, ExactlyOnce>>();
        let mut flow_state_borrow = self.flow_state().borrow_mut();

        flow_state_borrow.push_root(HydroRoot::SendExternal {
            to_external_key: from.key,
            to_port_id: next_external_port_id,
            to_many: false,
            unpaired: false,
            serialize_fn: None,
            instantiate_fn: DebugInstantiate::Building,
            input: Box::new(to_sink.ir_node.into_inner()),
            op_metadata: HydroIrOpMetadata::new(),
        });

        let raw_stream: Stream<
            Result<<Codec as Decoder>::Item, <Codec as Decoder>::Error>,
            Self,
            Unbounded,
            TotalOrder,
            ExactlyOnce,
        > = Stream::new(
            self.clone(),
            HydroNode::ExternalInput {
                from_external_key: from.key,
                from_port_id: next_external_port_id,
                from_many: false,
                codec_type: quote_type::<Codec>().into(),
                port_hint,
                instantiate_fn: DebugInstantiate::Building,
                deserialize_fn: None,
                metadata: self.new_node_metadata(Stream::<
                    Result<<Codec as Decoder>::Item, <Codec as Decoder>::Error>,
                    Self,
                    Unbounded,
                    TotalOrder,
                    ExactlyOnce,
                >::collection_kind()),
            },
        );

        (
            ExternalBytesPort {
                process_key: from.key,
                port_id: next_external_port_id,
                _phantom: PhantomData,
            },
            raw_stream.flatten_ordered(),
            fwd_ref,
        )
    }

    /// Establishes a bidirectional connection from a single external client using bincode serialization.
    ///
    /// Returns a port handle for the external process to connect to, a stream of incoming messages,
    /// and a handle to send outgoing messages. This is a convenience wrapper around
    /// [`Location::bind_single_client`] that uses bincode for serialization.
    ///
    /// # Type Parameters
    /// - `InT`: The type of incoming messages (must implement [`DeserializeOwned`])
    /// - `OutT`: The type of outgoing messages (must implement [`Serialize`])
    #[expect(clippy::type_complexity, reason = "stream markers")]
    fn bind_single_client_bincode<L, InT: DeserializeOwned, OutT: Serialize>(
        &self,
        from: &External<L>,
    ) -> (
        ExternalBincodeBidi<InT, OutT, NotMany>,
        Stream<InT, Self, Unbounded, TotalOrder, ExactlyOnce>,
        ForwardHandle<'a, Stream<OutT, Self, Unbounded, TotalOrder, ExactlyOnce>>,
    )
    where
        Self: Sized + NoTick,
    {
        let next_external_port_id = from
            .flow_state
            .borrow_mut()
            .next_external_port
            .get_and_increment();

        let (fwd_ref, to_sink) =
            self.forward_ref::<Stream<OutT, Self, Unbounded, TotalOrder, ExactlyOnce>>();
        let mut flow_state_borrow = self.flow_state().borrow_mut();

        let root = get_this_crate();

        let out_t_type = quote_type::<OutT>();
        let ser_fn: syn::Expr = syn::parse_quote! {
            #root::runtime_support::stageleft::runtime_support::fn1_type_hint::<#out_t_type, _>(
                |b| #root::runtime_support::bincode::serialize(&b).unwrap().into()
            )
        };

        flow_state_borrow.push_root(HydroRoot::SendExternal {
            to_external_key: from.key,
            to_port_id: next_external_port_id,
            to_many: false,
            unpaired: false,
            serialize_fn: Some(ser_fn.into()),
            instantiate_fn: DebugInstantiate::Building,
            input: Box::new(to_sink.ir_node.into_inner()),
            op_metadata: HydroIrOpMetadata::new(),
        });

        let in_t_type = quote_type::<InT>();

        let deser_fn: syn::Expr = syn::parse_quote! {
            |res| {
                let b = res.unwrap();
                #root::runtime_support::bincode::deserialize::<#in_t_type>(&b).unwrap()
            }
        };

        let raw_stream: Stream<InT, Self, Unbounded, TotalOrder, ExactlyOnce> = Stream::new(
            self.clone(),
            HydroNode::ExternalInput {
                from_external_key: from.key,
                from_port_id: next_external_port_id,
                from_many: false,
                codec_type: quote_type::<LengthDelimitedCodec>().into(),
                port_hint: NetworkHint::Auto,
                instantiate_fn: DebugInstantiate::Building,
                deserialize_fn: Some(deser_fn.into()),
                metadata: self.new_node_metadata(Stream::<
                    InT,
                    Self,
                    Unbounded,
                    TotalOrder,
                    ExactlyOnce,
                >::collection_kind()),
            },
        );

        (
            ExternalBincodeBidi {
                process_key: from.key,
                port_id: next_external_port_id,
                _phantom: PhantomData,
            },
            raw_stream,
            fwd_ref,
        )
    }

    /// Establishes a server on this location to receive bidirectional connections from multiple
    /// external clients using raw bytes.
    ///
    /// Unlike [`Location::bind_single_client`], this method supports multiple concurrent client
    /// connections. Each client is assigned a unique `u64` identifier.
    ///
    /// Returns:
    /// - A port handle for external processes to connect to
    /// - A keyed stream of incoming messages, keyed by client ID
    /// - A keyed stream of membership events (client joins/leaves), keyed by client ID
    /// - A handle to send outgoing messages, keyed by client ID
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
        let next_external_port_id = from
            .flow_state
            .borrow_mut()
            .next_external_port
            .get_and_increment();

        let (fwd_ref, to_sink) =
            self.forward_ref::<KeyedStream<u64, T, Self, Unbounded, NoOrder, ExactlyOnce>>();
        let mut flow_state_borrow = self.flow_state().borrow_mut();

        flow_state_borrow.push_root(HydroRoot::SendExternal {
            to_external_key: from.key,
            to_port_id: next_external_port_id,
            to_many: true,
            unpaired: false,
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
                from_external_key: from.key,
                from_port_id: next_external_port_id,
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
                from.key, next_external_port_id
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
                process_key: from.key,
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

    /// Establishes a server on this location to receive bidirectional connections from multiple
    /// external clients using bincode serialization.
    ///
    /// Unlike [`Location::bind_single_client_bincode`], this method supports multiple concurrent
    /// client connections. Each client is assigned a unique `u64` identifier.
    ///
    /// Returns:
    /// - A port handle for external processes to connect to
    /// - A keyed stream of incoming messages, keyed by client ID
    /// - A keyed stream of membership events (client joins/leaves), keyed by client ID
    /// - A handle to send outgoing messages, keyed by client ID
    ///
    /// # Type Parameters
    /// - `InT`: The type of incoming messages (must implement [`DeserializeOwned`])
    /// - `OutT`: The type of outgoing messages (must implement [`Serialize`])
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
        let next_external_port_id = from
            .flow_state
            .borrow_mut()
            .next_external_port
            .get_and_increment();

        let (fwd_ref, to_sink) =
            self.forward_ref::<KeyedStream<u64, OutT, Self, Unbounded, NoOrder, ExactlyOnce>>();
        let mut flow_state_borrow = self.flow_state().borrow_mut();

        let root = get_this_crate();

        let out_t_type = quote_type::<OutT>();
        let ser_fn: syn::Expr = syn::parse_quote! {
            #root::runtime_support::stageleft::runtime_support::fn1_type_hint::<(u64, #out_t_type), _>(
                |(id, b)| (id, #root::runtime_support::bincode::serialize(&b).unwrap().into())
            )
        };

        flow_state_borrow.push_root(HydroRoot::SendExternal {
            to_external_key: from.key,
            to_port_id: next_external_port_id,
            to_many: true,
            unpaired: false,
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
                    from_external_key: from.key,
                    from_port_id: next_external_port_id,
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
                from.key, next_external_port_id
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
                process_key: from.key,
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
    /// # #[cfg(feature = "deploy")] {
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
    /// # }
    /// ```
    fn singleton<T>(&self, e: impl QuotedWithContext<'a, T, Self>) -> Singleton<T, Self, Bounded>
    where
        T: Clone,
        Self: Sized,
    {
        let e = e.splice_untyped_ctx(self);

        Singleton::new(
            self.clone(),
            HydroNode::SingletonSource {
                value: e.into(),
                metadata: self.new_node_metadata(Singleton::<T, Self, Bounded>::collection_kind()),
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

    /// Creates a forward reference for defining recursive or mutually-dependent dataflows.
    ///
    /// Returns a handle that must be completed with the actual stream, and a placeholder
    /// stream that can be used in the dataflow graph before the actual stream is defined.
    ///
    /// This is useful for implementing feedback loops or recursive computations where
    /// a stream depends on its own output.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use hydro_lang::live_collections::stream::NoOrder;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// // Create a forward reference for the feedback stream
    /// let (complete, feedback) = process.forward_ref::<Stream<i32, _, _, NoOrder>>();
    ///
    /// // Combine initial input with feedback, then increment
    /// let input: Stream<_, _, Unbounded> = process.source_iter(q!([1])).into();
    /// let output: Stream<_, _, _, NoOrder> = input
    ///     .interleave(feedback)
    ///     .map(q!(|x| x + 1));
    ///
    /// // Complete the forward reference with the output
    /// complete.complete(output.clone());
    /// output
    /// # }, |mut stream| async move {
    /// // 2, 3, 4, 5, ...
    /// # assert_eq!(stream.next().await.unwrap(), 2);
    /// # assert_eq!(stream.next().await.unwrap(), 3);
    /// # assert_eq!(stream.next().await.unwrap(), 4);
    /// # }));
    /// # }
    /// ```
    fn forward_ref<S>(&self) -> (ForwardHandle<'a, S>, S)
    where
        S: CycleCollection<'a, ForwardRef, Location = Self>,
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

        let mut flow = FlowBuilder::new();
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

        let mut flow = FlowBuilder::new();
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

        let mut flow = FlowBuilder::new();
        let first_node = flow.process::<()>();
        let external = flow.external::<()>();

        let (in_port, input) = first_node.source_external_bytes(&external);
        let out = input.send_bincode_external(&external);

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

        let mut flow = FlowBuilder::new();
        let first_node = flow.process::<()>();
        let external = flow.external::<()>();

        let (in_port, input, _membership, complete_sink) =
            first_node.bidi_external_many_bincode(&external);
        let out = input.entries().send_bincode_external(&external);
        complete_sink.complete(
            first_node
                .source_iter::<(u64, ()), _>(q!([]))
                .into_keyed()
                .weaken_ordering(),
        );

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

        let mut flow = FlowBuilder::new();
        let first_node = flow.process::<()>();
        let external = flow.external::<()>();

        let (in_port, input, _membership, complete_sink) =
            first_node.bidi_external_many_bincode(&external);
        let out = input.entries().send_bincode_external(&external);
        complete_sink.complete(
            first_node
                .source_iter::<(u64, ()), _>(q!([]))
                .into_keyed()
                .weaken_ordering(),
        );

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

        let mut flow = FlowBuilder::new();
        let first_node = flow.process::<()>();
        let external = flow.external::<()>();

        let (in_port, input, _membership, complete_sink) = first_node
            .bidi_external_many_bytes::<_, _, LengthDelimitedCodec>(&external, NetworkHint::Auto);
        let out = input.entries().send_bincode_external(&external);
        complete_sink.complete(
            first_node
                .source_iter(q!([]))
                .into_keyed()
                .weaken_ordering(),
        );

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
    async fn single_client_external_bytes() {
        let mut deployment = Deployment::new();
        let mut flow = FlowBuilder::new();
        let first_node = flow.process::<()>();
        let external = flow.external::<()>();
        let (port, input, complete_sink) = first_node
            .bind_single_client::<_, _, LengthDelimitedCodec>(&external, NetworkHint::Auto);
        complete_sink.complete(input.map(q!(|data| {
            let mut resp: Vec<u8> = data.into();
            resp.push(42);
            resp.into() // : Bytes
        })));

        let nodes = flow
            .with_process(&first_node, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();
        deployment.start().await.unwrap();

        let (mut external_out, mut external_in) = nodes.connect(port).await;

        external_in.send(vec![1, 2, 3].into()).await.unwrap();
        assert_eq!(
            external_out.next().await.unwrap().unwrap(),
            vec![1, 2, 3, 42]
        );
    }

    #[tokio::test]
    async fn echo_external_bytes() {
        let mut deployment = Deployment::new();

        let mut flow = FlowBuilder::new();
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

        let mut flow = FlowBuilder::new();
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

        external_in_1.send("hi".to_owned()).await.unwrap();
        external_in_2.send("hello".to_owned()).await.unwrap();

        assert_eq!(external_out_1.next().await.unwrap(), "HI");
        assert_eq!(external_out_2.next().await.unwrap(), "HELLO");
    }
}
