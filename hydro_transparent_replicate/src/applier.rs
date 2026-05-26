//! Concrete applier types for use inside Hydro dataflow `q!()` closures.
//!
//! All types are always defined (not feature-gated) so they can be referenced
//! in `q!()` closures and as type parameters regardless of which features are active.
//! The actual storage operations are gated inside the implementations.
//!
//! ## Command / Response Wire Format
//!
//! Commands carry an optional nonce as the last colon-separated field:
//!   `PUT:<key>:<value>:<nonce>`   →  `OK nonce=<nonce> seq=<seq> PUT <key>=<value>`
//!   `GET:<key>:<nonce>`           →  `VALUE nonce=<nonce> GET <key>=<value>`

/// State for the redb applier inside a Hydro dataflow fold or scan.
/// Always defined so it can be used as a type in `q!()` closures.
#[derive(Clone)]
pub struct RedbApplierState {
    #[cfg(feature = "backend_redb")]
    pub(crate) db: std::sync::Arc<redb::Database>,
    #[cfg(feature = "backend_redb")]
    _tmp: std::sync::Arc<tempfile::NamedTempFile>,
    #[cfg(not(feature = "backend_redb"))]
    _phantom: (),
}

impl RedbApplierState {
    pub fn new() -> Self {
        #[cfg(feature = "backend_redb")]
        {
            let tmp = tempfile::NamedTempFile::new().expect("failed to create tempfile for redb");
            let db = redb::Database::create(tmp.path()).expect("failed to create redb database");
            Self {
                db: std::sync::Arc::new(db),
                _tmp: std::sync::Arc::new(tmp),
            }
        }
        #[cfg(not(feature = "backend_redb"))]
        {
            panic!("backend_redb feature is required to use RedbApplierState");
        }
    }

    /// Apply a command string and return a response string.
    ///
    /// Formats:
    ///   `PUT:<key>:<value>:<nonce>`  → `OK nonce=<n> seq=<s> PUT <k>=<v>`
    ///   `GET:<key>:<nonce>`          → `VALUE nonce=<n> GET <k>=<v>`
    pub fn apply_command(&self, seq: usize, cmd: &str) -> String {
        #[cfg(feature = "backend_redb")]
        {
            use redb::ReadableTable;
            const TABLE: redb::TableDefinition<'static, &[u8], &[u8]> =
                redb::TableDefinition::new("kv");

            let parts: Vec<&str> = cmd.splitn(4, ':').collect();
            match parts[0] {
                "PUT" => {
                    if parts.len() < 3 {
                        return format!("ERROR seq={} malformed PUT", seq);
                    }
                    let key = parts[1].as_bytes();
                    let value = parts[2].as_bytes();
                    let nonce: u64 = if parts.len() >= 4 {
                        parts[3].parse().unwrap_or(0)
                    } else {
                        0
                    };
                    let write_txn = self.db.begin_write().expect("begin_write");
                    {
                        let mut table = write_txn.open_table(TABLE).expect("open_table");
                        table.insert(key, value).expect("insert");
                    }
                    write_txn.commit().expect("commit");
                    format!("OK nonce={} seq={} PUT {}={}", nonce, seq, parts[1], parts[2])
                }
                "GET" => {
                    if parts.len() < 2 {
                        return format!("ERROR seq={} malformed GET", seq);
                    }
                    let key = parts[1].as_bytes();
                    let nonce: u64 = if parts.len() >= 3 {
                        parts[2].parse().unwrap_or(0)
                    } else {
                        0
                    };
                    let read_txn = self.db.begin_read().expect("begin_read");
                    let value = match read_txn.open_table(TABLE) {
                        Ok(table) => table
                            .get(key)
                            .expect("get")
                            .map(|v| String::from_utf8_lossy(v.value()).to_string())
                            .unwrap_or_else(|| "(nil)".to_string()),
                        Err(_) => "(nil)".to_string(),
                    };
                    format!("VALUE nonce={} GET {}={}", nonce, parts[1], value)
                }
                _ => format!("ERROR seq={} unknown command", seq),
            }
        }
        #[cfg(not(feature = "backend_redb"))]
        {
            let _ = (seq, cmd);
            panic!("backend_redb feature is required to use RedbApplierState");
        }
    }
}

impl Default for RedbApplierState {
    fn default() -> Self {
        Self::new()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// RedbStringService — ReplicableService impl for the EC2 demos
// ─────────────────────────────────────────────────────────────────────────────

/// A [`crate::ReplicableService`] that stores string key-value pairs in redb.
///
/// - `Command = String`: `PUT:<key>:<value>:<nonce>` or `GET:<key>:<nonce>`
/// - `Response = String`: `OK nonce=<n> seq=<s> PUT <k>=<v>` or `VALUE nonce=<n> GET <k>=<v>`
/// - GETs are read-only and bypass replication (served directly from primary).
///
/// Always defined (not feature-gated) so it can be used as a type parameter
/// in `replicate_service::<RedbStringService>(...)` calls.
#[derive(Clone)]
pub struct RedbStringService {
    #[cfg(feature = "backend_redb")]
    state: RedbApplierState,
    #[cfg(feature = "backend_redb")]
    seq: usize,
    #[cfg(not(feature = "backend_redb"))]
    _phantom: (),
}

impl Default for RedbStringService {
    fn default() -> Self {
        #[cfg(feature = "backend_redb")]
        {
            Self { state: RedbApplierState::new(), seq: 0 }
        }
        #[cfg(not(feature = "backend_redb"))]
        {
            Self { _phantom: () }
        }
    }
}

impl crate::ReplicableService for RedbStringService {
    type Command = String;
    type Response = String;

    fn apply(&mut self, command: String) -> String {
        #[cfg(feature = "backend_redb")]
        {
            let resp = self.state.apply_command(self.seq, &command);
            if command.starts_with("PUT:") {
                self.seq += 1;
            }
            resp
        }
        #[cfg(not(feature = "backend_redb"))]
        {
            let _ = command;
            panic!("backend_redb feature required");
        }
    }

    fn is_read_only(command: &String) -> bool {
        command.starts_with("GET:")
    }

    fn snapshot(&self) -> Vec<u8> {
        #[cfg(feature = "backend_redb")]
        {
            use redb::ReadableTable;
            const TABLE: redb::TableDefinition<'static, &[u8], &[u8]> =
                redb::TableDefinition::new("kv");
            let read_txn = self.state.db.begin_read().expect("begin_read");
            let mut pairs: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
            if let Ok(table) = read_txn.open_table(TABLE) {
                for entry in table.iter().expect("iter") {
                    let (k, v) = entry.expect("entry");
                    pairs.push((k.value().to_vec(), v.value().to_vec()));
                }
            }
            bincode::serialize(&(pairs, self.seq)).expect("serialize snapshot")
        }
        #[cfg(not(feature = "backend_redb"))]
        {
            panic!("backend_redb feature required");
        }
    }

    fn restore(&mut self, data: &[u8]) {
        #[cfg(feature = "backend_redb")]
        {
            use redb::ReadableTable;
            const TABLE: redb::TableDefinition<'static, &[u8], &[u8]> =
                redb::TableDefinition::new("kv");
            let (pairs, seq): (Vec<(Vec<u8>, Vec<u8>)>, usize) =
                bincode::deserialize(data).expect("deserialize snapshot");
            let write_txn = self.state.db.begin_write().expect("begin_write");
            {
                let mut table = write_txn.open_table(TABLE).expect("open_table");
                let keys: Vec<Vec<u8>> = table.iter().expect("iter")
                    .map(|e| e.expect("entry").0.value().to_vec())
                    .collect();
                for k in keys {
                    table.remove(k.as_slice()).expect("remove");
                }
                for (k, v) in pairs {
                    table.insert(k.as_slice(), v.as_slice()).expect("insert");
                }
            }
            write_txn.commit().expect("commit");
            self.seq = seq;
        }
        #[cfg(not(feature = "backend_redb"))]
        {
            let _ = data;
            panic!("backend_redb feature required");
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// FjallApplierState — fjall backend for use in q!() closures
// ─────────────────────────────────────────────────────────────────────────────

/// State for the fjall applier inside a Hydro dataflow scan.
/// Always defined (not feature-gated) so it can be used as a type in `q!()` closures.
/// Same wire format as `RedbApplierState`: `PUT:<key>:<value>:<nonce>` / `GET:<key>:<nonce>`.
#[derive(Clone)]
pub struct FjallApplierState {
    #[cfg(feature = "backend_fjall")]
    keyspace: std::sync::Arc<fjall::Keyspace>,
    #[cfg(feature = "backend_fjall")]
    partition: fjall::PartitionHandle,
    #[cfg(feature = "backend_fjall")]
    _tempdir: std::sync::Arc<tempfile::TempDir>,
    #[cfg(not(feature = "backend_fjall"))]
    _phantom: (),
}

impl FjallApplierState {
    pub fn new() -> Self {
        #[cfg(feature = "backend_fjall")]
        {
            let tempdir = tempfile::tempdir().expect("failed to create temp dir for fjall");
            let keyspace = fjall::Config::new(tempdir.path())
                .open()
                .expect("failed to open fjall keyspace");
            let partition = keyspace
                .open_partition("kv", fjall::PartitionCreateOptions::default())
                .expect("failed to open fjall partition");
            Self {
                keyspace: std::sync::Arc::new(keyspace),
                partition,
                _tempdir: std::sync::Arc::new(tempdir),
            }
        }
        #[cfg(not(feature = "backend_fjall"))]
        {
            panic!("backend_fjall feature is required to use FjallApplierState");
        }
    }

    /// Apply a command string and return a response string.
    /// Same wire format as `RedbApplierState`.
    pub fn apply_command(&self, seq: usize, cmd: &str) -> String {
        #[cfg(feature = "backend_fjall")]
        {
            let parts: Vec<&str> = cmd.splitn(4, ':').collect();
            match parts[0] {
                "PUT" => {
                    if parts.len() < 3 {
                        return format!("ERROR seq={} malformed PUT", seq);
                    }
                    let nonce: u64 = if parts.len() >= 4 {
                        parts[3].parse().unwrap_or(0)
                    } else {
                        0
                    };
                    self.partition
                        .insert(parts[1].as_bytes(), parts[2].as_bytes())
                        .expect("fjall insert failed");
                    format!("OK nonce={} seq={} PUT {}={}", nonce, seq, parts[1], parts[2])
                }
                "GET" => {
                    if parts.len() < 2 {
                        return format!("ERROR seq={} malformed GET", seq);
                    }
                    let nonce: u64 = if parts.len() >= 3 {
                        parts[2].parse().unwrap_or(0)
                    } else {
                        0
                    };
                    let value = self
                        .partition
                        .get(parts[1].as_bytes())
                        .expect("fjall get failed")
                        .map(|v| String::from_utf8_lossy(&v).to_string())
                        .unwrap_or_else(|| "(nil)".to_string());
                    format!("VALUE nonce={} GET {}={}", nonce, parts[1], value)
                }
                _ => format!("ERROR seq={} unknown command", seq),
            }
        }
        #[cfg(not(feature = "backend_fjall"))]
        {
            let _ = (seq, cmd);
            panic!("backend_fjall feature is required to use FjallApplierState");
        }
    }
}

impl Default for FjallApplierState {
    fn default() -> Self {
        Self::new()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// RusqliteApplierState — rusqlite backend for use in q!() closures
// ─────────────────────────────────────────────────────────────────────────────

/// State for the rusqlite applier inside a Hydro dataflow scan.
/// Always defined (not feature-gated) so it can be used as a type in `q!()` closures.
/// Same wire format as `RedbApplierState`: `PUT:<key>:<value>:<nonce>` / `GET:<key>:<nonce>`.
#[derive(Clone)]
pub struct RusqliteApplierState {
    #[cfg(feature = "backend_rusqlite")]
    conn: std::sync::Arc<std::sync::Mutex<rusqlite::Connection>>,
    #[cfg(not(feature = "backend_rusqlite"))]
    _phantom: (),
}

impl RusqliteApplierState {
    pub fn new() -> Self {
        #[cfg(feature = "backend_rusqlite")]
        {
            let conn = rusqlite::Connection::open_in_memory()
                .expect("failed to open in-memory SQLite");
            conn.execute(
                "CREATE TABLE IF NOT EXISTS kv (key TEXT PRIMARY KEY, value TEXT NOT NULL)",
                [],
            ).expect("failed to create kv table");
            Self { conn: std::sync::Arc::new(std::sync::Mutex::new(conn)) }
        }
        #[cfg(not(feature = "backend_rusqlite"))]
        {
            panic!("backend_rusqlite feature is required to use RusqliteApplierState");
        }
    }

    /// Apply a command string and return a response string.
    /// Same wire format as `RedbApplierState`.
    pub fn apply_command(&self, seq: usize, cmd: &str) -> String {
        #[cfg(feature = "backend_rusqlite")]
        {
            let conn = self.conn.lock().unwrap();
            let parts: Vec<&str> = cmd.splitn(4, ':').collect();
            match parts[0] {
                "PUT" => {
                    if parts.len() < 3 {
                        return format!("ERROR seq={} malformed PUT", seq);
                    }
                    let nonce: u64 = if parts.len() >= 4 {
                        parts[3].parse().unwrap_or(0)
                    } else {
                        0
                    };
                    conn.execute(
                        "INSERT OR REPLACE INTO kv (key, value) VALUES (?1, ?2)",
                        rusqlite::params![parts[1], parts[2]],
                    ).expect("rusqlite insert failed");
                    format!("OK nonce={} seq={} PUT {}={}", nonce, seq, parts[1], parts[2])
                }
                "GET" => {
                    if parts.len() < 2 {
                        return format!("ERROR seq={} malformed GET", seq);
                    }
                    let nonce: u64 = if parts.len() >= 3 {
                        parts[2].parse().unwrap_or(0)
                    } else {
                        0
                    };
                    let value: String = conn
                        .query_row(
                            "SELECT value FROM kv WHERE key = ?1",
                            rusqlite::params![parts[1]],
                            |row| row.get(0),
                        )
                        .unwrap_or_else(|_| "(nil)".to_string());
                    format!("VALUE nonce={} GET {}={}", nonce, parts[1], value)
                }
                _ => format!("ERROR seq={} unknown command", seq),
            }
        }
        #[cfg(not(feature = "backend_rusqlite"))]
        {
            let _ = (seq, cmd);
            panic!("backend_rusqlite feature is required to use RusqliteApplierState");
        }
    }
}

impl Default for RusqliteApplierState {
    fn default() -> Self {
        Self::new()
    }
}
