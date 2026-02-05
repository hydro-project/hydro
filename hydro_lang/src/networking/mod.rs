//! Types for configuring network channels with serialization formats, transports, etc.

use std::marker::PhantomData;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::live_collections::stream::networking::{deserialize_bincode, serialize_bincode};

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

#[sealed::sealed]
trait TransportKind {}

#[sealed::sealed]
#[diagnostic::on_unimplemented(
    message = "TCP transport requires a failure policy. For example, `TCP.fail_stop()` stops sending messages after a failed connection."
)]
trait TcpFailPolicy {}

/// A TCP failure policy that stops sending messages after a failed connection.
pub enum FailStop {}
#[sealed::sealed]
impl TcpFailPolicy for FailStop {}

/// Send items across a length-delimited TCP channel.
pub struct Tcp<F> {
    _phantom: PhantomData<F>,
}

#[sealed::sealed]
impl<F: TcpFailPolicy> TransportKind for Tcp<F> {}

/// A networking backend implementation that supports items of type `T`.
#[sealed::sealed]
pub trait NetworkFor<T: ?Sized> {
    /// Generates serialization logic for sending `T`.
    fn serialize_thunk(is_demux: bool) -> syn::Expr;

    /// Generates deserialization logic for receiving `T`.
    fn deserialize_thunk(tagged: Option<&syn::Type>) -> syn::Expr;

    /// Returns the optional name of the network channel.
    fn name(&self) -> Option<&str>;
}

/// A network channel configuration with `T` as transport backend and `S` as the serialization
/// backend.
pub struct NetworkingConfig<Tr: ?Sized, S: ?Sized, Name = ()> {
    name: Option<Name>,
    _phantom: (PhantomData<Tr>, PhantomData<S>),
}

impl<Tr: ?Sized, S: ?Sized> NetworkingConfig<Tr, S> {
    /// Configures the network channel to use [`bincode`] to serialize items.
    pub const fn bincode(self) -> NetworkingConfig<Tr, Bincode> {
        NetworkingConfig {
            name: self.name,
            _phantom: (PhantomData, PhantomData),
        }
    }

    /// Names the network channel and enables stable communication across multiple service versions.
    pub fn name(self, name: impl Into<String>) -> NetworkingConfig<Tr, S, String> {
        NetworkingConfig {
            name: Some(name.into()),
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
}

#[sealed::sealed]
impl<Tr: ?Sized, S: ?Sized, T: ?Sized> NetworkFor<T> for NetworkingConfig<Tr, S>
where
    Tr: TransportKind,
    S: SerKind<T>,
{
    fn serialize_thunk(is_demux: bool) -> syn::Expr {
        S::serialize_thunk(is_demux)
    }

    fn deserialize_thunk(tagged: Option<&syn::Type>) -> syn::Expr {
        S::deserialize_thunk(tagged)
    }

    fn name(&self) -> Option<&str> {
        None
    }
}

#[sealed::sealed]
impl<Tr: ?Sized, S: ?Sized, T: ?Sized> NetworkFor<T> for NetworkingConfig<Tr, S, String>
where
    Tr: TransportKind,
    S: SerKind<T>,
{
    fn serialize_thunk(is_demux: bool) -> syn::Expr {
        S::serialize_thunk(is_demux)
    }

    fn deserialize_thunk(tagged: Option<&syn::Type>) -> syn::Expr {
        S::deserialize_thunk(tagged)
    }

    fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }
}

/// A network channel that uses length-delimited TCP for transport.
pub const TCP: NetworkingConfig<Tcp<()>, NoSer> = NetworkingConfig {
    name: None,
    _phantom: (PhantomData, PhantomData),
};
