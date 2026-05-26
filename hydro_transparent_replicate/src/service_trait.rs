//! The [`ReplicableService`] trait definition.
//!
//! This module defines the core abstraction that users implement to make their
//! service transparently replicated. The trait captures the four operations
//! needed by the primary/backup protocol: command application, read-only
//! classification, snapshotting, and state restoration.

use serde::{Serialize, de::DeserializeOwned};
use std::fmt::Debug;

/// A service that can be transparently replicated via primary/backup replication.
///
/// Implementors wrap a replication-oblivious service (e.g., a key-value store,
/// a SQL database, or any stateful computation) and expose it through this trait.
/// The replication layer then handles sequencing, quorum acknowledgment, view
/// changes, and state transfer — all invisible to the service implementation.
///
/// # Observational Determinism Requirement
///
/// The [`apply`](ReplicableService::apply) method **must be observationally
/// deterministic**: given the same sequence of commands starting from the same
/// initial state (or a state restored via [`restore`](ReplicableService::restore)),
/// it must produce the same sequence of responses on every replica.
///
/// This is the fundamental correctness requirement that enables replication.
/// If two replicas apply the same command sequence, they must arrive at
/// equivalent states and produce identical responses. Violations of this
/// property will cause replicas to diverge silently.
///
/// **What to avoid in `apply` implementations:**
/// - Non-deterministic functions (e.g., `random()`, `datetime('now')`)
/// - Dependency on thread-local or global mutable state external to the service
/// - Floating-point operations that may differ across platforms (rare in practice)
/// - File I/O or network calls whose results may vary between replicas
///
/// # Trait Bounds
///
/// - `Send + 'static`: Required because the service instance lives inside a Hydro
///   dataflow operator closure that may be moved across threads.
/// - `Default`: Required because the replication protocol creates fresh service
///   instances inside dataflow operators (e.g., for the scan that applies committed
///   commands). The `Default` instance represents an empty/initial service state
///   before any commands have been applied or any snapshot has been restored.
///
/// # Example
///
/// ```rust,ignore
/// use hydro_transparent_replicate::r#trait::ReplicableService;
/// use serde::{Serialize, Deserialize};
/// use std::collections::HashMap;
///
/// #[derive(Serialize, Deserialize, Clone, Debug)]
/// enum KvCommand {
///     Put { key: String, value: String },
///     Get { key: String },
/// }
///
/// #[derive(Serialize, Deserialize, Clone, Debug)]
/// enum KvResponse {
///     Ok,
///     Value(Option<String>),
/// }
///
/// struct KvService {
///     store: HashMap<String, String>,
/// }
///
/// impl ReplicableService for KvService {
///     type Command = KvCommand;
///     type Response = KvResponse;
///
///     fn apply(&mut self, command: KvCommand) -> KvResponse {
///         match command {
///             KvCommand::Put { key, value } => {
///                 self.store.insert(key, value);
///                 KvResponse::Ok
///             }
///             KvCommand::Get { key } => {
///                 KvResponse::Value(self.store.get(&key).cloned())
///             }
///         }
///     }
///
///     fn is_read_only(command: &KvCommand) -> bool {
///         matches!(command, KvCommand::Get { .. })
///     }
///
///     fn snapshot(&self) -> Vec<u8> {
///         bincode::serialize(&self.store).unwrap()
///     }
///
///     fn restore(&mut self, data: &[u8]) {
///         self.store = bincode::deserialize(data).unwrap();
///     }
/// }
/// ```
pub trait ReplicableService: Send + Default + 'static {
    /// The command type sent by clients and replicated across the cluster.
    ///
    /// Commands represent operations to be applied to the service. They are
    /// serialized for network transport between replicas, so they must implement
    /// [`Serialize`] and [`DeserializeOwned`]. The [`Clone`] bound is needed
    /// because commands may be stored in logs and retransmitted during state
    /// transfer.
    type Command: Serialize + DeserializeOwned + Clone + Debug + Send;

    /// The response type returned to clients after a command is committed and applied.
    ///
    /// Responses are serialized for network transport back to the client.
    /// The [`Clone`] bound allows the protocol layer to retain responses for
    /// deduplication or retransmission.
    type Response: Serialize + DeserializeOwned + Clone + Debug + Send;

    /// Apply a command to the service, returning a response.
    ///
    /// This is the core state-transition function of the service. The replication
    /// layer calls this method on the primary (and optionally on backups) after
    /// a command has been committed by the quorum.
    ///
    /// # Observational Determinism
    ///
    /// This method **must** be deterministic: the same command applied to the
    /// same state must always produce the same response and the same resulting
    /// state. This invariant is critical for replica consistency — if violated,
    /// replicas will silently diverge.
    ///
    /// Determinism is required over the *observable* behavior (responses and
    /// subsequent snapshots), not internal representation details like memory
    /// layout or hash map iteration order (as long as those don't leak into
    /// responses or snapshots).
    fn apply(&mut self, command: Self::Command) -> Self::Response;

    /// Returns `true` if the given command is read-only (has no side effects).
    ///
    /// Read-only commands skip the replication protocol entirely: they are
    /// applied directly on the primary and the response is returned immediately
    /// without broadcasting to backups or waiting for quorum acknowledgment.
    ///
    /// This is a static classification — it depends only on the command value,
    /// not on the current service state. Misclassifying a mutating command as
    /// read-only will cause replica divergence.
    fn is_read_only(command: &Self::Command) -> bool;

    /// Serialize the entire service state to a byte vector.
    ///
    /// Used during state transfer when a new primary is elected after a view
    /// change. A surviving backup calls `snapshot()` to capture its current
    /// state, which is then sent to the new primary for restoration.
    ///
    /// The snapshot must capture all state necessary to reconstruct the service
    /// such that subsequent `apply` calls produce the same results as they
    /// would on the original instance.
    ///
    /// # Consistency
    ///
    /// The snapshot must be a consistent point-in-time capture. If the service
    /// uses multiple internal data structures, they must all reflect the same
    /// logical state (i.e., the state after applying commands up to some
    /// sequence number).
    fn snapshot(&self) -> Vec<u8>;

    /// Restore service state from a previously-taken snapshot.
    ///
    /// Called on the new primary during state transfer after a view change.
    /// After `restore` completes, the service must be in a state equivalent
    /// to the one that produced the snapshot — meaning subsequent `apply` calls
    /// on the same command sequence will produce identical responses.
    ///
    /// # Panics
    ///
    /// Implementations may panic if `data` is not a valid snapshot (i.e., was
    /// not produced by a prior call to [`snapshot`](ReplicableService::snapshot)
    /// on the same service type). The protocol layer handles this by retrying
    /// state transfer from another backup.
    fn restore(&mut self, data: &[u8]);
}
