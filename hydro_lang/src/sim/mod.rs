//! Deterministic simulation testing support for Hydro programs.
//!
//! See [`crate::compile::builder::FlowBuilder::sim`] and [`flow::SimFlow`] for more details.

use std::marker::PhantomData;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::live_collections::stream::{Ordering, Retries};

/// A receiver for an external bincode stream in a simulation.
pub struct SimReceiver<T: Serialize + DeserializeOwned, O: Ordering, R: Retries>(
    pub(crate) usize,
    pub(crate) PhantomData<(T, O, R)>,
);

/// A sender to an external bincode sink in a simulation.
pub struct SimSender<T: Serialize + DeserializeOwned, O: Ordering, R: Retries>(
    pub(crate) usize,
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
#[doc(hidden)]
pub mod runtime;

#[cfg(test)]
mod tests;
