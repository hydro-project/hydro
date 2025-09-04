#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]

//! Hydro is a high-level distributed programming framework for Rust.
//! Hydro can help you quickly write scalable distributed services that are correct by construction.
//! Much like Rust helps with memory safety, Hydro helps with [distributed safety](https://hydro.run/docs/hydro/correctness).
//!
//! The core Hydro API involves [live collections](https://hydro.run/docs/hydro/live-collections/), which represent asynchronously
//! updated sources of data such as incoming network requests and application state. The most common live collection is
//! [`live_collections::stream::Stream`]; other live collections can be found in [`live_collections`].
//!
//! Hydro uses a unique compilation approach where you define deployment logic as Rust code alongside your distributed system implementation.
//! For more details on this API, see the [Hydro docs](https://hydro.run/docs/hydro/deploy/) and the [`deploy`] module.

stageleft::stageleft_no_entry_crate!();

#[cfg(feature = "runtime_support")]
#[cfg_attr(docsrs, doc(cfg(feature = "runtime_support")))]
#[doc(hidden)]
pub mod runtime_support {
    pub use {bincode, dfir_rs, stageleft, tokio};
    pub mod resource_measurement;
}

#[doc(hidden)]
pub mod internal_constants {
    pub const CPU_USAGE_PREFIX: &str = "CPU:";
    // Should remain consistent with dfir_lang/src/graph/ops/_counter.rs
    pub const COUNTER_PREFIX: &str = "_counter";
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

    pub use crate::boundedness::{Bounded, Unbounded};
    pub use crate::builder::FlowBuilder;
    pub use crate::live_collections::keyed_singleton::KeyedSingleton;
    pub use crate::live_collections::keyed_stream::KeyedStream;
    pub use crate::live_collections::optional::Optional;
    pub use crate::live_collections::singleton::Singleton;
    pub use crate::live_collections::stream::Stream;
    pub use crate::location::{Cluster, External, Location as _, Process, Tick};
    pub use crate::nondet::{NonDet, nondet};
}

#[cfg(feature = "dfir_context")]
#[cfg_attr(docsrs, doc(cfg(feature = "dfir_context")))]
#[expect(missing_docs, reason = "TODO")]
pub mod runtime_context;

#[expect(missing_docs, reason = "TODO")]
pub mod nondet;

#[expect(missing_docs, reason = "TODO")]
pub mod boundedness;

#[expect(missing_docs, reason = "TODO")]
pub mod live_collections;

#[expect(missing_docs, reason = "TODO")]
pub mod location;

#[expect(missing_docs, reason = "TODO")]
#[cfg(any(
    feature = "deploy",
    feature = "deploy_integration" // hidden internal feature enabled in the trybuild
))]
#[cfg_attr(docsrs, doc(cfg(feature = "deploy")))]
pub mod deploy;

#[expect(missing_docs, reason = "TODO")]
pub mod cycle;

#[expect(missing_docs, reason = "TODO")]
pub mod builder;

mod manual_expr;

#[expect(missing_docs, reason = "TODO")]
pub mod ir;

#[cfg(feature = "viz")]
#[expect(missing_docs, reason = "TODO")]
pub mod graph;

#[expect(missing_docs, reason = "TODO")]
pub mod rewrites;

mod staging_util;

pub mod backtrace;

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

#[cfg(test)]
mod test_init {
    #[ctor::ctor]
    fn init() {
        crate::deploy::init_test();
    }
}
