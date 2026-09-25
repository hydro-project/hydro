//! Serialization codecs for simulation inputs and outputs.

use serde::Serialize;
use serde::de::DeserializeOwned;
#[cfg(stageleft_runtime)]
use stageleft::quote_type;
#[cfg(stageleft_runtime)]
use syn::parse_quote;

#[cfg(stageleft_runtime)]
use crate::staging_util::get_this_crate;

/// A serialization codec for values crossing the simulation dylib boundary.
///
/// The test and simulation dylib exchange only encoded bytes, so custom codecs do not need to
/// use [`serde`]. See [`BincodeCodec`] for the default.
///
/// The codec is selected purely at the type level, as a type parameter of
/// [`Stream::sim_output_with`], [`Location::sim_input_with`] and their cluster
/// counterparts (e.g. `stream.sim_output_with::<MyCodec>()`). No codec value is ever
/// constructed, so a codec is typically a unit struct, but any type will do.
///
/// [`Stream::sim_output_with`]: crate::prelude::Stream::sim_output_with
/// [`Location::sim_input_with`]: crate::location::Location::sim_input_with
///
/// # Defining a custom codec
///
/// Generated dylib code refers to the codec by its definition path. The codec must therefore:
///
/// * be public through to the crate root, and
/// * live outside `#[cfg(test)]` and integration-test targets.
///
/// Its serialization library and `hydro_lang` with the `sim` feature must be regular
/// dependencies, not dev-dependencies.
///
/// ```
/// use hydro_lang::sim::codec::SimCodec;
///
/// pub struct Message(u32);
///
/// pub struct MessageCodec;
///
/// impl SimCodec<Message> for MessageCodec {
///     fn encode(value: &Message) -> Vec<u8> {
///         value.0.to_le_bytes().to_vec()
///     }
///
///     fn decode(bytes: &[u8]) -> Message {
///         Message(u32::from_le_bytes(bytes.try_into().unwrap()))
///     }
/// }
/// ```
///
/// # Codecs from a test-only crate
///
/// A codec may instead live in a separate crate the tests depend on, keeping `hydro_lang/sim`
/// out of the tested crate's regular dependencies. Generated dylib code cannot yet name a
/// dev-dependency ([stageleft#104]), so also declare the crate as an optional regular
/// dependency, and name a feature on the dev entry so the simulator's generated crate enables it:
///
/// ```toml
/// [dependencies]
/// my_codecs = { version = "...", optional = true }
///
/// [dev-dependencies]
/// my_codecs = { version = "...", features = ["default"] }
/// ```
///
/// `my_codecs` must define the named feature; an empty `default = []` suffices.
///
/// [stageleft#104]: https://github.com/hydro-project/stageleft/pull/104
pub trait SimCodec<T> {
    /// Encodes `value` for transport across the simulation dylib boundary.
    fn encode(value: &T) -> Vec<u8>;

    /// Decodes a value received across the simulation dylib boundary.
    fn decode(bytes: &[u8]) -> T;
}

/// The default simulation codec, using [`bincode`].
#[derive(Clone, Copy, Debug, Default)]
pub struct BincodeCodec;

impl<T: Serialize + DeserializeOwned> SimCodec<T> for BincodeCodec {
    fn encode(value: &T) -> Vec<u8> {
        bincode::serialize(value).unwrap()
    }

    fn decode(bytes: &[u8]) -> T {
        bincode::deserialize(bytes).unwrap()
    }
}

#[cfg(stageleft_runtime)]
pub(crate) fn staged_serialize<T, C: SimCodec<T>>() -> syn::Expr {
    let root = get_this_crate();
    let t_type = quote_type::<T>();
    let codec_type = quote_type::<C>();

    parse_quote! {
        #root::runtime_support::stageleft::runtime_support::fn1_type_hint::<#t_type, _>(
            |data| {
                #root::runtime_support::dfir_rs::bytes::Bytes::from(
                    <#codec_type as #root::__staged::sim::codec::SimCodec<#t_type>>::encode(&data)
                )
            }
        )
    }
}

#[cfg(stageleft_runtime)]
pub(crate) fn staged_deserialize<T, C: SimCodec<T>>() -> syn::Expr {
    let root = get_this_crate();
    let t_type = quote_type::<T>();
    let codec_type = quote_type::<C>();

    parse_quote! {
        |res| {
            let bytes = res.unwrap();
            <#codec_type as #root::__staged::sim::codec::SimCodec<#t_type>>::decode(&bytes)
        }
    }
}
