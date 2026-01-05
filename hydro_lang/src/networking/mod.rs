//! Types for configuring network channels with serialization formats, transports, etc.

use std::marker::PhantomData;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::live_collections::stream::networking::{deserialize_bincode, serialize_bincode};

trait SerKind<T: ?Sized> {
    fn serialize_thunk(is_demux: bool) -> syn::Expr;

    fn deserialize_thunk(tagged: Option<&syn::Type>) -> syn::Expr;
}

/// Serialize items using the [`bincode`] crate.
pub enum Bincode {}
impl<T: ?Sized + Serialize + DeserializeOwned> SerKind<T> for Bincode {
    fn serialize_thunk(is_demux: bool) -> syn::Expr {
        serialize_bincode::<T>(is_demux)
    }

    fn deserialize_thunk(tagged: Option<&syn::Type>) -> syn::Expr {
        deserialize_bincode::<T>(tagged)
    }
}

/// A networking backend implementation that supports items of type `T`.
pub trait NetworkFor<T: ?Sized> {
    /// Generates serialization logic for sending `T`.
    fn serialize_thunk(is_demux: bool) -> syn::Expr;

    /// Generates deserialization logic for receiving `T`.
    fn deserialize_thunk(tagged: Option<&syn::Type>) -> syn::Expr;
}

/// A network channel configuration with `S` as the serialization backend.
pub struct NetworkingConfig<S: ?Sized> {
    _phantom: PhantomData<S>,
}

impl<S: ?Sized, T: ?Sized> NetworkFor<T> for NetworkingConfig<S>
where
    S: SerKind<T>,
{
    fn serialize_thunk(is_demux: bool) -> syn::Expr {
        S::serialize_thunk(is_demux)
    }

    fn deserialize_thunk(tagged: Option<&syn::Type>) -> syn::Expr {
        S::deserialize_thunk(tagged)
    }
}

/// A network channel that uses [`bincode`] for serialization.
pub const BINCODE: NetworkingConfig<Bincode> = NetworkingConfig {
    _phantom: PhantomData,
};
