//! DFIR's inner (intra-subgraph) compiled layer.
//!
//! The compiled layer mainly consists of [`Iterator`]s and [`Sink`](futures::sink::Sink)s
//!
//! This module contains some extra helpers and adaptors for use with them.
pub mod pull;
pub mod push;
