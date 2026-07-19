//! Reusable, protocol-agnostic dataflow combinators.
//!
//! These are the pieces of the key-value store that aren't specific to keys,
//! values, or the Put/Get protocol — they are generic distributed-systems
//! patterns that happen to be useful here. Because Hydro programs the whole
//! system as one piece, we can factor these out as standalone, independently
//! tested building blocks rather than splitting the logic along network
//! boundaries. Each has its own inline simulation tests.

pub mod atomic_store;
pub mod collect_quorum_responses;
pub mod hrw_scatter;

pub use atomic_store::atomic_store;
pub use collect_quorum_responses::collect_quorum_responses;
pub use hrw_scatter::hrw_scatter;
