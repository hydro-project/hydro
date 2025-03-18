#![cfg_attr(docsrs, feature(doc_cfg))]

stageleft::stageleft_no_entry_crate!();

pub use dfir_rs;
pub use stageleft::q;

#[doc(hidden)]
pub mod runtime_support {
    pub use {bincode, stageleft, tokio};
    pub mod resource_measurement;
}

pub mod runtime_context;
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
