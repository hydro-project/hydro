#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(not(stageleft_trybuild), warn(missing_docs))]

//! Hydro is a high-level distributed programming framework for Rust.
//! Hydro can help you quickly write scalable distributed services that are correct by construction.
//! Much like Rust helps with memory safety, Hydro helps with [distributed safety](https://hydro.run/docs/hydro/reference/correctness).
//!
//! The core Hydro API involves [live collections](https://hydro.run/docs/hydro/reference/live-collections/), which represent asynchronously
//! updated sources of data such as incoming network requests and application state. The most common live collection is
//! [`live_collections::stream::Stream`]; other live collections can be found in [`live_collections`].
//!
//! Hydro uses a unique compilation approach where you define deployment logic as Rust code alongside your distributed system implementation.
//! For more details on this API, see the [Hydro docs](https://hydro.run/docs/hydro/reference/deploy/) and the [`deploy`] module.

stageleft::stageleft_no_entry_crate!();

#[cfg(feature = "runtime_support")]
#[cfg_attr(docsrs, doc(cfg(feature = "runtime_support")))]
#[doc(hidden)]
pub mod runtime_support {
    #[cfg(feature = "sim")]
    pub use colored;
    #[cfg(feature = "deploy_integration")]
    pub use hydro_deploy_integration;
    pub use {bincode, dfir_rs, slotmap, stageleft, tokio};

    #[cfg(any(feature = "deploy_integration", feature = "docker_runtime"))]
    pub mod launch;
}

#[doc(hidden)]
pub mod macro_support {
    pub use copy_span;
}

pub mod prelude {
    // taken from `tokio`
    //! A "prelude" for users of the `hydro_lang` crate.
    //!
    //! This prelude is similar to the standard library's prelude in that you'll almost always want to import its entire contents, but unlike the standard library's prelude you'll have to do so manually:
    //! ```
    //! # #![allow(warnings)]
    //! use hydro_lang::prelude::*;
    //! ```
    //!
    //! The prelude may grow over time as additional items see ubiquitous use.

    pub use stageleft::q;

    pub use crate::compile::builder::FlowBuilder;
    pub use crate::live_collections::boundedness::{Bounded, Unbounded};
    pub use crate::live_collections::keyed_singleton::KeyedSingleton;
    pub use crate::live_collections::keyed_stream::KeyedStream;
    pub use crate::live_collections::optional::Optional;
    pub use crate::live_collections::singleton::Singleton;
    pub use crate::live_collections::sliced::sliced;
    pub use crate::live_collections::stream::Stream;
    pub use crate::location::{Cluster, External, Location as _, Process, Tick};
    pub use crate::networking::TCP;
    pub use crate::nondet::{NonDet, nondet};
    pub use crate::properties::ManualProof;

    /// A macro to set up a Hydro crate.
    #[macro_export]
    macro_rules! setup {
        () => {
            stageleft::stageleft_no_entry_crate!();

            #[cfg(test)]
            mod test_init {
                #[ctor::ctor]
                fn init() {
                    $crate::compile::init_test();
                }
            }
        };
    }
}

#[cfg(feature = "dfir_context")]
#[cfg_attr(docsrs, doc(cfg(feature = "dfir_context")))]
pub mod runtime_context;

pub mod nondet;

pub mod live_collections;

pub mod location;

pub mod networking;

pub mod properties;

pub mod telemetry;

#[cfg(any(
    feature = "deploy",
    feature = "deploy_integration" // hidden internal feature enabled in the trybuild
))]
#[cfg_attr(docsrs, doc(cfg(feature = "deploy")))]
pub mod deploy;

#[cfg(feature = "sim")]
#[cfg_attr(docsrs, doc(cfg(feature = "sim")))]
pub mod sim;

pub mod forward_handle;

pub mod compile;

mod manual_expr;

#[cfg(stageleft_runtime)]
#[cfg(feature = "viz")]
#[cfg_attr(docsrs, doc(cfg(feature = "viz")))]
#[expect(missing_docs, reason = "TODO")]
pub mod viz;

mod staging_util;

#[cfg(feature = "deploy")]
#[cfg_attr(docsrs, doc(cfg(feature = "deploy")))]
pub mod test_util;

#[cfg(feature = "build")]
#[ctor::ctor]
fn init_rewrites() {
    stageleft::add_private_reexport(
        vec!["tokio_util", "codec", "lines_codec"],
        vec!["tokio_util", "codec"],
    );
}

#[cfg(all(test, feature = "trybuild"))]
mod test_init {
    #[ctor::ctor]
    fn init() {
        crate::compile::init_test();
    }
}

/// Creates a newtype wrapper around an integer type.
///
/// Usage:
/// ```rust
/// hydro_lang::newtype_counter! {
///     /// My counter.
///     pub struct MyCounter(u32);
///
///     /// My secret counter.
///     struct SecretCounter(u64);
/// }
/// ```
#[doc(hidden)]
#[macro_export]
macro_rules! newtype_counter {
    (
        $(
            $( #[$attr:meta] )*
            $vis:vis struct $name:ident($typ:ty);
        )*
    ) => {
        $(
            $( #[$attr] )*
            #[repr(transparent)]
            #[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
            $vis struct $name($typ);

            #[allow(clippy::allow_attributes, dead_code, reason = "macro-generated methods may be unused")]
            impl $name {
                /// Gets the current ID and increments for the next.
                pub fn get_and_increment(&mut self) -> Self {
                    let id = self.0;
                    self.0 += 1;
                    Self(id)
                }

                /// Returns an iterator from zero up to (but excluding) `self`.
                ///
                /// This is useful for iterating already-allocated values.
                pub fn range_up_to(&self) -> impl std::iter::DoubleEndedIterator<Item = Self>
                    + std::iter::FusedIterator
                {
                    (0..self.0).map(Self)
                }

                /// Reveals the inner ID.
                pub fn into_inner(self) -> $typ {
                    self.0
                }
            }

            impl std::fmt::Display for $name {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    write!(f, "{}", self.0)
                }
            }

            impl serde::ser::Serialize for $name {
                fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                where
                    S: serde::Serializer
                {
                    serde::ser::Serialize::serialize(&self.0, serializer)
                }
            }

            impl<'de> serde::de::Deserialize<'de> for $name {
                fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                where
                    D: serde::Deserializer<'de>
                {
                    serde::de::Deserialize::deserialize(deserializer).map(Self)
                }
            }
        )*
    };
}
