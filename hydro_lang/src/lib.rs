#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]

//! Hydro is a high-level distributed programming framework for Rust.
//! Hydro can help you quickly write scalable distributed services that are correct by construction.
//! Much like Rust helps with memory safety, Hydro helps with [distributed safety](https://hydro.run/docs/hydro/correctness).
//!
//! The core Hydro API involves [live collections](https://hydro.run/docs/hydro/live-collections/), which represent asynchronously
//! updated sources of data such as incoming network requests and application state. The most common live collection is a [`Stream`];
//! other live collections can be found in modules inside this crate.
//!
//! Hydro uses a unique compilation approach where you define deployment logic as Rust code alongside your distributed system implementation.
//! For more details on this API, see the [Hydro docs](https://hydro.run/docs/hydro/deploy/) and the [`deploy`] module.

stageleft::stageleft_no_entry_crate!();

pub use stageleft::q;

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

#[cfg(feature = "dfir_context")]
#[cfg_attr(docsrs, doc(cfg(feature = "dfir_context")))]
#[expect(missing_docs, reason = "TODO")]
pub mod runtime_context;
#[cfg(feature = "dfir_context")]
#[cfg_attr(docsrs, doc(cfg(feature = "dfir_context")))]
pub use runtime_context::RUNTIME_CONTEXT;

#[expect(missing_docs, reason = "TODO")]
pub mod unsafety;
pub use unsafety::*;

#[expect(missing_docs, reason = "TODO")]
pub mod boundedness;
pub use boundedness::{Bounded, Unbounded};

#[expect(missing_docs, reason = "TODO")]
pub mod stream;
pub use stream::{NoOrder, Stream, TotalOrder};

#[expect(missing_docs, reason = "TODO")]
pub mod keyed_singleton;
#[expect(missing_docs, reason = "TODO")]
pub mod keyed_stream;
pub use keyed_stream::KeyedStream;

#[expect(missing_docs, reason = "TODO")]
pub mod singleton;
pub use singleton::Singleton;

#[expect(missing_docs, reason = "TODO")]
pub mod optional;
pub use optional::Optional;

#[expect(missing_docs, reason = "TODO")]
pub mod location;
pub use location::cluster::CLUSTER_SELF_ID;
pub use location::{Atomic, Cluster, External, Location, MemberId, NetworkHint, Process, Tick};

#[cfg(feature = "build")]
#[cfg_attr(docsrs, doc(cfg(feature = "build")))]
#[expect(missing_docs, reason = "TODO")]
pub mod deploy;

#[expect(missing_docs, reason = "TODO")]
pub mod deploy_runtime;

#[expect(missing_docs, reason = "TODO")]
pub mod cycle;

#[expect(missing_docs, reason = "TODO")]
pub mod builder;
pub use builder::FlowBuilder;

mod manual_expr;

#[expect(missing_docs, reason = "TODO")]
pub mod ir;

#[cfg(feature = "viz")]
#[expect(missing_docs, reason = "TODO")]
pub mod graph;

#[cfg(feature = "viz")]
#[cfg_attr(docsrs, doc(cfg(feature = "viz")))]
#[expect(missing_docs, reason = "TODO")]
pub mod graph_util;

#[expect(missing_docs, reason = "TODO")]
pub mod rewrites;

mod staging_util;

#[expect(missing_docs, reason = "TODO")]
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
