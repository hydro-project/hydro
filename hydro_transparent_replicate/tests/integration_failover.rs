//! Integration test: primary failover with write durability.
//!
//! Demonstrates that:
//! 1. Writes committed through the replication protocol are durable.
//! 2. After a simulated primary failover (snapshot + restore on new primary),
//!    reads return previously written values.
//! 3. The new primary can continue serving both reads and writes.

use std::collections::HashMap;
use hydro_transparent_replicate::ReplicableService;

#[derive(Clone, Debug, Default)]
struct KvService {
    store: HashMap<String, String>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
enum KvCommand {
    Put { key: String, value: String },
    Get { key: String },
    Delete { key: String },
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
enum KvResponse {
    Ok,
    Value(Option<String>),
    Deleted(bool),
}

impl ReplicableService for KvService {
    type Command = KvCommand;
    type Response = KvResponse;

    fn apply(&mut self, command: Self::Command) -> Self::Response {
        match command {
            KvCommand::Put { key, value } => {
                self.store.insert(key, value);
                KvResponse::Ok
            }
            KvCommand::Get { key } => KvResponse::Value(self.store.get(&key).cloned()),
            KvCommand::Delete { key } => KvResponse::Deleted(self.store.remove(&key).is_some()),
        }
    }

    fn is_read_only(command: &Self::Command) -> bool {
        matches!(command, KvCommand::Get { .. })
    }

    fn snapshot(&self) -> Vec<u8> {
        bincode::serialize(&self.store).unwrap()
    }

    fn restore(&mut self, data: &[u8]) {
        self.store = bincode::deserialize(data).unwrap();
    }
}

/// Simulates a 3-replica cluster with primary/backup replication.
struct SimCluster {
    replicas: Vec<KvService>,
    primary_idx: usize,
    next_seq: usize,
}

impl SimCluster {
    fn new(n: usize) -> Self {
        Self {
            replicas: (0..n).map(|_| KvService::default()).collect(),
            primary_idx: 0,
            next_seq: 0,
        }
    }

    /// Execute a command. Mutating commands are replicated to all; reads go to primary only.
    fn execute(&mut self, command: KvCommand) -> KvResponse {
        if KvService::is_read_only(&command) {
            return self.replicas[self.primary_idx].apply(command);
        }
        self.next_seq += 1;
        let mut primary_resp = None;
        for (i, replica) in self.replicas.iter_mut().enumerate() {
            let resp = replica.apply(command.clone());
            if i == self.primary_idx {
                primary_resp = Some(resp);
            }
        }
        primary_resp.unwrap()
    }

    /// Kill the primary. Elect the next replica. New primary restores from a surviving backup's snapshot.
    /// The old primary's state is wiped — it cannot contribute to the new primary's state.
    fn failover(&mut self) {
        let old = self.primary_idx;
        let new = (old + 1) % self.replicas.len();

        // Take snapshot from the new primary (it was a backup with up-to-date state).
        let snapshot = self.replicas[new].snapshot();

        // Wipe ALL replicas to simulate a real crash scenario where we can't trust
        // any in-memory state — only the snapshot is authoritative.
        for r in self.replicas.iter_mut() {
            *r = KvService::default();
        }

        // New primary restores exclusively from the snapshot.
        self.replicas[new].restore(&snapshot);
        self.primary_idx = new;
    }

    /// Read a key from the current primary.
    fn get(&mut self, key: &str) -> Option<String> {
        match self.execute(KvCommand::Get { key: key.to_string() }) {
            KvResponse::Value(v) => v,
            other => panic!("unexpected response: {:?}", other),
        }
    }

    /// Write a key-value pair through the replication protocol.
    fn put(&mut self, key: &str, value: &str) {
        let resp = self.execute(KvCommand::Put {
            key: key.to_string(),
            value: value.to_string(),
        });
        assert_eq!(resp, KvResponse::Ok);
    }
}

#[test]
fn test_writes_durable_after_primary_failover() {
    let mut cluster = SimCluster::new(3);

    // Write several key-value pairs.
    cluster.put("name", "alice");
    cluster.put("city", "seattle");
    cluster.put("lang", "rust");

    // Verify reads work before failover.
    assert_eq!(cluster.get("name"), Some("alice".to_string()));
    assert_eq!(cluster.get("city"), Some("seattle".to_string()));
    assert_eq!(cluster.get("lang"), Some("rust".to_string()));

    // Kill the primary, elect a new one via snapshot-based state transfer.
    cluster.failover();

    // Reads on the NEW primary must return the previously written values.
    assert_eq!(cluster.get("name"), Some("alice".to_string()));
    assert_eq!(cluster.get("city"), Some("seattle".to_string()));
    assert_eq!(cluster.get("lang"), Some("rust".to_string()));

    // New primary can continue serving writes.
    cluster.put("version", "2.0");
    assert_eq!(cluster.get("version"), Some("2.0".to_string()));
}

#[test]
fn test_multiple_failovers_preserve_state() {
    let mut cluster = SimCluster::new(3);

    cluster.put("counter", "1");
    assert_eq!(cluster.get("counter"), Some("1".to_string()));

    // First failover: primary 0 → primary 1
    cluster.failover();
    assert_eq!(cluster.get("counter"), Some("1".to_string()));

    cluster.put("counter", "2");
    assert_eq!(cluster.get("counter"), Some("2".to_string()));

    // Second failover: primary 1 → primary 2
    cluster.failover();
    assert_eq!(cluster.get("counter"), Some("2".to_string()));

    cluster.put("counter", "3");
    assert_eq!(cluster.get("counter"), Some("3".to_string()));
}

#[test]
fn test_delete_survives_failover() {
    let mut cluster = SimCluster::new(3);

    cluster.put("temp", "value");
    assert_eq!(cluster.get("temp"), Some("value".to_string()));

    // Delete the key.
    let resp = cluster.execute(KvCommand::Delete { key: "temp".to_string() });
    assert_eq!(resp, KvResponse::Deleted(true));
    assert_eq!(cluster.get("temp"), None);

    // Failover — deletion must be preserved.
    cluster.failover();
    assert_eq!(cluster.get("temp"), None);
}

#[test]
fn test_read_only_commands_dont_mutate_state_across_failover() {
    let mut cluster = SimCluster::new(3);

    cluster.put("key", "val");

    // Issue many reads — they should not affect state.
    for _ in 0..100 {
        assert_eq!(cluster.get("key"), Some("val".to_string()));
        assert_eq!(cluster.get("nonexistent"), None);
    }

    // Failover — state should be unchanged (only the one put).
    cluster.failover();
    assert_eq!(cluster.get("key"), Some("val".to_string()));
    assert_eq!(cluster.get("nonexistent"), None);
}

/// Tests with the redb backend (if feature enabled).
#[cfg(feature = "backend_redb")]
mod redb_failover {
    use hydro_transparent_replicate::backends::redb::{RedbCommand, RedbResponse, RedbService};
    use hydro_transparent_replicate::ReplicableService;

    #[test]
    fn test_redb_write_durability_across_failover() {
        // Primary applies writes.
        let mut primary = RedbService::default();
        let mut backup = RedbService::default();

        let cmds = vec![
            RedbCommand::Put { key: b"k1".to_vec(), value: b"v1".to_vec() },
            RedbCommand::Put { key: b"k2".to_vec(), value: b"v2".to_vec() },
            RedbCommand::Put { key: b"k3".to_vec(), value: b"v3".to_vec() },
        ];

        for cmd in &cmds {
            primary.apply(cmd.clone());
            backup.apply(cmd.clone());
        }

        // Simulate failover: snapshot from backup, restore on new primary.
        // Wipe the primary first — it's dead. The new primary starts fresh and
        // restores ONLY from the snapshot. This is the real test.
        let snapshot = backup.snapshot();
        drop(primary); // primary is dead
        let mut new_primary = RedbService::default();
        new_primary.restore(&snapshot);

        // Reads on new primary must return previously written values.
        assert_eq!(
            new_primary.apply(RedbCommand::Get { key: b"k1".to_vec() }),
            RedbResponse::Value(Some(b"v1".to_vec()))
        );
        assert_eq!(
            new_primary.apply(RedbCommand::Get { key: b"k2".to_vec() }),
            RedbResponse::Value(Some(b"v2".to_vec()))
        );
        assert_eq!(
            new_primary.apply(RedbCommand::Get { key: b"k3".to_vec() }),
            RedbResponse::Value(Some(b"v3".to_vec()))
        );

        // New primary can continue writing.
        new_primary.apply(RedbCommand::Put { key: b"k4".to_vec(), value: b"v4".to_vec() });
        assert_eq!(
            new_primary.apply(RedbCommand::Get { key: b"k4".to_vec() }),
            RedbResponse::Value(Some(b"v4".to_vec()))
        );
    }
}

/// Tests with the rusqlite backend (if feature enabled).
#[cfg(feature = "backend_rusqlite")]
mod rusqlite_failover {
    use hydro_transparent_replicate::backends::rusqlite::{RusqliteService, SqlCommand, SqlResponse};
    use hydro_transparent_replicate::ReplicableService;

    #[test]
    fn test_rusqlite_write_durability_across_failover() {
        let mut primary = RusqliteService::default();
        let mut backup = RusqliteService::default();

        let setup = vec![
            SqlCommand("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)".to_string()),
            SqlCommand("INSERT INTO users VALUES (1, 'alice')".to_string()),
            SqlCommand("INSERT INTO users VALUES (2, 'bob')".to_string()),
        ];

        for cmd in &setup {
            primary.apply(cmd.clone());
            backup.apply(cmd.clone());
        }

        // Failover via snapshot/restore.
        let snapshot = backup.snapshot();
        let mut new_primary = RusqliteService::default();
        new_primary.restore(&snapshot);

        // Read from new primary — data must be present.
        let resp = new_primary.apply(SqlCommand("SELECT name FROM users ORDER BY id".to_string()));
        match resp {
            SqlResponse::Query { columns, rows } => {
                assert_eq!(columns, vec!["name".to_string()]);
                assert_eq!(rows.len(), 2);
                assert_eq!(rows[0], vec![Some("alice".to_string())]);
                assert_eq!(rows[1], vec![Some("bob".to_string())]);
            }
            other => panic!("expected Query response, got {:?}", other),
        }

        // New primary can continue writing.
        new_primary.apply(SqlCommand("INSERT INTO users VALUES (3, 'carol')".to_string()));
        let resp = new_primary.apply(SqlCommand("SELECT name FROM users WHERE id = 3".to_string()));
        match resp {
            SqlResponse::Query { rows, .. } => {
                assert_eq!(rows[0], vec![Some("carol".to_string())]);
            }
            other => panic!("expected Query response, got {:?}", other),
        }
    }
}
