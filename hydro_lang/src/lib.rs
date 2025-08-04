#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(nightly, feature(file_lock))]

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
pub mod runtime_context;
#[cfg(feature = "dfir_context")]
#[cfg_attr(docsrs, doc(cfg(feature = "dfir_context")))]
pub use runtime_context::RUNTIME_CONTEXT;

pub mod boundedness;
pub use boundedness::{Bounded, Unbounded};

pub mod stream;
pub use stream::{NoOrder, Stream, TotalOrder};

pub mod singleton;
pub use singleton::Singleton;

pub mod optional;
pub use optional::Optional;

pub mod location;
pub use location::cluster::CLUSTER_SELF_ID;
pub use location::{Atomic, Cluster, ClusterId, External, Location, NetworkHint, Process, Tick};

#[cfg(feature = "build")]
#[cfg_attr(docsrs, doc(cfg(feature = "build")))]
pub mod deploy;

pub mod deploy_runtime;

pub mod cycle;

pub mod builder;
pub use builder::FlowBuilder;

mod manual_expr;

pub mod ir;

#[cfg(feature = "viz")]
pub mod graph;

#[cfg(feature = "viz")]
#[cfg_attr(docsrs, doc(cfg(feature = "viz")))]
pub mod graph_util;

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
