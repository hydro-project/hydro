//! rusqlite backend implementation of `ReplicableService`.
//!
//! This module provides a SQL database service backed by
//! [rusqlite](https://docs.rs/rusqlite), a safe Rust wrapper around SQLite.
//!
//! # Observational Determinism
//!
//! SQL statements are deterministic **provided** the user avoids
//! non-deterministic SQLite functions. The following functions MUST NOT be used
//! in replicated SQL commands, as they will cause replica divergence:
//!
//! - `datetime('now')`, `date('now')`, `time('now')`, `julianday('now')`
//! - `random()`
//! - `last_insert_rowid()` (safe only if insert sequence is identical)
//! - `changes()` (safe only if mutation sequence is identical)
//!
//! The following are safe:
//! - `AUTOINCREMENT` (deterministic given the same insert sequence)
//! - `ROWID` assignment (deterministic for the same sequence of inserts/deletes)
//! - All arithmetic, string, and aggregate functions
//! - `CASE`, `COALESCE`, `NULLIF`, etc.

use crate::ReplicableService;
use serde::{Deserialize, Serialize};

use rusqlite::Connection;
use std::io::Write;

/// A replicated SQL database service backed by rusqlite (SQLite).
///
/// Wraps an in-memory SQLite connection. The `Default` implementation creates
/// a fresh in-memory database.
pub struct RusqliteService {
    conn: Connection,
}

/// A SQL command to execute against the database.
///
/// The string should contain a single SQL statement. Whether it is read-only
/// or mutating is determined by parsing the SQL prefix (SELECT, PRAGMA, EXPLAIN
/// are read-only; everything else is mutating).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SqlCommand(pub String);

/// Responses from the rusqlite service.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum SqlResponse {
    /// Result of a mutating statement (INSERT, UPDATE, DELETE, CREATE, etc.).
    Execute {
        /// Number of rows affected by the statement.
        rows_affected: usize,
    },
    /// Result of a read-only query (SELECT, PRAGMA, EXPLAIN).
    Query {
        /// Column names from the result set.
        columns: Vec<String>,
        /// Rows of values. Each value is `None` for SQL NULL, or `Some(text)`.
        rows: Vec<Vec<Option<String>>>,
    },
    /// An error occurred while executing the statement.
    /// Only deterministic errors (syntax errors, constraint violations) should
    /// appear here. Non-deterministic errors (disk full, etc.) will cause a panic.
    Error(String),
}

impl RusqliteService {
    /// Create a new `RusqliteService` with the given connection.
    pub fn new(conn: Connection) -> Self {
        Self { conn }
    }
}

impl Default for RusqliteService {
    fn default() -> Self {
        let conn =
            Connection::open_in_memory().expect("failed to open in-memory SQLite database");
        Self { conn }
    }
}

/// Returns `true` if the SQL statement is read-only based on its prefix.
///
/// Recognized read-only prefixes (case-insensitive, after trimming whitespace):
/// - `SELECT`
/// - `PRAGMA` (when not setting a value)
/// - `EXPLAIN`
fn is_sql_read_only(sql: &str) -> bool {
    let trimmed = sql.trim();
    let upper = trimmed.to_uppercase();
    upper.starts_with("SELECT")
        || upper.starts_with("PRAGMA")
        || upper.starts_with("EXPLAIN")
}

impl ReplicableService for RusqliteService {
    type Command = SqlCommand;
    type Response = SqlResponse;

    fn apply(&mut self, command: Self::Command) -> Self::Response {
        let sql = &command.0;

        if is_sql_read_only(sql) {
            // Execute as a query (returns rows)
            match self.execute_query(sql) {
                Ok(resp) => resp,
                Err(e) => SqlResponse::Error(e.to_string()),
            }
        } else {
            // Execute as a mutating statement
            match self.execute_mutate(sql) {
                Ok(resp) => resp,
                Err(e) => SqlResponse::Error(e.to_string()),
            }
        }
    }

    fn is_read_only(command: &Self::Command) -> bool {
        is_sql_read_only(&command.0)
    }

    fn snapshot(&self) -> Vec<u8> {
        // Use SQLite's backup API to serialize the database to bytes.
        // Back up the in-memory DB to a temp file, then read the bytes.
        let tmp = tempfile::NamedTempFile::new()
            .expect("failed to create temp file for SQLite snapshot");
        let tmp_path = tmp.path().to_path_buf();

        // Open a destination connection to the temp file
        let mut dst = Connection::open(&tmp_path)
            .expect("failed to open destination for SQLite backup");

        // Use the backup API to copy from our in-memory DB to the file
        {
            let backup = rusqlite::backup::Backup::new(&self.conn, &mut dst)
                .expect("failed to create backup for snapshot");
            backup
                .run_to_completion(100, std::time::Duration::from_millis(0), None)
                .expect("backup snapshot failed");
        }

        // Close the destination connection and read the file bytes
        drop(dst);
        std::fs::read(&tmp_path).expect("failed to read SQLite snapshot file")
    }

    fn restore(&mut self, data: &[u8]) {
        // Write the snapshot bytes to a temp file
        let mut tmp = tempfile::NamedTempFile::new()
            .expect("failed to create temp file for SQLite restore");
        tmp.write_all(data)
            .expect("failed to write SQLite snapshot to temp file");
        tmp.flush().expect("failed to flush temp file");

        let tmp_path = tmp.path().to_path_buf();

        // Open the snapshot file as a source database
        let src =
            Connection::open(&tmp_path).expect("failed to open SQLite snapshot for restore");

        // Create a new in-memory connection as the destination
        let mut new_conn = Connection::open_in_memory()
            .expect("failed to open new in-memory SQLite database");

        // Use the backup API: backup from src to new_conn
        {
            let backup = rusqlite::backup::Backup::new(&src, &mut new_conn)
                .expect("failed to create backup for restore");
            backup
                .run_to_completion(100, std::time::Duration::from_millis(0), None)
                .expect("backup restore failed");
        }

        self.conn = new_conn;
    }
}

impl RusqliteService {
    /// Execute a read-only query and return columns + rows.
    fn execute_query(&self, sql: &str) -> Result<SqlResponse, rusqlite::Error> {
        let mut stmt = self.conn.prepare(sql)?;
        let column_count = stmt.column_count();
        let columns: Vec<String> = (0..column_count)
            .map(|i| stmt.column_name(i).unwrap_or("?").to_string())
            .collect();

        let rows: Vec<Vec<Option<String>>> = stmt
            .query_map([], |row| {
                let mut values = Vec::with_capacity(column_count);
                for i in 0..column_count {
                    let value: Option<String> = row.get::<_, Option<String>>(i).unwrap_or(None);
                    values.push(value);
                }
                Ok(values)
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(SqlResponse::Query { columns, rows })
    }

    /// Execute a mutating statement and return the number of rows affected.
    fn execute_mutate(&mut self, sql: &str) -> Result<SqlResponse, rusqlite::Error> {
        let rows_affected = self.conn.execute(sql, [])?;
        Ok(SqlResponse::Execute { rows_affected })
    }
}
