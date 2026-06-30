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

/// A sender for an atomic input in a simulation.
///
/// `send_atomic` is synchronous — the value is immediately available in
/// the next atomic slice without requiring a separate tick.
pub struct SimAtomicSender<T: Serialize + DeserializeOwned, O: Ordering, R: Retries>(
    pub(crate) SimSender<T, O, R>,
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

#[cfg(test)]
mod tests;
