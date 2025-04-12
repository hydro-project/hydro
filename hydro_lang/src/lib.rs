#![cfg(nightly)]
#![feature(file_lock)]

#![cfg_attr(docsrs, feature(doc_cfg))]

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
pub use location::{Atomic, Cluster, ClusterId, ExternalProcess, Location, Process, Tick};

#[cfg(feature = "build")]
#[cfg_attr(docsrs, doc(cfg(feature = "build")))]
pub mod deploy;

pub mod deploy_runtime;

pub mod cycle;

pub mod builder;
pub use builder::FlowBuilder;

pub mod ir;

pub mod rewrites;

mod staging_util;

#[cfg(feature = "deploy")]
#[cfg_attr(docsrs, doc(cfg(feature = "build")))]
pub mod test_util;

#[cfg(test)]
mod test_init {
    #[ctor::ctor]
    fn init() {
        crate::deploy::init_test();
    }
}
