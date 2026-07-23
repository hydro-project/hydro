//! Deterministic simulation testing support for Hydro programs.
//!
//! See [`crate::compile::builder::FlowBuilder::sim`] and [`crate::sim::flow::SimFlow`] for more details.

use std::marker::PhantomData;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::compile::builder::ExternalPortId;
use crate::live_collections::stream::{Ordering, Retries};

/// A receiver for an external bincode stream in a simulation.
pub struct SimReceiver<T: Serialize + DeserializeOwned, O: Ordering, R: Retries>(
    pub(crate) ExternalPortId,
    pub(crate) PhantomData<(T, O, R)>,
);

/// A sender to an external bincode sink in a simulation.
pub struct SimSender<T: Serialize + DeserializeOwned, O: Ordering, R: Retries>(
    pub(crate) ExternalPortId,
    pub(crate) PhantomData<(T, O, R)>,
);

/// A receiver for an external cluster stream in a simulation.
///
/// Each received value is a `(u32, T)` tuple where the `u32` is the raw
/// cluster member ID that produced the value.
pub struct SimClusterReceiver<T: Serialize + DeserializeOwned, O: Ordering, R: Retries>(
    pub(crate) ExternalPortId,
    pub(crate) PhantomData<(T, O, R)>,
);

/// A sender to an external cluster sink in a simulation.
///
/// Each sent value is a `(u32, T)` tuple where the `u32` is the raw
/// cluster member ID that should receive the value.
pub struct SimClusterSender<T: Serialize + DeserializeOwned, O: Ordering, R: Retries>(
    pub(crate) ExternalPortId,
    pub(crate) PhantomData<(T, O, R)>,
);

#[cfg(stageleft_runtime)]
mod builder;

#[cfg(stageleft_runtime)]
pub mod compiled;

#[cfg(stageleft_runtime)]
pub(crate) mod graph;

#[cfg(stageleft_runtime)]
pub mod flow;

#[cfg(stageleft_runtime)]
pub(crate) mod versioned_network;

#[cfg(stageleft_runtime)]
#[doc(hidden)]
pub mod runtime;

#[cfg(stageleft_runtime)]
#[doc(hidden)]
pub use compiled::assume_impl;

/// Assumes that a condition holds in the current simulation instance, discarding the instance
/// if the condition is false.
///
/// This is useful inside simulation tests ([`crate::sim::flow::SimFlow::fuzz`],
/// [`crate::sim::flow::SimFlow::exhaustive`], and the corresponding
/// [`crate::sim::compiled::CompiledSim`] APIs) to restrict exploration to executions that
/// satisfy some precondition, similar to `prop_assume!` in proptest. When the condition is
/// false, the current instance is stopped and discarded: it is **not** treated as a test
/// failure (and will never be recorded as a fuzzing reproducer), and the fuzzer / exhaustive
/// search simply moves on to the next instance. If logging is enabled (always during replays,
/// or when `HYDRO_SIM_LOG=1`), the failed assumption is logged.
///
/// Like the standard `assert!` macro, an optional custom message with format arguments can be
/// provided.
///
/// ```rust,ignore
/// flow.sim().fuzz(async || {
///     in_send.send_many([1, 2]);
///     let all: Vec<u32> = out_recv.collect().await;
///     hydro_lang::sim::assume!(all.len() == 2, "expected both values in one batch, got {:?}", all);
///     // ... assertions that only make sense when the assumption holds ...
/// });
/// ```
#[doc(hidden)]
#[macro_export]
macro_rules! assume {
    ($cond:expr $(,)?) => {
        $crate::sim::assume_impl($cond, ::core::format_args!("{}", ::core::stringify!($cond)))
    };
    ($cond:expr, $($arg:tt)+) => {
        $crate::sim::assume_impl($cond, ::core::format_args!($($arg)+))
    };
}

#[doc(inline)]
pub use crate::assume;

#[cfg(test)]
mod tests;
