//! Types for representing external processes that communicate with a Hydro dataflow.
//!
//! An **external process** is a process that lives outside the Hydro dataflow graph but
//! can send data to and receive data from locations within the graph. This is the primary
//! mechanism for feeding input into a Hydro program and observing its output at runtime.
//!
//! The main type in this module is [`External`], which represents a handle to an external
//! process. Port types such as [`ExternalBytesPort`], [`ExternalBincodeSink`],
//! [`ExternalBincodeBidi`], and [`ExternalBincodeStream`] represent the different kinds of
//! communication channels that can be established between an external process and the
//! dataflow.

use std::marker::PhantomData;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::compile::builder::{ExternalPortId, FlowState};
use crate::live_collections::stream::{ExactlyOnce, Ordering, Retries, TotalOrder};
use crate::location::LocationKey;
use crate::staging_util::Invariant;

/// Marker type indicating that a port is connected to a single external client.
pub enum NotMany {}
/// Marker type indicating that a port is connected to multiple external clients.
pub enum Many {}

/// A port handle for sending or receiving raw bytes with an external process.
///
/// The `M` type parameter is either [`NotMany`] (single client) or [`Many`] (multiple
/// clients). When `M` is [`Many`], the port can be cloned to allow multiple external
/// clients to connect.
pub struct ExternalBytesPort<M = NotMany> {
    pub(crate) process_key: LocationKey,
    pub(crate) port_id: ExternalPortId,
    pub(crate) _phantom: PhantomData<M>,
}

impl Clone for ExternalBytesPort<Many> {
    fn clone(&self) -> Self {
        Self {
            process_key: self.process_key,
            port_id: self.port_id,
            _phantom: Default::default(),
        }
    }
}

/// A sink handle for sending bincode-serialized data from an external process into the
/// dataflow.
///
/// The type parameters control the serialized type (`Type`), whether multiple clients
/// are supported (`Many`), the ordering guarantee (`O`), and the retry semantics (`R`).
pub struct ExternalBincodeSink<
    Type,
    Many = NotMany,
    O: Ordering = TotalOrder,
    R: Retries = ExactlyOnce,
> where
    Type: Serialize,
{
    pub(crate) process_key: LocationKey,
    pub(crate) port_id: ExternalPortId,
    pub(crate) _phantom: PhantomData<(Type, Many, O, R)>,
}

impl<T: Serialize, O: Ordering, R: Retries> Clone for ExternalBincodeSink<T, Many, O, R> {
    fn clone(&self) -> Self {
        Self {
            process_key: self.process_key,
            port_id: self.port_id,
            _phantom: Default::default(),
        }
    }
}

/// A bidirectional port handle for exchanging bincode-serialized data with an external
/// process.
///
/// `InType` is the type of messages received from the external process, and `OutType` is
/// the type of messages sent back. The `M` type parameter controls whether multiple
/// clients are supported ([`NotMany`] or [`Many`]).
pub struct ExternalBincodeBidi<InType, OutType, M = NotMany> {
    pub(crate) process_key: LocationKey,
    pub(crate) port_id: ExternalPortId,
    pub(crate) _phantom: PhantomData<(InType, OutType, M)>,
}

impl<InT, OutT> Clone for ExternalBincodeBidi<InT, OutT, Many> {
    fn clone(&self) -> Self {
        Self {
            process_key: self.process_key,
            port_id: self.port_id,
            _phantom: Default::default(),
        }
    }
}

/// A stream handle for receiving bincode-serialized data from an external process.
///
/// The type parameters control the deserialized element type (`Type`), the ordering
/// guarantee (`O`), and the retry semantics (`R`).
pub struct ExternalBincodeStream<Type, O: Ordering = TotalOrder, R: Retries = ExactlyOnce>
where
    Type: DeserializeOwned,
{
    #[cfg_attr(
        not(feature = "build"),
        expect(unused, reason = "unused without feature")
    )]
    pub(crate) process_key: LocationKey,
    #[cfg_attr(
        not(feature = "build"),
        expect(unused, reason = "unused without feature")
    )]
    pub(crate) port_id: ExternalPortId,
    pub(crate) _phantom: PhantomData<(Type, O, R)>,
}

/// A handle representing an external process that can communicate with the Hydro dataflow.
///
/// External processes live outside the compiled dataflow graph and interact with it by
/// sending and receiving data through ports. Use methods on [`Location`](super::Location)
/// such as [`source_external_bytes`](super::Location::source_external_bytes),
/// [`source_external_bincode`](super::Location::source_external_bincode), and
/// [`bind_single_client`](super::Location::bind_single_client) to establish communication
/// channels between a location and an external process.
///
/// The `Tag` type parameter is a user-defined marker type that distinguishes different
/// external process roles at the type level.
pub struct External<'a, Tag> {
    pub(crate) key: LocationKey,

    pub(crate) flow_state: FlowState,

    pub(crate) _phantom: Invariant<'a, Tag>,
}

impl<P> Clone for External<'_, P> {
    fn clone(&self) -> Self {
        External {
            key: self.key,
            flow_state: self.flow_state.clone(),
            _phantom: PhantomData,
        }
    }
}
