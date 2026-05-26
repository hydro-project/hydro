//! sled backend implementation of `ReplicableService`.
//!
//! This module provides a key-value store backed by [sled](https://docs.rs/sled),
//! an embedded database with lock-free concurrent B+ tree internals.
//!
//! # Observational Determinism
//!
//! All operations (insert, get, remove) are deterministic given the same command
//! sequence from the same initial state. sled does not expose any
//! non-deterministic functions in its key-value API, so all `SledCommand`
//! variants are safe for replication.

use crate::ReplicableService;
use serde::{Deserialize, Serialize};

/// A replicated key-value service backed by sled.
///
/// Uses the default tree of a sled database. The `Default` implementation
/// creates a temporary (non-persisted) database suitable for testing.
pub struct SledService {
    db: sled::Db,
}

/// Commands that can be applied to the sled service.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum SledCommand {
    /// Insert or update a key-value pair.
    Insert { key: Vec<u8>, value: Vec<u8> },
    /// Read the value associated with a key.
    Get { key: Vec<u8> },
    /// Remove a key-value pair. Returns the old value if it existed.
    Remove { key: Vec<u8> },
}

/// Responses from the sled service.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum SledResponse {
    /// Acknowledgment for a successful insert operation.
    Ok,
    /// The value associated with a key (None if key does not exist).
    Value(Option<Vec<u8>>),
    /// The previous value that was removed (None if key did not exist).
    Removed(Option<Vec<u8>>),
}

impl SledService {
    /// Create a new `SledService` with the given sled database.
    pub fn new(db: sled::Db) -> Self {
        Self { db }
    }
}

impl Default for SledService {
    fn default() -> Self {
        let db = sled::Config::new()
            .temporary(true)
            .open()
            .expect("failed to open temporary sled database");
        Self { db }
    }
}

impl ReplicableService for SledService {
    type Command = SledCommand;
    type Response = SledResponse;

    fn apply(&mut self, command: Self::Command) -> Self::Response {
        match command {
            SledCommand::Insert { key, value } => {
                self.db.insert(key, value).expect("sled insert failed");
                SledResponse::Ok
            }
            SledCommand::Get { key } => {
                let value = self
                    .db
                    .get(key)
                    .expect("sled get failed")
                    .map(|ivec| ivec.to_vec());
                SledResponse::Value(value)
            }
            SledCommand::Remove { key } => {
                let old = self
                    .db
                    .remove(key)
                    .expect("sled remove failed")
                    .map(|ivec| ivec.to_vec());
                SledResponse::Removed(old)
            }
        }
    }

    fn is_read_only(command: &Self::Command) -> bool {
        matches!(command, SledCommand::Get { .. })
    }

    fn snapshot(&self) -> Vec<u8> {
        let pairs: Vec<(Vec<u8>, Vec<u8>)> = self
            .db
            .iter()
            .map(|entry| {
                let entry = entry.expect("sled iter entry failed");
                (entry.0.to_vec(), entry.1.to_vec())
            })
            .collect();
        bincode::serialize(&pairs).expect("serialize failed")
    }

    fn restore(&mut self, data: &[u8]) {
        let pairs: Vec<(Vec<u8>, Vec<u8>)> =
            bincode::deserialize(data).expect("deserialize failed");

        // Clear existing data
        self.db.clear().expect("sled clear failed");

        // Re-insert all pairs from the snapshot
        for (key, value) in pairs {
            self.db.insert(key, value).expect("sled insert failed");
        }
    }
}
