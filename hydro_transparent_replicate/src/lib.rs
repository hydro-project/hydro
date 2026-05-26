//! # hydro_transparent_replicate
//!
//! A transparent replication crate that wraps any `ReplicableService` in a
//! fault-tolerant primary/backup replication layer. The user implements the
//! `ReplicableService` trait (four methods, two associated types), and the crate
//! provides consensus-free data path replication with Paxos-backed view changes.

#[cfg(stageleft_runtime)]
hydro_lang::setup!();

pub mod service_trait;
pub mod messages;
pub mod config;
pub mod protocol;
pub mod client;
pub mod applier;

/// Marker type for the redb state machine cluster — one per replica host, co-located with the replica.
/// Always defined (not feature-gated) so it can be used as a location type in Hydro pipelines
/// regardless of which feature is active at the call site.
pub struct ReplicaDb;

/// Marker type for the coordinator process in the EC2 demo.
pub struct Coordinator;

#[cfg(any(
    feature = "backend_redb",
    feature = "backend_sled",
    feature = "backend_fjall",
    feature = "backend_rusqlite"
))]
pub mod backends;

// Re-exports
pub use service_trait::ReplicableService;
pub use config::ReplicateConfig;
pub use messages::TransparentReplica;
pub use messages::View;
pub use messages::{Ack, Replicate, CommitNotification, StateTransferRequest, StateTransferResponse, ClientRequest, ClientResponse};
pub use client::{ReplicatedClient, ClientError};
pub use hydro_lang::location::MemberId;
