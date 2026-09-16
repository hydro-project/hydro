//! Types for configuring network channels with serialization formats, transports, etc.

use std::marker::PhantomData;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::live_collections::stream::networking::{deserialize_bincode, serialize_bincode};
use crate::live_collections::stream::{NoOrder, TotalOrder};
use crate::location::cluster::{Consistency, EventualConsistency, NoConsistency};
use crate::nondet::NonDet;
use crate::sim_hooks::LossHook;

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

/// Leaves serialization of items to code outside of Hydro.
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
    type OrderingGuarantee: crate::live_collections::stream::Ordering;

    /// The consistency guarantee this transport can preserve for replicated outputs
    /// (see [`NetworkFor::ConsistencyGuarantee`]).
    type ConsistencyGuarantee: Consistency;

    /// Returns the [`NetworkingInfo`] describing this transport's configuration.
    fn networking_info() -> NetworkingInfo;
}

#[sealed::sealed]
#[diagnostic::on_unimplemented(
    message = "TCP transport requires a failure policy. For example, `TCP.fail_stop()` stops sending messages after a failed connection."
)]
/// A failure policy for TCP connections, determining how the transport handles
/// connection failures and what ordering guarantees the output stream provides.
pub trait TcpFailPolicy {
    /// The ordering guarantee provided by this failure policy.
    type OrderingGuarantee: crate::live_collections::stream::Ordering;

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

    // A failed connection stops *all* future deliveries to that recipient, which models the
    // recipient as having failed. Consistency guarantees only apply to live members, so
    // eventual consistency of replicated outputs is preserved.
    type ConsistencyGuarantee = EventualConsistency;

    fn tcp_fault() -> TcpFault {
        TcpFault::FailStop
    }
}

/// A failure policy that allows messages to be lost.
pub enum Lossy {}
#[sealed::sealed]
impl TcpFailPolicy for Lossy {
    type OrderingGuarantee = TotalOrder;

    // A lossy channel can drop an arbitrary message for one recipient while continuing to
    // deliver later messages, so replicated outputs can diverge across members forever.
    type ConsistencyGuarantee = NoConsistency;

    fn tcp_fault() -> TcpFault {
        TcpFault::Lossy
    }
}

/// A failure policy that treats dropped messages as indefinitely delayed.
///
/// Unlike [`Lossy`], this does not require a [`NonDet`] annotation because the output
/// stream is always lower in the partial order than the ideal stream (dropped messages
/// are modeled as infinite delays). The tradeoff is that the output has [`NoOrder`]
/// guarantees, imposing stricter conditions on downstream consumers.
///
/// When using this mode in the Hydro simulator, you must call
/// [`.test_safety_only()`](crate::sim::flow::SimFlow::test_safety_only): the simulator
/// will not actually drop packets—it delays "dropped" messages until the end of the
/// execution, which catches safety bugs but cannot test liveness.
pub enum LossyDelayedForever {}
#[sealed::sealed]
impl TcpFailPolicy for LossyDelayedForever {
    type OrderingGuarantee = NoOrder;

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
    message = "UDP transport requires a failure policy. For example, `UDP.lossy_delayed_forever()` treats dropped messages as indefinitely delayed."
)]
/// A failure policy for UDP channels, determining how the transport handles
/// message loss. Because UDP provides no ordering guarantees, all policies
/// produce [`NoOrder`] output streams, and there is no `fail_stop` option
/// (UDP is connectionless, so there is no connection to fail).
pub trait UdpFailPolicy {
    /// The consistency guarantee this failure policy can preserve for replicated outputs
    /// (see [`NetworkFor::ConsistencyGuarantee`]).
    type ConsistencyGuarantee: Consistency;

    /// Returns the [`UdpFault`] variant for this failure policy.
    fn udp_fault() -> UdpFault;
}

#[sealed::sealed]
impl UdpFailPolicy for Lossy {
    // A lossy channel can drop an arbitrary message for one recipient while continuing to
    // deliver later messages, so replicated outputs can diverge across members forever.
    type ConsistencyGuarantee = NoConsistency;

    fn udp_fault() -> UdpFault {
        UdpFault::Lossy
    }
}

#[sealed::sealed]
impl UdpFailPolicy for LossyDelayedForever {
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

    type ConsistencyGuarantee = F::ConsistencyGuarantee;

    fn networking_info() -> NetworkingInfo {
        NetworkingInfo::Tcp {
            fault: F::tcp_fault(),
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
    type OrderingGuarantee: crate::live_collections::stream::Ordering;

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

    /// Whether this network channel leaves serialization to code outside of Hydro (see
    /// [`Embedded`]). When `true`, [`Self::serialize_thunk`] and [`Self::deserialize_thunk`] are
    /// never called; the raw element type flows across the channel unserialized.
    fn is_embedded() -> bool {
        false
    }

    /// Returns the optional name of the network channel.
    fn name(&self) -> Option<&str>;

    /// Returns the ID of the simulator hook handle bound to this channel's fault
    /// non-determinism, if any (see [`crate::sim_hooks::LossHook`] and
    /// [`NetworkingConfig::lossy`]). Ignored by non-simulator backends.
    fn sim_hook_id(&self) -> Option<usize> {
        None
    }

    /// Returns the [`NetworkingInfo`] describing this network channel's transport and fault model.
    fn networking_info() -> NetworkingInfo;
}

/// A [`NetworkFor`] whose fault-non-determinism hook (if any) matches the channel's
/// **endpoints**: `FromScope` / `ToScope` name the kind (and tag) of the sending and
/// receiving locations, mirroring
/// [`Location::SimHookScope`](crate::location::Location::SimHookScope)
/// ([`OnProcess<P>`](crate::sim_hooks::OnProcess) /
/// [`OnCluster<C>`](crate::sim_hooks::OnCluster)).
///
/// Network operators (`send`, `demux`, `broadcast`, ...) bound their configuration by
/// this trait so that a [`LossHook`] bound via [`NetworkingConfig::lossy`] is typed by
/// the link it controls — the decision surface depends on both ends:
///
/// - The **receiving** end is the handle's scope: on a channel *to a cluster*, every
///   member receives through its own independent channel instance, selected with
///   [`.on(member_id)`](crate::sim_hooks::LossHook::on).
/// - The **sending** end shapes the decisions: on a channel *from a cluster*, each
///   in-flight message belongs to a sender, so decisions name `(sender_id, value)` (and
///   the positional forms take the sender whose front to resolve).
///
/// Configurations without a hook (`()` payload) implement this trait for every endpoint
/// shape.
#[diagnostic::on_unimplemented(
    message = "this network configuration cannot be used for a channel from `{FromScope}` to `{ToScope}`",
    note = "a `LossHook` bound with `lossy(nondet!(... hook = handle))` is typed by the channel's endpoints: the handle's sender scope must be the sending location's kind (`OnProcess<P>` / `OnCluster<C>`) and its trailing scope the receiving location's kind"
)]
#[sealed::sealed]
pub trait NetworkForLink<T: ?Sized, FromScope, ToScope>: NetworkFor<T> {}

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
}

/// A network channel configuration with `T` as transport backend and `S` as the serialization
/// backend.
///
/// The `Hook` parameter carries the simulator hook payload type bound to the channel's
/// fault non-determinism, when the fault policy has any (see [`Self::lossy`] and
/// [`crate::sim_hooks::LossHook`]); it is `()` for fault policies with no hookable
/// decision.
pub struct NetworkingConfig<Tr: ?Sized, S: ?Sized, Name = (), Hook = ()> {
    name: Option<Name>,
    /// The ID of the bound simulator hook handle, if any. The handle's full type lives
    /// in the `Hook` parameter (so `NetworkFor<T>` can check it against the channel's
    /// element type); only the ID is needed at runtime.
    sim_hook_id: Option<usize>,
    _phantom: (PhantomData<Tr>, PhantomData<S>, PhantomData<Hook>),
}

impl<Tr: ?Sized, S: ?Sized, Hook> NetworkingConfig<Tr, S, (), Hook> {
    /// Names the network channel and enables stable communication across multiple service versions.
    pub fn name(self, name: impl Into<String>) -> NetworkingConfig<Tr, S, String, Hook> {
        NetworkingConfig {
            name: Some(name.into()),
            sim_hook_id: self.sim_hook_id,
            _phantom: (PhantomData, PhantomData, PhantomData),
        }
    }
}

impl<Tr: ?Sized, N, Hook> NetworkingConfig<Tr, NoSer, N, Hook> {
    /// Configures the network channel to use [`bincode`] to serialize items.
    pub const fn bincode(mut self) -> NetworkingConfig<Tr, Bincode, N, Hook> {
        let taken_name = self.name.take();
        let sim_hook_id = self.sim_hook_id;
        std::mem::forget(self); // nothing else is stored
        NetworkingConfig {
            name: taken_name,
            sim_hook_id,
            _phantom: (PhantomData, PhantomData, PhantomData),
        }
    }

    /// Configures the network channel to leave serialization to code outside of Hydro.
    ///
    /// This is only supported by the embedded deployment backend (it will panic on all other
    /// backends). The generated network channel exposes the raw element type to the developer
    /// (rather than serialized bytes), so they can perform custom serialization logic outside of
    /// the Hydro program for that channel.
    pub const fn embedded(mut self) -> NetworkingConfig<Tr, Embedded, N, Hook> {
        let taken_name = self.name.take();
        let sim_hook_id = self.sim_hook_id;
        std::mem::forget(self); // nothing else is stored
        NetworkingConfig {
            name: taken_name,
            sim_hook_id,
            _phantom: (PhantomData, PhantomData, PhantomData),
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
            sim_hook_id: None,
            _phantom: (PhantomData, PhantomData, PhantomData),
        }
    }

    /// Configures the TCP transport to allow messages to be lost.
    ///
    /// This is appropriate for networks where messages may be dropped, such as when
    /// running under a Maelstrom partition nemesis. Unlike `fail_stop`, which guarantees
    /// a prefix of messages is delivered, `lossy` makes no such guarantee.
    ///
    /// # Non-Determinism
    /// A lossy TCP channel will non-deterministically drop messages during execution.
    /// Because a legal execution may drop *every* message, the simulator cannot explore
    /// this non-determinism autonomously (no exploration strategy is fair to programs
    /// that need messages delivered). Under simulation, the guard **must** carry a
    /// [`LossHook`] handle (`TCP.lossy(nondet!(... hook = handle))`), and the test
    /// scripts each in-flight message's fate with `deliver` / `lose` decisions;
    /// simulating an unhooked lossy channel is a build-time error. Alternatively, use
    /// [`Self::lossy_delayed_forever`] to model drops as indefinite delays, which the
    /// simulator can explore autonomously.
    ///
    /// The handle is typed by the channel's **endpoints** (see
    /// [`NetworkForLink`]), usually inferred from the `send` / `demux` / `broadcast`
    /// call: on a channel to a cluster each recipient member's instance is selected
    /// with `.on(member_id)`, and on a channel from a cluster decisions name
    /// `(sender_id, value)`.
    #[must_use]
    pub fn lossy<T, FromScope, ToScope>(
        self,
        mut nondet: NonDet<Option<LossHook<T, TotalOrder, FromScope, ToScope>>>,
    ) -> NetworkingConfig<Tcp<Lossy>, S, (), Option<LossHook<T, TotalOrder, FromScope, ToScope>>>
    {
        NetworkingConfig {
            name: self.name,
            sim_hook_id: nondet.take_hook().map(|hook| hook.id),
            _phantom: (PhantomData, PhantomData, PhantomData),
        }
    }

    /// Configures the TCP transport to treat dropped messages as indefinitely delayed.
    ///
    /// This is appropriate for networks where messages may be dropped, such as when
    /// running under a Maelstrom partition nemesis. Unlike [`Self::lossy`], this does
    /// *not* require a [`NonDet`] annotation because the output is always lower in the
    /// partial order than the ideal stream. However, the output stream will have
    /// [`NoOrder`] guarantees, imposing stricter conditions on downstream consumers.
    ///
    /// Unlike [`Self::lossy`], this mode can easily be simulated in exhaustive mode
    /// without running into fairness issues.
    ///
    /// When using this mode in the Hydro simulator, you must call
    /// [`.test_safety_only()`](crate::sim::flow::SimFlow::test_safety_only) to opt in:
    /// the simulator will not actually drop packets—it delays "dropped" messages until
    /// the end of the execution, which catches safety bugs but cannot test liveness.
    #[must_use]
    pub const fn lossy_delayed_forever(self) -> NetworkingConfig<Tcp<LossyDelayedForever>, S> {
        NetworkingConfig {
            name: self.name,
            sim_hook_id: None,
            _phantom: (PhantomData, PhantomData, PhantomData),
        }
    }
}

impl<S: ?Sized> NetworkingConfig<Udp<()>, S> {
    /// Configures the UDP transport to allow messages to be lost.
    ///
    /// UDP never guarantees delivery or ordering, so unlike TCP there is no `fail_stop`
    /// policy — messages may always be dropped and the output stream always has
    /// [`NoOrder`] guarantees.
    ///
    /// # Non-Determinism
    /// A lossy UDP channel will non-deterministically drop messages during execution.
    /// Because a legal execution may drop *every* message, the simulator cannot explore
    /// this non-determinism autonomously (no exploration strategy is fair to programs
    /// that need messages delivered). Under simulation, the guard **must** carry a
    /// [`LossHook`] handle (`UDP.lossy(nondet!(... hook = handle))`), and the test
    /// scripts each in-flight message's fate with `deliver` / `lose` decisions;
    /// simulating an unhooked lossy channel is a build-time error. Alternatively, use
    /// [`Self::lossy_delayed_forever`] to model drops as indefinite delays, which the
    /// simulator can explore autonomously.
    ///
    /// The handle is typed by the channel's **endpoints** (see
    /// [`NetworkForLink`]), usually inferred from the `send` / `demux` / `broadcast`
    /// call: on a channel to a cluster each recipient member's instance is selected
    /// with `.on(member_id)`, and on a channel from a cluster decisions name
    /// `(sender_id, value)`.
    #[must_use]
    pub fn lossy<T, FromScope, ToScope>(
        self,
        mut nondet: NonDet<Option<LossHook<T, NoOrder, FromScope, ToScope>>>,
    ) -> NetworkingConfig<Udp<Lossy>, S, (), Option<LossHook<T, NoOrder, FromScope, ToScope>>> {
        NetworkingConfig {
            name: self.name,
            sim_hook_id: nondet.take_hook().map(|hook| hook.id),
            _phantom: (PhantomData, PhantomData, PhantomData),
        }
    }

    /// Configures the UDP transport to treat dropped messages as indefinitely delayed.
    ///
    /// UDP never guarantees delivery or ordering, so unlike TCP there is no `fail_stop`
    /// policy. Unlike [`Self::lossy`], this does *not* require a [`NonDet`] annotation
    /// because the output is always lower in the partial order than the ideal stream
    /// (dropped messages are modeled as infinite delays). The output stream has
    /// [`NoOrder`] guarantees, imposing stricter conditions on downstream consumers.
    ///
    /// Unlike [`Self::lossy`], this mode can easily be simulated in exhaustive mode
    /// without running into fairness issues.
    ///
    /// When using this mode in the Hydro simulator, you must call
    /// [`.test_safety_only()`](crate::sim::flow::SimFlow::test_safety_only) to opt in:
    /// the simulator will not actually drop packets—it delays "dropped" messages until
    /// the end of the execution, which catches safety bugs but cannot test liveness.
    #[must_use]
    pub const fn lossy_delayed_forever(self) -> NetworkingConfig<Udp<LossyDelayedForever>, S> {
        NetworkingConfig {
            name: self.name,
            sim_hook_id: None,
            _phantom: (PhantomData, PhantomData, PhantomData),
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

// The hookable variants: a config whose fault policy carries a `LossHook` payload (see
// `lossy`) implements `NetworkFor<T>` only when the hook's element type is the channel's
// element type (and its ordering the transport's guarantee), so a mistyped handle is an
// ordinary compile error at the `send`/`broadcast` call.
#[sealed::sealed]
impl<Tr, S, T, FromScope, ToScope> NetworkFor<T>
    for NetworkingConfig<Tr, S, (), Option<LossHook<T, Tr::OrderingGuarantee, FromScope, ToScope>>>
where
    Tr: ?Sized + TransportKind,
    S: ?Sized + SerKind<T>,
{
    type OrderingGuarantee = Tr::OrderingGuarantee;

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

    fn sim_hook_id(&self) -> Option<usize> {
        self.sim_hook_id
    }

    fn networking_info() -> NetworkingInfo {
        Tr::networking_info()
    }
}

#[sealed::sealed]
impl<Tr, S, T, FromScope, ToScope> NetworkFor<T>
    for NetworkingConfig<
        Tr,
        S,
        String,
        Option<LossHook<T, Tr::OrderingGuarantee, FromScope, ToScope>>,
    >
where
    Tr: ?Sized + TransportKind,
    S: ?Sized + SerKind<T>,
{
    type OrderingGuarantee = Tr::OrderingGuarantee;

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

    fn sim_hook_id(&self) -> Option<usize> {
        self.sim_hook_id
    }

    fn networking_info() -> NetworkingInfo {
        Tr::networking_info()
    }
}

// `NetworkForLink` pins a hooked configuration's endpoint scopes to the link it is used
// for (see the trait docs); configurations without a hook fit any link.
#[sealed::sealed]
impl<Tr, S, T: ?Sized, FromScope, ToScope> NetworkForLink<T, FromScope, ToScope>
    for NetworkingConfig<Tr, S>
where
    Tr: ?Sized + TransportKind,
    S: ?Sized + SerKind<T>,
{
}

#[sealed::sealed]
impl<Tr, S, T: ?Sized, FromScope, ToScope> NetworkForLink<T, FromScope, ToScope>
    for NetworkingConfig<Tr, S, String>
where
    Tr: ?Sized + TransportKind,
    S: ?Sized + SerKind<T>,
{
}

#[sealed::sealed]
impl<Tr, S, T, FromScope, ToScope> NetworkForLink<T, FromScope, ToScope>
    for NetworkingConfig<Tr, S, (), Option<LossHook<T, Tr::OrderingGuarantee, FromScope, ToScope>>>
where
    Tr: ?Sized + TransportKind,
    S: ?Sized + SerKind<T>,
{
}

#[sealed::sealed]
impl<Tr, S, T, FromScope, ToScope> NetworkForLink<T, FromScope, ToScope>
    for NetworkingConfig<
        Tr,
        S,
        String,
        Option<LossHook<T, Tr::OrderingGuarantee, FromScope, ToScope>>,
    >
where
    Tr: ?Sized + TransportKind,
    S: ?Sized + SerKind<T>,
{
}

/// A network channel that uses length-delimited TCP for transport.
pub const TCP: NetworkingConfig<Tcp<()>, NoSer> = NetworkingConfig {
    name: None,
    sim_hook_id: None,
    _phantom: (PhantomData, PhantomData, PhantomData),
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
/// the Hydro simulator, `lossy` requires binding a [`LossHook`] that scripts each
/// message's delivery, and `lossy_delayed_forever` requires
/// [`.test_safety_only()`](crate::sim::flow::SimFlow::test_safety_only): the
/// simulator will not actually drop packets—it delays "dropped" messages until the
/// end of the execution, which catches safety bugs but cannot test liveness.
pub const UDP: NetworkingConfig<Udp<()>, NoSer> = NetworkingConfig {
    name: None,
    sim_hook_id: None,
    _phantom: (PhantomData, PhantomData, PhantomData),
};
