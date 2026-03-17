//! Types for configuring network channels with serialization formats, transports, etc.

use std::marker::PhantomData;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::live_collections::stream::networking::{deserialize_bincode, serialize_bincode};
use crate::live_collections::stream::{NoOrder, TotalOrder};
use crate::nondet::NonDet;

#[sealed::sealed]
trait SerKind<T: ?Sized> {
    fn serialize_thunk(is_demux: bool) -> syn::Expr;

    fn deserialize_thunk(tagged: Option<&syn::Type>) -> syn::Expr;
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

/// An unconfigured serialization backend.
pub enum NoSer {}

/// A transport backend for network channels.
#[sealed::sealed]
pub trait TransportKind {
    /// The ordering guarantee provided by this transport.
    type OrderingGuarantee: crate::live_collections::stream::Ordering;

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

    /// Returns the [`TcpFault`] variant for this failure policy.
    fn tcp_fault() -> TcpFault;
}

/// A TCP failure policy that stops sending messages after a failed connection.
pub enum FailStop {}
#[sealed::sealed]
impl TcpFailPolicy for FailStop {
    type OrderingGuarantee = TotalOrder;

    fn tcp_fault() -> TcpFault {
        TcpFault::FailStop
    }
}

/// A TCP failure policy that allows messages to be lost.
pub enum Lossy {}
#[sealed::sealed]
impl TcpFailPolicy for Lossy {
    type OrderingGuarantee = TotalOrder;

    fn tcp_fault() -> TcpFault {
        TcpFault::Lossy
    }
}

/// A TCP failure policy that treats dropped messages as indefinitely delayed.
///
/// Unlike [`Lossy`], this does not require a [`NonDet`] annotation because the output
/// stream is always lower in the partial order than the ideal stream (dropped messages
/// are modeled as infinite delays). The tradeoff is that the output has [`NoOrder`]
/// guarantees, imposing stricter conditions on downstream consumers.
///
/// When using this mode in the Hydro simulator, you must call
/// [`.test_safety_only()`](crate::sim::flow::SimFlow::test_safety_only) because the
/// simulator models dropped messages as indefinitely delayed, which only tests safety
/// properties (not liveness).
pub enum LossyDelayedForever {}
#[sealed::sealed]
impl TcpFailPolicy for LossyDelayedForever {
    type OrderingGuarantee = NoOrder;

    fn tcp_fault() -> TcpFault {
        TcpFault::LossyDelayedForever
    }
}

/// Send items across a length-delimited TCP channel.
pub struct Tcp<F> {
    _phantom: PhantomData<F>,
}

#[sealed::sealed]
impl<F: TcpFailPolicy> TransportKind for Tcp<F> {
    type OrderingGuarantee = F::OrderingGuarantee;

    fn networking_info() -> NetworkingInfo {
        NetworkingInfo::Tcp {
            fault: F::tcp_fault(),
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

    /// Generates serialization logic for sending `T`.
    fn serialize_thunk(is_demux: bool) -> syn::Expr;

    /// Generates deserialization logic for receiving `T`.
    fn deserialize_thunk(tagged: Option<&syn::Type>) -> syn::Expr;

    /// Returns the optional name of the network channel.
    fn name(&self) -> Option<&str>;

    /// Returns the [`NetworkingInfo`] describing this network channel's transport and fault model.
    fn networking_info() -> NetworkingInfo;
}

/// The fault model for a TCP connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TcpFault {
    /// Stops sending messages after a failed connection.
    FailStop,
    /// Messages may be lost (e.g. due to network partitions).
    Lossy,
    /// Dropped messages are treated as indefinitely delayed with no ordering guarantee.
    LossyDelayedForever,
}

/// Describes the networking configuration for a network channel at the IR level.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NetworkingInfo {
    /// A TCP-based network channel with a specific fault model.
    Tcp {
        /// The fault model for this TCP connection.
        fault: TcpFault,
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
    /// a prefix of messages is delivered, `lossy` makes no such guarantee.
    ///
    /// # Non-Determinism
    /// A lossy TCP channel will non-deterministically drop messages during execution.
    pub const fn lossy(self, nondet: NonDet) -> NetworkingConfig<Tcp<Lossy>, S> {
        let _ = nondet;
        NetworkingConfig {
            name: self.name,
            _phantom: (PhantomData, PhantomData),
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
    /// [`.test_safety_only()`](crate::sim::flow::SimFlow::test_safety_only) to opt in,
    /// because the simulator models dropped messages as indefinitely delayed, which only
    /// tests safety properties (not liveness).
    pub const fn lossy_delayed_forever(self) -> NetworkingConfig<Tcp<LossyDelayedForever>, S> {
        NetworkingConfig {
            name: self.name,
            _phantom: (PhantomData, PhantomData),
        }
    }
}

#[sealed::sealed]
impl<Tr: ?Sized, S: ?Sized, T: ?Sized> NetworkFor<T> for NetworkingConfig<Tr, S>
where
    Tr: TransportKind,
    S: SerKind<T>,
{
    type OrderingGuarantee = Tr::OrderingGuarantee;

    fn serialize_thunk(is_demux: bool) -> syn::Expr {
        S::serialize_thunk(is_demux)
    }

    fn deserialize_thunk(tagged: Option<&syn::Type>) -> syn::Expr {
        S::deserialize_thunk(tagged)
    }

    fn name(&self) -> Option<&str> {
        None
    }

    fn networking_info() -> NetworkingInfo {
        Tr::networking_info()
    }
}

#[sealed::sealed]
impl<Tr: ?Sized, S: ?Sized, T: ?Sized> NetworkFor<T> for NetworkingConfig<Tr, S, String>
where
    Tr: TransportKind,
    S: SerKind<T>,
{
    type OrderingGuarantee = Tr::OrderingGuarantee;

    fn serialize_thunk(is_demux: bool) -> syn::Expr {
        S::serialize_thunk(is_demux)
    }

    fn deserialize_thunk(tagged: Option<&syn::Type>) -> syn::Expr {
        S::deserialize_thunk(tagged)
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
