//! redb backend implementation of `ReplicableService`.
//!
//! This module provides a key-value store backed by [redb](https://docs.rs/redb),
//! an embedded B-tree database with ACID transactions and copy-on-write semantics.
//!
//! # Observational Determinism
//!
//! All operations (put, get, delete) are deterministic given the same command
//! sequence from the same initial state. redb does not expose any
//! non-deterministic functions in its API, so all `RedbCommand` variants are
//! safe for replication.

use crate::ReplicableService;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use redb::{Database, ReadableTable, TableDefinition, TableError};

/// The table definition for the generic byte-key/byte-value store.
const TABLE: TableDefinition<'static, &[u8], &[u8]> = TableDefinition::new("kv");

/// A replicated key-value service backed by redb.
///
/// Uses a single table with byte keys and byte values. The database is stored
/// on disk (a temporary file is used for the `Default` implementation).
pub struct RedbService {
    db: Arc<Database>,
}

/// Commands that can be applied to the redb service.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum RedbCommand {
    /// Insert or update a key-value pair.
    Put { key: Vec<u8>, value: Vec<u8> },
    /// Read the value associated with a key.
    Get { key: Vec<u8> },
    /// Delete a key-value pair. Returns whether the key existed.
    Delete { key: Vec<u8> },
}

/// Responses from the redb service.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum RedbResponse {
    /// Acknowledgment for a successful put operation.
    Ok,
    /// The value associated with a key (None if key does not exist).
    Value(Option<Vec<u8>>),
    /// Whether a key was deleted (true if it existed, false otherwise).
    Deleted(bool),
}

impl RedbService {
    /// Create a new `RedbService` with the given database.
    pub fn new(db: Database) -> Self {
        Self { db: Arc::new(db) }
    }
}

impl Default for RedbService {
    fn default() -> Self {
        let tmp = tempfile::NamedTempFile::new()
            .expect("failed to create temp file for redb");
        let db = Database::create(tmp.path())
            .expect("failed to create redb database");
        Self { db: Arc::new(db) }
    }
}

impl ReplicableService for RedbService {
    type Command = RedbCommand;
    type Response = RedbResponse;

    fn apply(&mut self, command: Self::Command) -> Self::Response {
        match command {
            RedbCommand::Put { key, value } => {
                let write_txn = self.db.begin_write().expect("begin_write failed");
                {
                    let mut table = write_txn.open_table(TABLE).expect("open_table failed");
                    table
                        .insert(key.as_slice(), value.as_slice())
                        .expect("insert failed");
                }
                write_txn.commit().expect("commit failed");
                RedbResponse::Ok
            }
            RedbCommand::Get { key } => {
                let read_txn = self.db.begin_read().expect("begin_read failed");
                let table = match read_txn.open_table(TABLE) {
                    Ok(t) => t,
                    Err(TableError::TableDoesNotExist(_)) => return RedbResponse::Value(None),
                    Err(e) => panic!("open_table failed: {e}"),
                };
                let value = table
                    .get(key.as_slice())
                    .expect("get failed")
                    .map(|v| v.value().to_vec());
                RedbResponse::Value(value)
            }
            RedbCommand::Delete { key } => {
                let write_txn = self.db.begin_write().expect("begin_write failed");
                let existed = {
                    let mut table = write_txn.open_table(TABLE).expect("open_table failed");
                    table
                        .remove(key.as_slice())
                        .expect("remove failed")
                        .is_some()
                };
                write_txn.commit().expect("commit failed");
                RedbResponse::Deleted(existed)
            }
        }
    }

    fn is_read_only(command: &Self::Command) -> bool {
        matches!(command, RedbCommand::Get { .. })
    }

    fn snapshot(&self) -> Vec<u8> {
        let read_txn = self.db.begin_read().expect("begin_read failed");
        let table = match read_txn.open_table(TABLE) {
            Ok(t) => t,
            Err(TableError::TableDoesNotExist(_)) => {
                // No data yet — return empty snapshot
                let pairs: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
                return bincode::serialize(&pairs).expect("serialize failed");
            }
            Err(e) => panic!("open_table failed: {e}"),
        };
        let pairs: Vec<(Vec<u8>, Vec<u8>)> = table
            .iter()
            .expect("iter failed")
            .map(|entry| {
                let entry = entry.expect("entry failed");
                (entry.0.value().to_vec(), entry.1.value().to_vec())
            })
            .collect();
        bincode::serialize(&pairs).expect("serialize failed")
    }

    fn restore(&mut self, data: &[u8]) {
        let pairs: Vec<(Vec<u8>, Vec<u8>)> =
            bincode::deserialize(data).expect("deserialize failed");

        let write_txn = self.db.begin_write().expect("begin_write failed");
        {
            let mut table = write_txn.open_table(TABLE).expect("open_table failed");
            // Clear existing data by draining all entries
            let keys: Vec<Vec<u8>> = table
                .iter()
                .expect("iter failed")
                .map(|entry| {
                    let entry = entry.expect("entry failed");
                    entry.0.value().to_vec()
                })
                .collect();
            for key in keys {
                table.remove(key.as_slice()).expect("remove failed");
            }
            // Re-insert all pairs from the snapshot
            for (key, value) in pairs {
                table
                    .insert(key.as_slice(), value.as_slice())
                    .expect("insert failed");
            }
        }
        write_txn.commit().expect("commit failed");
    }
}
