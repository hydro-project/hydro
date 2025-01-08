stageleft::stageleft_no_entry_crate!();

pub use dfir_rs;
pub use stageleft::q;

#[doc(hidden)]
pub mod runtime_support {
    pub use bincode;
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
pub use location::{Cluster, ClusterId, ExternalProcess, Location, Process, Tick, Timestamped};

#[cfg(feature = "build")]
pub mod deploy;

pub mod deploy_runtime;

pub mod cycle;

pub mod builder;
pub use builder::FlowBuilder;

pub mod ir;

pub mod rewrites;

mod staging_util;

#[cfg(feature = "deploy")]
pub mod test_util;

#[stageleft::runtime]
#[cfg(test)]
mod tests {
    #[ctor::ctor]
    fn init() {
        crate::deploy::init_test();
    }
}
