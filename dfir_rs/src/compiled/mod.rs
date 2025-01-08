//! Hydroflow's inner (intra-subgraph) compiled layer.
//!
//! The compiled layer mainly consists of [`Iterator`]s (from standard Rust)
//! and [`Pusherator`](::pusherator::Pusherator)s (from the [`pusherator`] crate). This module
//! contains some extra helpers and adaptors for Hydroflow to use with them.
pub mod pull;
