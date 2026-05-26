//! fjall backend implementation of `ReplicableService`.
//!
//! This module provides a key-value store backed by [fjall](https://docs.rs/fjall),
//! an LSM-based embeddable key-value storage engine with cross-partition snapshots.
//!
//! # Observational Determinism
//!
//! All operations (insert, get, remove) are deterministic given the same command
//! sequence from the same initial state. fjall does not expose any
//! non-deterministic functions in its key-value API, so all `FjallCommand`
//! variants are safe for replication.

use crate::ReplicableService;
use serde::{Deserialize, Serialize};

use std::sync::Arc;

/// A replicated key-value service backed by fjall.
///
/// Uses a single keyspace with one partition named "data". The `Default`
/// implementation creates a temporary keyspace suitable for testing.
pub struct FjallService {
    /// Kept alive so the partition handle remains valid.
    #[allow(dead_code)]
    keyspace: Arc<fjall::Keyspace>,
    partition: fjall::PartitionHandle,
    /// We hold onto the tempdir so it doesn't get dropped (and deleted) prematurely.
    _tempdir: Option<Arc<tempfile::TempDir>>,
}

/// Commands that can be applied to the fjall service.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum FjallCommand {
    /// Insert or update a key-value pair.
    Insert { key: Vec<u8>, value: Vec<u8> },
    /// Read the value associated with a key.
    Get { key: Vec<u8> },
    /// Remove a key-value pair. Returns whether the key existed.
    Remove { key: Vec<u8> },
}

/// Responses from the fjall service.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum FjallResponse {
    /// Acknowledgment for a successful insert operation.
    Ok,
    /// The value associated with a key (None if key does not exist).
    Value(Option<Vec<u8>>),
    /// Whether a key was removed (true if it existed, false otherwise).
    Removed(bool),
}

impl FjallService {
    /// Create a new `FjallService` with the given keyspace and partition.
    pub fn new(keyspace: fjall::Keyspace, partition: fjall::PartitionHandle) -> Self {
        Self {
            keyspace: Arc::new(keyspace),
            partition,
            _tempdir: None,
        }
    }
}

impl Default for FjallService {
    fn default() -> Self {
        let tempdir = tempfile::tempdir().expect("failed to create temp dir for fjall");
        let keyspace = fjall::Config::new(tempdir.path())
            .open()
            .expect("failed to open fjall keyspace");
        let partition = keyspace
            .open_partition("data", fjall::PartitionCreateOptions::default())
            .expect("failed to open fjall partition");
        Self {
            keyspace: Arc::new(keyspace),
            partition,
            _tempdir: Some(Arc::new(tempdir)),
        }
    }
}

impl ReplicableService for FjallService {
    type Command = FjallCommand;
    type Response = FjallResponse;

    fn apply(&mut self, command: Self::Command) -> Self::Response {
        match command {
            FjallCommand::Insert { key, value } => {
                self.partition
                    .insert(key, value)
                    .expect("fjall insert failed");
                FjallResponse::Ok
            }
            FjallCommand::Get { key } => {
                let value = self
                    .partition
                    .get(key)
                    .expect("fjall get failed")
                    .map(|slice| slice.to_vec());
                FjallResponse::Value(value)
            }
            FjallCommand::Remove { key } => {
                // Check if key exists before removing
                let existed = self
                    .partition
                    .get(&key)
                    .expect("fjall get failed")
                    .is_some();
                self.partition.remove(&key).expect("fjall remove failed");
                FjallResponse::Removed(existed)
            }
        }
    }

    fn is_read_only(command: &Self::Command) -> bool {
        matches!(command, FjallCommand::Get { .. })
    }

    fn snapshot(&self) -> Vec<u8> {
        let pairs: Vec<(Vec<u8>, Vec<u8>)> = self
            .partition
            .iter()
            .map(|entry| {
                let entry = entry.expect("fjall iter entry failed");
                (entry.0.to_vec(), entry.1.to_vec())
            })
            .collect();
        bincode::serialize(&pairs).expect("serialize failed")
    }

    fn restore(&mut self, data: &[u8]) {
        let pairs: Vec<(Vec<u8>, Vec<u8>)> =
            bincode::deserialize(data).expect("deserialize failed");

        // Clear existing data by removing all keys
        let keys: Vec<Vec<u8>> = self
            .partition
            .iter()
            .map(|entry| {
                let entry = entry.expect("fjall iter entry failed");
                entry.0.to_vec()
            })
            .collect();
        for key in keys {
            self.partition.remove(key).expect("fjall remove failed");
        }

        // Re-insert all pairs from the snapshot
        for (key, value) in pairs {
            self.partition
                .insert(key, value)
                .expect("fjall insert failed");
        }
    }
}
