//! DFIR's inner (intra-subgraph) compiled layer.
//!
//! The compiled layer consists of [`dfir_pipes::pull::Pull`]s and [`dfir_pipes::push::Push`]es.
//!
//! This module contains some extra helpers and adaptors for use with them.

pub mod pull;
pub mod push;
