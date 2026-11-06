//! Types for configuring network channels with serialization formats, transports, etc.

use std::marker::PhantomData;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::live_collections::stream::networking::{deserialize_bincode, serialize_bincode};
use crate::live_collections::stream::{
    AtLeastOnce, Drops, ExactlyOnce, MayDrop, NoDrop, NoOrder, Ordering, Retries, TotalOrder,
};
use crate::location::cluster::{Consistency, EventualConsistency, NoConsistency};

#[sealed::sealed]
trait SerKind<T: ?Sized> {
    fn serialize_thunk(is_demux: bool) -> syn::Expr;

    fn deserialize_thunk(tagged: Option<&syn::Type>) -> syn::Expr;

    /// Whether this serialization backend leaves serialization to code outside of Hydro (see
    /// [`Embedded`]). When `true`, [`Self::serialize_thunk`] and [`Self::deserialize_thunk`] are
    /// never called; the raw element type flows across the channel unserialized.
    fn is_embedded() -> bool {
        false
    }
}

/// Serialize items using the [`bincode`] crate.
pub enum Bincode {}

#[sealed::sealed]
impl<T: Serialize + DeserializeOwned> SerKind<T> for Bincode {
    fn serialize_thunk(is_demux: bool) -> syn::Expr {
        serialize_bincode::<T>(is_demux)
    }

    fn deserialize_thunk(tagged: Option<&syn::Type>) -> syn::Expr {
        deserialize_bincode::<T>(tagged)
    }
}

/// Leaves serialization of items to code outside of Hydro. Every [`EMBEDDED`] channel uses it.
///
/// This serialization backend is only supported by the embedded deployment backend (it will panic
/// on all other backends). The generated network channel exposes the raw element type `T` to the
/// developer (rather than serialized bytes), so they can perform custom serialization logic outside
/// of the Hydro program for that channel.
pub enum Embedded {}

#[sealed::sealed]
impl<T> SerKind<T> for Embedded {
    fn serialize_thunk(_is_demux: bool) -> syn::Expr {
        unreachable!("embedded serialization does not use a serialize thunk")
    }

    fn deserialize_thunk(_tagged: Option<&syn::Type>) -> syn::Expr {
        unreachable!("embedded serialization does not use a deserialize thunk")
    }

    fn is_embedded() -> bool {
        true
    }
}

/// An unconfigured serialization backend.
pub enum NoSer {}

/// A transport backend for network channels.
#[sealed::sealed]
pub trait TransportKind {
    /// The ordering guarantee provided by this transport.
    type OrderingGuarantee: Ordering;

    /// The retry guarantee provided by this transport: [`ExactlyOnce`] if it never delivers a
    /// message more than once.
    type RetriesGuarantee: Retries;

    /// The drops guarantee provided by this transport: [`MayDrop`] if it may never deliver a
    /// message (see [`NetworkFor::DropsGuarantee`]).
    type DropsGuarantee: Drops;

    /// The consistency guarantee this transport can preserve for replicated outputs
    /// (see [`NetworkFor::ConsistencyGuarantee`]).
    type ConsistencyGuarantee: Consistency;

    /// Returns the [`NetworkingInfo`] describing this transport's configuration.
    fn networking_info() -> NetworkingInfo;
}

#[sealed::sealed]
#[diagnostic::on_unimplemented(
    message = "the TCP transport requires a failure policy",
    note = "for example, `TCP.fail_stop()` stops delivering messages after a failed connection"
)]
/// A failure policy for TCP connections, determining how the transport handles
/// connection failures and what ordering guarantees the output stream provides.
pub trait TcpFailPolicy {
    /// The ordering guarantee provided by this failure policy.
    type OrderingGuarantee: Ordering;

    /// The drops guarantee of the received stream under this failure policy: [`NoDrop`] if
    /// it loses only a suffix of a recipient's messages, [`MayDrop`] otherwise.
    type Drops: Drops;

    /// The consistency guarantee this failure policy can preserve for replicated outputs
    /// (see [`NetworkFor::ConsistencyGuarantee`]).
    type ConsistencyGuarantee: Consistency;

    /// Returns the [`TcpFault`] variant for this failure policy.
    fn tcp_fault() -> TcpFault;
}

/// A TCP failure policy that stops sending messages after a failed connection.
pub enum FailStop {}
#[sealed::sealed]
impl TcpFailPolicy for FailStop {
    type OrderingGuarantee = TotalOrder;

    // A failed connection loses only a suffix of a recipient's messages, as if the recipient
    // had crashed, so the received stream is still a prefix of the ideal stream.
    type Drops = NoDrop;

    // A failed connection stops *all* future deliveries to that recipient, which models the
    // recipient as having failed. Consistency guarantees only apply to live members, so
    // eventual consistency of replicated outputs is preserved.
    type ConsistencyGuarantee = EventualConsistency;

    fn tcp_fault() -> TcpFault {
        TcpFault::FailStop
    }
}

/// A failure policy that allows any message to be lost, for one recipient and not another, while
/// later ones are delivered. The received stream is a [`MayDrop`] subsequence of the messages
/// sent, whose consumers observe the drops with
/// [`observe_drops`](crate::live_collections::stream::Stream::observe_drops): with a `nondet!`
/// and a `nonlive!` if it is ordered, or only a `nonlive!` if it is not.
pub enum Lossy {}
#[sealed::sealed]
impl TcpFailPolicy for Lossy {
    type OrderingGuarantee = TotalOrder;

    // Any message may be lost while later ones are delivered: the received stream is a
    // subsequence of the messages sent.
    type Drops = MayDrop;

    // A lossy channel can drop an arbitrary message for one recipient while continuing to
    // deliver later messages, so replicated outputs can diverge across members forever.
    type ConsistencyGuarantee = NoConsistency;

    fn tcp_fault() -> TcpFault {
        TcpFault::Lossy
    }
}

/// A failure policy that treats dropped messages as indefinitely delayed.
///
/// The received stream is [`NoOrder`] and [`MayDrop`]: without an order, a dropped message
/// cannot be told from one that is merely late, so its consumers observe the drops with only a
/// [`nonlive!`](crate::liveness::nonlive).
///
/// When those drops are observed in the Hydro simulator, you must call
/// [`.test_safety_only()`](crate::sim::flow::SimFlow::test_safety_only): the simulator does not
/// actually drop packets, but delays "dropped" messages until the end of the execution, which
/// catches safety bugs but cannot test liveness.
pub enum LossyDelayedForever {}
#[sealed::sealed]
impl TcpFailPolicy for LossyDelayedForever {
    type OrderingGuarantee = NoOrder;

    // A message that is never delivered is missing from the received stream (modelled as
    // delayed forever, which an unordered stream cannot tell apart from a lost message).
    type Drops = MayDrop;

    // Dropped messages are modeled as indefinitely delayed, so the output on each member is
    // always a lower bound of the ideal stream that is eventually delivered in full; replicated
    // outputs therefore remain eventually consistent.
    type ConsistencyGuarantee = EventualConsistency;

    fn tcp_fault() -> TcpFault {
        TcpFault::LossyDelayedForever
    }
}

#[sealed::sealed]
#[diagnostic::on_unimplemented(
    message = "the UDP transport requires a failure policy",
    note = "for example, `UDP.lossy_delayed_forever()` treats dropped messages as indefinitely delayed"
)]
/// A failure policy for UDP channels, determining how the transport handles
/// message loss. Because UDP provides no ordering guarantees, all policies
/// produce [`NoOrder`] output streams, and there is no `fail_stop` option
/// (UDP is connectionless, so there is no connection to fail).
pub trait UdpFailPolicy {
    /// The drops guarantee of the received stream under this failure policy: [`NoDrop`] if
    /// it loses only a suffix of a recipient's messages, [`MayDrop`] otherwise.
    type Drops: Drops;

    /// The consistency guarantee this failure policy can preserve for replicated outputs
    /// (see [`NetworkFor::ConsistencyGuarantee`]).
    type ConsistencyGuarantee: Consistency;

    /// Returns the [`UdpFault`] variant for this failure policy.
    fn udp_fault() -> UdpFault;
}

#[sealed::sealed]
impl UdpFailPolicy for Lossy {
    // Any message may be lost while later ones are delivered: the received stream is a
    // subsequence of the messages sent.
    type Drops = MayDrop;

    // A lossy channel can drop an arbitrary message for one recipient while continuing to
    // deliver later messages, so replicated outputs can diverge across members forever.
    type ConsistencyGuarantee = NoConsistency;

    fn udp_fault() -> UdpFault {
        UdpFault::Lossy
    }
}

#[sealed::sealed]
impl UdpFailPolicy for LossyDelayedForever {
    // A message that is never delivered is missing from the received stream (modelled as
    // delayed forever, which an unordered stream cannot tell apart from a lost message).
    type Drops = MayDrop;

    // Dropped messages are modeled as indefinitely delayed, so the output on each member is
    // always a lower bound of the ideal stream that is eventually delivered in full; replicated
    // outputs therefore remain eventually consistent.
    type ConsistencyGuarantee = EventualConsistency;

    fn udp_fault() -> UdpFault {
        UdpFault::LossyDelayedForever
    }
}

/// Send items across a length-delimited TCP channel.
pub struct Tcp<F> {
    _phantom: PhantomData<F>,
}

#[sealed::sealed]
impl<F: TcpFailPolicy> TransportKind for Tcp<F> {
    type OrderingGuarantee = F::OrderingGuarantee;

    type RetriesGuarantee = ExactlyOnce;

    type DropsGuarantee = F::Drops;

    type ConsistencyGuarantee = F::ConsistencyGuarantee;

    fn networking_info() -> NetworkingInfo {
        NetworkingInfo::Tcp {
            fault: F::tcp_fault(),
        }
    }
}

/// Items cross a channel that the host of an embedded program implements however it likes (a
/// network connection, an in-memory queue, ...), keeping the retries `R`, ordering `O`, and
/// failure policy `F` that follow from what the channel declares about each anomaly (`()` until
/// declared).
pub struct EmbeddedTransport<R, O, F> {
    _phantom: PhantomData<(R, O, F)>,
}

/// The retries an [`EMBEDDED`] channel declares: [`ExactlyOnce`]
/// ([`never_duplicates`](NetworkingConfig::never_duplicates)) or [`AtLeastOnce`]
/// ([`may_duplicate`](NetworkingConfig::may_duplicate)).
#[sealed::sealed]
#[diagnostic::on_unimplemented(
    message = "this `EMBEDDED` channel does not declare whether it may duplicate messages",
    note = "add `.never_duplicates()` or `.may_duplicate()` to the network config"
)]
pub trait EmbeddedRetries: Retries {}

#[sealed::sealed]
impl EmbeddedRetries for ExactlyOnce {}

#[sealed::sealed]
impl EmbeddedRetries for AtLeastOnce {}

/// The ordering an [`EMBEDDED`] channel declares: [`TotalOrder`]
/// ([`never_reorders`](NetworkingConfig::never_reorders)) or [`NoOrder`]
/// ([`may_reorder`](NetworkingConfig::may_reorder)).
#[sealed::sealed]
#[diagnostic::on_unimplemented(
    message = "this `EMBEDDED` channel does not declare whether it may reorder messages",
    note = "add `.never_reorders()` or `.may_reorder()` to the network config"
)]
pub trait EmbeddedOrdering: Ordering {}

#[sealed::sealed]
impl EmbeddedOrdering for TotalOrder {}

#[sealed::sealed]
impl EmbeddedOrdering for NoOrder {}

/// The failure policy an [`EMBEDDED`] channel declares: [`FailStop`]
/// ([`never_drops`](NetworkingConfig::never_drops)), [`Lossy`]
/// ([`may_drop`](NetworkingConfig::may_drop)), or [`LossyDelayedForever`]
/// ([`may_delay_forever`](NetworkingConfig::may_delay_forever)).
#[sealed::sealed]
#[diagnostic::on_unimplemented(
    message = "this `EMBEDDED` channel does not declare whether it may drop messages",
    note = "add `.never_drops()`, `.may_drop()`, or `.may_delay_forever()` to the network config"
)]
pub trait EmbeddedFailPolicy {
    /// The drops guarantee of the received stream under this failure policy: [`NoDrop`] if
    /// it loses only a suffix of a recipient's messages, [`MayDrop`] otherwise.
    type Drops: Drops;

    /// The ordering of the received stream when the channel declares ordering `O`.
    type Ordering<O: Ordering>: Ordering;

    /// The consistency guarantee this failure policy can preserve for replicated outputs
    /// (see [`NetworkFor::ConsistencyGuarantee`]).
    type ConsistencyGuarantee: Consistency;

    /// Returns the [`EmbeddedFault`] variant for this failure policy.
    fn embedded_fault() -> EmbeddedFault;
}

#[sealed::sealed]
impl EmbeddedFailPolicy for FailStop {
    // A failed connection loses only a suffix of a recipient's messages, as if the recipient
    // had crashed, so the received stream is still a prefix of the ideal stream.
    type Drops = NoDrop;

    type Ordering<O: Ordering> = O;

    type ConsistencyGuarantee = EventualConsistency;

    fn embedded_fault() -> EmbeddedFault {
        EmbeddedFault::FailStop
    }
}

#[sealed::sealed]
impl EmbeddedFailPolicy for Lossy {
    // Any message may be lost while later ones are delivered: the received stream is a
    // subsequence of the messages sent.
    type Drops = MayDrop;

    type Ordering<O: Ordering> = O;

    type ConsistencyGuarantee = NoConsistency;

    fn embedded_fault() -> EmbeddedFault {
        EmbeddedFault::Lossy
    }
}

#[sealed::sealed]
impl EmbeddedFailPolicy for LossyDelayedForever {
    // A message that is never delivered is missing from the received stream (modelled as
    // delayed forever, which an unordered stream cannot tell apart from a lost message).
    type Drops = MayDrop;

    type Ordering<O: Ordering> = NoOrder;

    type ConsistencyGuarantee = EventualConsistency;

    fn embedded_fault() -> EmbeddedFault {
        EmbeddedFault::LossyDelayedForever
    }
}

/// An [`EMBEDDED`] channel's guarantees are the ones its declarations leave, with the
/// consistency guarantee of its failure policy (as for [`Tcp`] and [`Udp`]).
#[sealed::sealed]
impl<R: EmbeddedRetries, O: EmbeddedOrdering, F: EmbeddedFailPolicy> TransportKind
    for EmbeddedTransport<R, O, F>
{
    type OrderingGuarantee = F::Ordering<O>;

    type RetriesGuarantee = R;

    type DropsGuarantee = F::Drops;

    type ConsistencyGuarantee = F::ConsistencyGuarantee;

    fn networking_info() -> NetworkingInfo {
        NetworkingInfo::Embedded {
            retries: R::RETRIES_KIND,
            ordering: O::ORDERING_KIND,
            fault: F::embedded_fault(),
        }
    }
}

/// Send items across a UDP channel, which does not guarantee delivery or ordering.
pub struct Udp<F> {
    _phantom: PhantomData<F>,
}

#[sealed::sealed]
impl<F: UdpFailPolicy> TransportKind for Udp<F> {
    type OrderingGuarantee = NoOrder;

    type RetriesGuarantee = ExactlyOnce;

    type DropsGuarantee = F::Drops;

    type ConsistencyGuarantee = F::ConsistencyGuarantee;

    fn networking_info() -> NetworkingInfo {
        NetworkingInfo::Udp {
            fault: F::udp_fault(),
        }
    }
}

/// A networking backend implementation that supports items of type `T`.
#[sealed::sealed]
pub trait NetworkFor<T: ?Sized> {
    /// The ordering guarantee provided by this network configuration.
    /// When combined with an input stream's ordering `O`, the output ordering
    /// will be `<O as MinOrder<Self::OrderingGuarantee>>::Min`.
    type OrderingGuarantee: Ordering;

    /// The retry guarantee provided by this network configuration.
    /// When combined with an input stream's retries `R`, the output retries
    /// will be `<R as MinRetries<Self::RetriesGuarantee>>::Min`.
    type RetriesGuarantee: Retries;

    /// The drops guarantee provided by this network configuration: [`MayDrop`] if it may never
    /// deliver a message (`lossy`, `lossy_delayed_forever`, `may_drop`, or
    /// `may_delay_forever`). When combined with an input stream's drops `D`, the output drops
    /// will be `<D as MinDrops<Self::DropsGuarantee>>::Min`. A `MayDrop` received stream is
    /// observed with [`observe_drops`](crate::live_collections::stream::Stream::observe_drops).
    type DropsGuarantee: Drops;

    /// The consistency guarantee this network configuration can preserve when the same data is
    /// replicated to several recipients (e.g. via
    /// [`Stream::broadcast_closed`](crate::live_collections::stream::Stream::broadcast_closed)).
    ///
    /// Failure policies that guarantee each recipient eventually observes the full stream of
    /// sent messages, or that model failures as the recipient stopping entirely (such as
    /// `fail_stop` or `lossy_delayed_forever`), preserve
    /// [`EventualConsistency`]. Plain `lossy`
    /// channels can silently drop individual messages for some recipients while others receive
    /// them, so replicated outputs can permanently diverge and only
    /// [`NoConsistency`] is guaranteed.
    type ConsistencyGuarantee: Consistency;

    /// Generates serialization logic for sending `T`.
    fn serialize_thunk(is_demux: bool) -> syn::Expr;

    /// Generates deserialization logic for receiving `T`.
    fn deserialize_thunk(tagged: Option<&syn::Type>) -> syn::Expr;

    /// Whether this network channel leaves serialization to code outside of Hydro (an
    /// [`EMBEDDED`] channel). When `true`, [`Self::serialize_thunk`] and
    /// [`Self::deserialize_thunk`] are never called; the raw element type flows across the
    /// channel unserialized.
    fn is_embedded() -> bool {
        false
    }

    /// Returns the optional name of the network channel.
    fn name(&self) -> Option<&str>;

    /// Returns the [`NetworkingInfo`] describing this network channel's transport and fault model.
    fn networking_info() -> NetworkingInfo;
}

/// The fault model for a TCP connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize)]
pub enum TcpFault {
    /// Stops sending messages after a failed connection.
    FailStop,
    /// Messages may be lost (e.g. due to network partitions).
    Lossy,
    /// Dropped messages are treated as indefinitely delayed with no ordering guarantee.
    LossyDelayedForever,
}

/// The failure policy of an [`EMBEDDED`] channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize)]
pub enum EmbeddedFault {
    /// Stops delivering a sender's messages once its channel fails.
    FailStop,
    /// Messages may be lost.
    Lossy,
    /// Undelivered messages are treated as indefinitely delayed.
    LossyDelayedForever,
}

/// The fault model for a UDP channel.
///
/// UDP is connectionless and never guarantees delivery, so there is no
/// `FailStop` variant — messages can always be dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize)]
pub enum UdpFault {
    /// Messages may be lost (e.g. due to network partitions or congestion).
    Lossy,
    /// Dropped messages are treated as indefinitely delayed.
    LossyDelayedForever,
}

/// Describes the networking configuration for a network channel at the IR level.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize)]
pub enum NetworkingInfo {
    /// A TCP-based network channel with a specific fault model.
    Tcp {
        /// The fault model for this TCP connection.
        fault: TcpFault,
    },
    /// A UDP-based network channel with a specific fault model.
    Udp {
        /// The fault model for this UDP channel.
        fault: UdpFault,
    },
    /// A channel implemented by the host of an embedded program ([`EMBEDDED`]), described by
    /// the guarantees it is declared with, which the host's channel must keep.
    Embedded {
        /// Whether the channel may deliver a message more than once (`AtLeastOnce`) or not
        /// (`ExactlyOnce`).
        retries: crate::compile::ir::StreamRetry,
        /// The order the channel must deliver each sender's messages in: `TotalOrder` (the order
        /// they were sent) or `NoOrder` (any).
        ordering: crate::compile::ir::StreamOrder,
        /// What the channel may leave undelivered.
        fault: EmbeddedFault,
    },
}

/// A network channel configuration with `T` as transport backend and `S` as the serialization
/// backend.
pub struct NetworkingConfig<Tr: ?Sized, S: ?Sized, Name = ()> {
    name: Option<Name>,
    _phantom: (PhantomData<Tr>, PhantomData<S>),
}

impl<Tr: ?Sized, S: ?Sized> NetworkingConfig<Tr, S> {
    /// Names the network channel and enables stable communication across multiple service versions.
    pub fn name(self, name: impl Into<String>) -> NetworkingConfig<Tr, S, String> {
        NetworkingConfig {
            name: Some(name.into()),
            _phantom: (PhantomData, PhantomData),
        }
    }
}

impl<Tr: ?Sized, N> NetworkingConfig<Tr, NoSer, N> {
    /// Configures the network channel to use [`bincode`] to serialize items.
    pub const fn bincode(mut self) -> NetworkingConfig<Tr, Bincode, N> {
        let taken_name = self.name.take();
        std::mem::forget(self); // nothing else is stored
        NetworkingConfig {
            name: taken_name,
            _phantom: (PhantomData, PhantomData),
        }
    }
}

impl<S: ?Sized> NetworkingConfig<Tcp<()>, S> {
    /// Configures the TCP transport to stop sending messages after a failed connection.
    ///
    /// Note that the Hydro simulator will not simulate connection failures that impact the
    /// *liveness* of a program. If an output assertion depends on a `fail_stop` channel
    /// making progress, that channel will not experience a failure that would cause the test to
    /// block indefinitely. However, any *safety* issues caused by connection failures will still
    /// be caught, such as a race condition between a failed connection and some other message.
    #[must_use]
    pub const fn fail_stop(self) -> NetworkingConfig<Tcp<FailStop>, S> {
        NetworkingConfig {
            name: self.name,
            _phantom: (PhantomData, PhantomData),
        }
    }

    /// Configures the TCP transport to allow messages to be lost.
    ///
    /// This is appropriate for networks where messages may be dropped, such as when
    /// running under a Maelstrom partition nemesis. Unlike `fail_stop`, which guarantees
    /// a prefix of messages is delivered, `lossy` makes no such guarantee: the received stream
    /// is an in-order [`MayDrop`] subsequence of the messages sent, whose consumers observe the
    /// drops with
    /// [`observe_drops(nondet!(..), nonlive!(..))`](crate::live_collections::stream::Stream::observe_drops),
    /// where the Hydro simulator explores them.
    #[must_use]
    pub const fn lossy(self) -> NetworkingConfig<Tcp<Lossy>, S> {
        NetworkingConfig {
            name: self.name,
            _phantom: (PhantomData, PhantomData),
        }
    }

    /// Configures the TCP transport to treat dropped messages as indefinitely delayed.
    ///
    /// This is appropriate for networks where messages may be dropped, such as when
    /// running under a Maelstrom partition nemesis. The received stream is [`NoOrder`] and
    /// [`MayDrop`]: without an order, a dropped message cannot be told from one that is
    /// merely late, so its consumers observe the drops with only a
    /// [`nonlive!`](crate::liveness::nonlive).
    ///
    /// When those drops are observed in the Hydro simulator, you must call
    /// [`.test_safety_only()`](crate::sim::flow::SimFlow::test_safety_only): the simulator
    /// does not actually drop packets, but delays "dropped" messages until the end of the
    /// execution, which catches safety bugs but cannot test liveness.
    #[must_use]
    pub const fn lossy_delayed_forever(self) -> NetworkingConfig<Tcp<LossyDelayedForever>, S> {
        NetworkingConfig {
            name: self.name,
            _phantom: (PhantomData, PhantomData),
        }
    }
}

impl<O, F, S: ?Sized> NetworkingConfig<EmbeddedTransport<(), O, F>, S> {
    /// Declares that the host's channel never delivers a message more than once, so the
    /// received stream keeps the sent stream's retries.
    #[must_use]
    pub const fn never_duplicates(
        self,
    ) -> NetworkingConfig<EmbeddedTransport<ExactlyOnce, O, F>, S> {
        NetworkingConfig {
            name: self.name,
            _phantom: (PhantomData, PhantomData),
        }
    }

    /// Declares that the host's channel may deliver a message more than once, so the received
    /// stream is [`AtLeastOnce`].
    #[must_use]
    pub const fn may_duplicate(self) -> NetworkingConfig<EmbeddedTransport<AtLeastOnce, O, F>, S> {
        NetworkingConfig {
            name: self.name,
            _phantom: (PhantomData, PhantomData),
        }
    }
}

impl<R, F, S: ?Sized> NetworkingConfig<EmbeddedTransport<R, (), F>, S> {
    /// Declares that the host's channel delivers each sender's messages in the order it sent
    /// them, so the received stream keeps the sent stream's order.
    #[must_use]
    pub const fn never_reorders(self) -> NetworkingConfig<EmbeddedTransport<R, TotalOrder, F>, S> {
        NetworkingConfig {
            name: self.name,
            _phantom: (PhantomData, PhantomData),
        }
    }

    /// Declares that the host's channel may deliver a sender's messages in a different order
    /// than it sent them, so the received stream is [`NoOrder`].
    #[must_use]
    pub const fn may_reorder(self) -> NetworkingConfig<EmbeddedTransport<R, NoOrder, F>, S> {
        NetworkingConfig {
            name: self.name,
            _phantom: (PhantomData, PhantomData),
        }
    }
}

impl<R, O, S: ?Sized> NetworkingConfig<EmbeddedTransport<R, O, ()>, S> {
    /// Declares that the host's channel never drops a message: while it stays live, it delivers
    /// every message sent. Like [`never_reorders`](NetworkingConfig::never_reorders) and
    /// [`never_duplicates`](NetworkingConfig::never_duplicates), this holds as long as the
    /// channel remains live; if it fails, it delivers nothing more, like
    /// [`TCP.fail_stop()`](NetworkingConfig::fail_stop).
    #[must_use]
    pub const fn never_drops(self) -> NetworkingConfig<EmbeddedTransport<R, O, FailStop>, S> {
        NetworkingConfig {
            name: self.name,
            _phantom: (PhantomData, PhantomData),
        }
    }

    /// Declares that the host's channel may lose messages and keep delivering later ones, like
    /// [`TCP.lossy()`](NetworkingConfig::lossy): the received stream is [`MayDrop`].
    #[must_use]
    pub const fn may_drop(self) -> NetworkingConfig<EmbeddedTransport<R, O, Lossy>, S> {
        NetworkingConfig {
            name: self.name,
            _phantom: (PhantomData, PhantomData),
        }
    }

    /// Declares that the host's channel may never deliver a message, which is treated as
    /// indefinitely delayed, like
    /// [`TCP.lossy_delayed_forever()`](NetworkingConfig::lossy_delayed_forever). The received
    /// stream is [`NoOrder`] and [`MayDrop`].
    #[must_use]
    pub const fn may_delay_forever(
        self,
    ) -> NetworkingConfig<EmbeddedTransport<R, O, LossyDelayedForever>, S> {
        NetworkingConfig {
            name: self.name,
            _phantom: (PhantomData, PhantomData),
        }
    }
}

impl<S: ?Sized> NetworkingConfig<Udp<()>, S> {
    /// Configures the UDP transport to allow messages to be lost.
    ///
    /// UDP never guarantees delivery or ordering, so unlike TCP there is no `fail_stop`
    /// policy — messages may always be dropped and the output stream always has
    /// [`NoOrder`] guarantees. The received stream is [`MayDrop`], observed with
    /// [`observe_drops(nonlive!(..))`](crate::live_collections::stream::Stream::observe_drops).
    #[must_use]
    pub const fn lossy(self) -> NetworkingConfig<Udp<Lossy>, S> {
        NetworkingConfig {
            name: self.name,
            _phantom: (PhantomData, PhantomData),
        }
    }

    /// Configures the UDP transport to treat dropped messages as indefinitely delayed.
    ///
    /// UDP never guarantees delivery or ordering, so unlike TCP there is no `fail_stop`
    /// policy. Dropped messages are modeled as infinite delays: the received stream is
    /// [`NoOrder`] and [`MayDrop`], observed with only a [`nonlive!`](crate::liveness::nonlive).
    ///
    /// When those drops are observed in the Hydro simulator, you must call
    /// [`.test_safety_only()`](crate::sim::flow::SimFlow::test_safety_only): the simulator
    /// does not actually drop packets, but delays "dropped" messages until the end of the
    /// execution, which catches safety bugs but cannot test liveness.
    #[must_use]
    pub const fn lossy_delayed_forever(self) -> NetworkingConfig<Udp<LossyDelayedForever>, S> {
        NetworkingConfig {
            name: self.name,
            _phantom: (PhantomData, PhantomData),
        }
    }
}

#[sealed::sealed]
impl<Tr, S, T: ?Sized> NetworkFor<T> for NetworkingConfig<Tr, S>
where
    Tr: ?Sized + TransportKind,
    S: ?Sized + SerKind<T>,
{
    type OrderingGuarantee = Tr::OrderingGuarantee;

    type RetriesGuarantee = Tr::RetriesGuarantee;

    type DropsGuarantee = Tr::DropsGuarantee;

    type ConsistencyGuarantee = Tr::ConsistencyGuarantee;

    fn serialize_thunk(is_demux: bool) -> syn::Expr {
        S::serialize_thunk(is_demux)
    }

    fn deserialize_thunk(tagged: Option<&syn::Type>) -> syn::Expr {
        S::deserialize_thunk(tagged)
    }

    fn is_embedded() -> bool {
        S::is_embedded()
    }

    fn name(&self) -> Option<&str> {
        None
    }

    fn networking_info() -> NetworkingInfo {
        Tr::networking_info()
    }
}

#[sealed::sealed]
impl<Tr, S, T: ?Sized> NetworkFor<T> for NetworkingConfig<Tr, S, String>
where
    Tr: ?Sized + TransportKind,
    S: ?Sized + SerKind<T>,
{
    type OrderingGuarantee = Tr::OrderingGuarantee;

    type RetriesGuarantee = Tr::RetriesGuarantee;

    type DropsGuarantee = Tr::DropsGuarantee;

    type ConsistencyGuarantee = Tr::ConsistencyGuarantee;

    fn serialize_thunk(is_demux: bool) -> syn::Expr {
        S::serialize_thunk(is_demux)
    }

    fn deserialize_thunk(tagged: Option<&syn::Type>) -> syn::Expr {
        S::deserialize_thunk(tagged)
    }

    fn is_embedded() -> bool {
        S::is_embedded()
    }

    fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    fn networking_info() -> NetworkingInfo {
        Tr::networking_info()
    }
}

/// A network channel that uses length-delimited TCP for transport.
pub const TCP: NetworkingConfig<Tcp<()>, NoSer> = NetworkingConfig {
    name: None,
    _phantom: (PhantomData, PhantomData),
};

/// A network channel implemented by the host of an embedded program, which chooses how messages
/// travel: a network connection, an in-memory queue, or anything else. The program declares, for
/// each anomaly, whether the host's channel may introduce it, and the received stream is weakened
/// by each one it may:
///
/// - reordering: [`never_reorders`](NetworkingConfig::never_reorders) or
///   [`may_reorder`](NetworkingConfig::may_reorder);
/// - duplication: [`never_duplicates`](NetworkingConfig::never_duplicates) or
///   [`may_duplicate`](NetworkingConfig::may_duplicate);
/// - loss: [`never_drops`](NetworkingConfig::never_drops),
///   [`may_drop`](NetworkingConfig::may_drop), or
///   [`may_delay_forever`](NetworkingConfig::may_delay_forever).
///
/// Each axis must be declared (for example
/// `EMBEDDED.may_reorder().never_duplicates().never_drops().name("messages")`). The `never_*`
/// declarations hold as long as the channel remains live.
///
/// Items cross the channel unserialized: the generated channel ends carry the raw element type,
/// and the host encodes it if its transport needs to. The program names only the anomalies, not
/// a transport, so it is only supported by the embedded deployment backend and the Hydro
/// simulator (which carries the raw values in memory); other backends use [`TCP`] or [`UDP`].
pub const EMBEDDED: NetworkingConfig<EmbeddedTransport<(), (), ()>, Embedded> = NetworkingConfig {
    name: None,
    _phantom: (PhantomData, PhantomData),
};

/// A network channel that uses UDP for transport.
///
/// Unlike [`TCP`], UDP does not guarantee delivery or ordering, so output streams
/// always have [`NoOrder`] guarantees. Because UDP is connectionless, there is no
/// `fail_stop` policy; only [`lossy`](NetworkingConfig::lossy) and
/// [`lossy_delayed_forever`](NetworkingConfig::lossy_delayed_forever) are available.
///
/// # Availability
/// UDP is **not yet available** in "deploy" deployment mode (via Hydro Deploy,
/// including Docker and ECS deployments); attempting to deploy a UDP channel there
/// will panic at compile time. Both UDP modes are available for embedded
/// deployments (the only production deployment option) and Maelstrom testing. In
/// the Hydro simulator, both deliver every message, and the received stream's drops are
/// explored where the program observes them (see
/// [`observe_drops`](crate::live_collections::stream::Stream::observe_drops)).
pub const UDP: NetworkingConfig<Udp<()>, NoSer> = NetworkingConfig {
    name: None,
    _phantom: (PhantomData, PhantomData),
};

#[cfg(test)]
mod tests {
    use super::*;

    /// Compiles only if `K` gives retries `R`, ordering `O`, and consistency guarantee `C`.
    fn guarantees<K, R, O, C>()
    where
        K: TransportKind<RetriesGuarantee = R, OrderingGuarantee = O, ConsistencyGuarantee = C>,
    {
    }

    /// An `EMBEDDED` channel's retries and ordering are the declared ones (except that
    /// `lossy_delayed_forever` is `NoOrder`), and its consistency guarantee follows from its
    /// failure policy: only `lossy` drops to `NoConsistency`.
    #[test]
    fn embedded_guarantees_follow_declaration() {
        type E<R, O, F> = EmbeddedTransport<R, O, F>;
        guarantees::<
            E<ExactlyOnce, TotalOrder, FailStop>,
            ExactlyOnce,
            TotalOrder,
            EventualConsistency,
        >();
        guarantees::<E<AtLeastOnce, NoOrder, FailStop>, AtLeastOnce, NoOrder, EventualConsistency>(
        );
        guarantees::<E<ExactlyOnce, TotalOrder, Lossy>, ExactlyOnce, TotalOrder, NoConsistency>();
        guarantees::<E<AtLeastOnce, NoOrder, Lossy>, AtLeastOnce, NoOrder, NoConsistency>();
        guarantees::<
            E<ExactlyOnce, TotalOrder, LossyDelayedForever>,
            ExactlyOnce,
            NoOrder,
            EventualConsistency,
        >();
        guarantees::<
            E<AtLeastOnce, NoOrder, LossyDelayedForever>,
            AtLeastOnce,
            NoOrder,
            EventualConsistency,
        >();
    }
}
