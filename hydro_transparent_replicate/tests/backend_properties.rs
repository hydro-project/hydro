//! Property-based tests for all four backend implementations.
//!
//! Tests Properties 4, 9, and 10 from the design document against each backend:
//! - Property 4: Read-only commands preserve state
//! - Property 9: Observational determinism
//! - Property 10: Snapshot/restore round trip

use proptest::prelude::*;

// Needed in scope for calling trait methods (apply, snapshot, restore) on concrete types
use hydro_transparent_replicate::ReplicableService;

// ============================================================================
// Strategies for redb backend
// ============================================================================

#[cfg(feature = "backend_redb")]
mod redb_strategies {
    use super::*;
    use hydro_transparent_replicate::backends::redb::RedbCommand;

    /// Generate a random redb command with small keys/values.
    pub fn arb_redb_command() -> impl Strategy<Value = RedbCommand> {
        prop_oneof![
            (
                prop::collection::vec(any::<u8>(), 1..10),
                prop::collection::vec(any::<u8>(), 1..10)
            )
                .prop_map(|(key, value)| RedbCommand::Put { key, value }),
            prop::collection::vec(any::<u8>(), 1..10)
                .prop_map(|key| RedbCommand::Get { key }),
            prop::collection::vec(any::<u8>(), 1..10)
                .prop_map(|key| RedbCommand::Delete { key }),
        ]
    }

    /// Generate a mutating redb command (Put or Delete only).
    pub fn arb_redb_mutating_command() -> impl Strategy<Value = RedbCommand> {
        prop_oneof![
            (
                prop::collection::vec(any::<u8>(), 1..10),
                prop::collection::vec(any::<u8>(), 1..10)
            )
                .prop_map(|(key, value)| RedbCommand::Put { key, value }),
            prop::collection::vec(any::<u8>(), 1..10)
                .prop_map(|key| RedbCommand::Delete { key }),
        ]
    }

    /// Generate a read-only redb command (Get only).
    pub fn arb_redb_read_only_command() -> impl Strategy<Value = RedbCommand> {
        prop::collection::vec(any::<u8>(), 1..10).prop_map(|key| RedbCommand::Get { key })
    }
}

// ============================================================================
// Strategies for sled backend
// ============================================================================

#[cfg(feature = "backend_sled")]
mod sled_strategies {
    use super::*;
    use hydro_transparent_replicate::backends::sled::SledCommand;

    /// Generate a random sled command with small keys/values.
    pub fn arb_sled_command() -> impl Strategy<Value = SledCommand> {
        prop_oneof![
            (
                prop::collection::vec(any::<u8>(), 1..10),
                prop::collection::vec(any::<u8>(), 1..10)
            )
                .prop_map(|(key, value)| SledCommand::Insert { key, value }),
            prop::collection::vec(any::<u8>(), 1..10)
                .prop_map(|key| SledCommand::Get { key }),
            prop::collection::vec(any::<u8>(), 1..10)
                .prop_map(|key| SledCommand::Remove { key }),
        ]
    }

    /// Generate a mutating sled command (Insert or Remove only).
    pub fn arb_sled_mutating_command() -> impl Strategy<Value = SledCommand> {
        prop_oneof![
            (
                prop::collection::vec(any::<u8>(), 1..10),
                prop::collection::vec(any::<u8>(), 1..10)
            )
                .prop_map(|(key, value)| SledCommand::Insert { key, value }),
            prop::collection::vec(any::<u8>(), 1..10)
                .prop_map(|key| SledCommand::Remove { key }),
        ]
    }

    /// Generate a read-only sled command (Get only).
    pub fn arb_sled_read_only_command() -> impl Strategy<Value = SledCommand> {
        prop::collection::vec(any::<u8>(), 1..10).prop_map(|key| SledCommand::Get { key })
    }
}

// ============================================================================
// Strategies for fjall backend
// ============================================================================

#[cfg(feature = "backend_fjall")]
mod fjall_strategies {
    use super::*;
    use hydro_transparent_replicate::backends::fjall::FjallCommand;

    /// Generate a random fjall command with small keys/values.
    pub fn arb_fjall_command() -> impl Strategy<Value = FjallCommand> {
        prop_oneof![
            (
                prop::collection::vec(any::<u8>(), 1..10),
                prop::collection::vec(any::<u8>(), 1..10)
            )
                .prop_map(|(key, value)| FjallCommand::Insert { key, value }),
            prop::collection::vec(any::<u8>(), 1..10)
                .prop_map(|key| FjallCommand::Get { key }),
            prop::collection::vec(any::<u8>(), 1..10)
                .prop_map(|key| FjallCommand::Remove { key }),
        ]
    }

    /// Generate a mutating fjall command (Insert or Remove only).
    pub fn arb_fjall_mutating_command() -> impl Strategy<Value = FjallCommand> {
        prop_oneof![
            (
                prop::collection::vec(any::<u8>(), 1..10),
                prop::collection::vec(any::<u8>(), 1..10)
            )
                .prop_map(|(key, value)| FjallCommand::Insert { key, value }),
            prop::collection::vec(any::<u8>(), 1..10)
                .prop_map(|key| FjallCommand::Remove { key }),
        ]
    }

    /// Generate a read-only fjall command (Get only).
    pub fn arb_fjall_read_only_command() -> impl Strategy<Value = FjallCommand> {
        prop::collection::vec(any::<u8>(), 1..10).prop_map(|key| FjallCommand::Get { key })
    }
}

// ============================================================================
// Strategies for rusqlite backend
// ============================================================================

#[cfg(feature = "backend_rusqlite")]
mod rusqlite_strategies {
    use super::*;
    use hydro_transparent_replicate::backends::rusqlite::SqlCommand;

    /// Generate a safe table name (lowercase alpha, 3-8 chars).
    fn arb_table_name() -> impl Strategy<Value = String> {
        "[a-z]{3,8}".prop_map(|s| s)
    }

    /// Generate a safe column value (alphanumeric, 1-10 chars).
    fn arb_value() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9]{1,10}".prop_map(|s| s)
    }

    /// Generate a random integer for use in SQL.
    fn arb_int_value() -> impl Strategy<Value = i32> {
        0..1000i32
    }

    /// Generate a mutating SQL command (CREATE TABLE or INSERT).
    pub fn arb_rusqlite_mutating_command() -> impl Strategy<Value = SqlCommand> {
        prop_oneof![
            // CREATE TABLE IF NOT EXISTS with a fixed schema
            arb_table_name().prop_map(|name| SqlCommand(format!(
                "CREATE TABLE IF NOT EXISTS {name} (id INTEGER PRIMARY KEY, val TEXT NOT NULL)"
            ))),
            // INSERT into a known table
            (arb_table_name(), arb_int_value(), arb_value()).prop_map(|(name, id, val)| {
                SqlCommand(format!(
                    "INSERT OR REPLACE INTO {name} (id, val) VALUES ({id}, '{val}')"
                ))
            }),
        ]
    }

    /// Generate a read-only SQL command (SELECT).
    pub fn arb_rusqlite_read_only_command() -> impl Strategy<Value = SqlCommand> {
        arb_table_name().prop_map(|name| {
            SqlCommand(format!("SELECT * FROM {name} WHERE 1=0"))
        })
    }

    /// Generate a sequence of SQL commands that builds up state deterministically.
    /// First creates a table, then inserts rows, then optionally selects.
    pub fn arb_rusqlite_command_sequence(size: usize) -> impl Strategy<Value = Vec<SqlCommand>> {
        // Always start with a CREATE TABLE to ensure the table exists
        let table_name = "test_tbl";
        (
            prop::collection::vec(
                (arb_int_value(), arb_value()).prop_map(move |(id, val)| {
                    SqlCommand(format!(
                        "INSERT OR REPLACE INTO {table_name} (id, val) VALUES ({id}, '{val}')"
                    ))
                }),
                1..size,
            )
        )
            .prop_map(|inserts| {
                let mut cmds = vec![SqlCommand(format!(
                    "CREATE TABLE IF NOT EXISTS test_tbl (id INTEGER PRIMARY KEY, val TEXT NOT NULL)"
                ))];
                cmds.extend(inserts);
                cmds
            })
    }
}

// ============================================================================
// Property tests for redb backend
// ============================================================================

#[cfg(feature = "backend_redb")]
mod redb_properties {
    use super::*;
    use hydro_transparent_replicate::backends::redb::RedbService;
    use redb_strategies::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(20))]

        /// **Validates: Requirements 10.4**
        ///
        /// Property 4: Read-only commands preserve state.
        /// Applying a read-only command does not change snapshot().
        #[test]
        fn property_4_read_only_preserves_state_redb(
            setup_commands in prop::collection::vec(arb_redb_mutating_command(), 0..10),
            read_command in arb_redb_read_only_command(),
        ) {
            let mut service = RedbService::default();

            // Apply some mutating commands to build up state
            for cmd in setup_commands {
                service.apply(cmd);
            }

            // Take snapshot before read-only command
            let snapshot_before = service.snapshot();

            // Apply read-only command
            let _ = service.apply(read_command);

            // Take snapshot after read-only command
            let snapshot_after = service.snapshot();

            // Snapshots must be identical
            prop_assert_eq!(snapshot_before, snapshot_after);
        }

        /// **Validates: Requirements 10.3, 8.3**
        ///
        /// Property 9: Observational determinism.
        /// Same command sequence on two fresh instances produces identical responses.
        #[test]
        fn property_9_observational_determinism_redb(
            commands in prop::collection::vec(arb_redb_command(), 1..30),
        ) {
            let mut service_a = RedbService::default();
            let mut service_b = RedbService::default();

            for cmd in commands {
                let resp_a = service_a.apply(cmd.clone());
                let resp_b = service_b.apply(cmd);
                prop_assert_eq!(resp_a, resp_b);
            }
        }

        /// **Validates: Requirements 10.3, 10.4**
        ///
        /// Property 10: Snapshot/restore round trip.
        /// snapshot() then restore() on a fresh instance produces equivalent state.
        #[test]
        fn property_10_snapshot_restore_round_trip_redb(
            setup_commands in prop::collection::vec(arb_redb_command(), 1..20),
            verify_commands in prop::collection::vec(arb_redb_command(), 1..10),
        ) {
            // Apply setup commands to instance A
            let mut service_a = RedbService::default();
            for cmd in &setup_commands {
                service_a.apply(cmd.clone());
            }

            // Take snapshot of A
            let snapshot = service_a.snapshot();

            // Create fresh instance B, restore from snapshot
            let mut service_b = RedbService::default();
            service_b.restore(&snapshot);

            // Apply the same verify commands to both
            for cmd in verify_commands {
                let resp_a = service_a.apply(cmd.clone());
                let resp_b = service_b.apply(cmd);
                prop_assert_eq!(resp_a, resp_b);
            }
        }
    }
}

// ============================================================================
// Property tests for sled backend
// ============================================================================

#[cfg(feature = "backend_sled")]
mod sled_properties {
    use super::*;
    use hydro_transparent_replicate::backends::sled::SledService;
    use sled_strategies::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(20))]

        /// **Validates: Requirements 10.4**
        ///
        /// Property 4: Read-only commands preserve state.
        /// Applying a read-only command does not change snapshot().
        #[test]
        fn property_4_read_only_preserves_state_sled(
            setup_commands in prop::collection::vec(arb_sled_mutating_command(), 0..10),
            read_command in arb_sled_read_only_command(),
        ) {
            let mut service = SledService::default();

            // Apply some mutating commands to build up state
            for cmd in setup_commands {
                service.apply(cmd);
            }

            // Take snapshot before read-only command
            let snapshot_before = service.snapshot();

            // Apply read-only command
            let _ = service.apply(read_command);

            // Take snapshot after read-only command
            let snapshot_after = service.snapshot();

            // Snapshots must be identical
            prop_assert_eq!(snapshot_before, snapshot_after);
        }

        /// **Validates: Requirements 10.3, 8.3**
        ///
        /// Property 9: Observational determinism.
        /// Same command sequence on two fresh instances produces identical responses.
        #[test]
        fn property_9_observational_determinism_sled(
            commands in prop::collection::vec(arb_sled_command(), 1..30),
        ) {
            let mut service_a = SledService::default();
            let mut service_b = SledService::default();

            for cmd in commands {
                let resp_a = service_a.apply(cmd.clone());
                let resp_b = service_b.apply(cmd);
                prop_assert_eq!(resp_a, resp_b);
            }
        }

        /// **Validates: Requirements 10.3, 10.4**
        ///
        /// Property 10: Snapshot/restore round trip.
        /// snapshot() then restore() on a fresh instance produces equivalent state.
        #[test]
        fn property_10_snapshot_restore_round_trip_sled(
            setup_commands in prop::collection::vec(arb_sled_command(), 1..20),
            verify_commands in prop::collection::vec(arb_sled_command(), 1..10),
        ) {
            // Apply setup commands to instance A
            let mut service_a = SledService::default();
            for cmd in &setup_commands {
                service_a.apply(cmd.clone());
            }

            // Take snapshot of A
            let snapshot = service_a.snapshot();

            // Create fresh instance B, restore from snapshot
            let mut service_b = SledService::default();
            service_b.restore(&snapshot);

            // Apply the same verify commands to both
            for cmd in verify_commands {
                let resp_a = service_a.apply(cmd.clone());
                let resp_b = service_b.apply(cmd);
                prop_assert_eq!(resp_a, resp_b);
            }
        }
    }
}

// ============================================================================
// Property tests for fjall backend
// ============================================================================

#[cfg(feature = "backend_fjall")]
mod fjall_properties {
    use super::*;
    use hydro_transparent_replicate::backends::fjall::FjallService;
    use fjall_strategies::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(20))]

        /// **Validates: Requirements 10.4**
        ///
        /// Property 4: Read-only commands preserve state.
        /// Applying a read-only command does not change snapshot().
        #[test]
        fn property_4_read_only_preserves_state_fjall(
            setup_commands in prop::collection::vec(arb_fjall_mutating_command(), 0..10),
            read_command in arb_fjall_read_only_command(),
        ) {
            let mut service = FjallService::default();

            // Apply some mutating commands to build up state
            for cmd in setup_commands {
                service.apply(cmd);
            }

            // Take snapshot before read-only command
            let snapshot_before = service.snapshot();

            // Apply read-only command
            let _ = service.apply(read_command);

            // Take snapshot after read-only command
            let snapshot_after = service.snapshot();

            // Snapshots must be identical
            prop_assert_eq!(snapshot_before, snapshot_after);
        }

        /// **Validates: Requirements 10.3, 8.3**
        ///
        /// Property 9: Observational determinism.
        /// Same command sequence on two fresh instances produces identical responses.
        #[test]
        fn property_9_observational_determinism_fjall(
            commands in prop::collection::vec(arb_fjall_command(), 1..30),
        ) {
            let mut service_a = FjallService::default();
            let mut service_b = FjallService::default();

            for cmd in commands {
                let resp_a = service_a.apply(cmd.clone());
                let resp_b = service_b.apply(cmd);
                prop_assert_eq!(resp_a, resp_b);
            }
        }

        /// **Validates: Requirements 10.3, 10.4**
        ///
        /// Property 10: Snapshot/restore round trip.
        /// snapshot() then restore() on a fresh instance produces equivalent state.
        #[test]
        fn property_10_snapshot_restore_round_trip_fjall(
            setup_commands in prop::collection::vec(arb_fjall_command(), 1..20),
            verify_commands in prop::collection::vec(arb_fjall_command(), 1..10),
        ) {
            // Apply setup commands to instance A
            let mut service_a = FjallService::default();
            for cmd in &setup_commands {
                service_a.apply(cmd.clone());
            }

            // Take snapshot of A
            let snapshot = service_a.snapshot();

            // Create fresh instance B, restore from snapshot
            let mut service_b = FjallService::default();
            service_b.restore(&snapshot);

            // Apply the same verify commands to both
            for cmd in verify_commands {
                let resp_a = service_a.apply(cmd.clone());
                let resp_b = service_b.apply(cmd);
                prop_assert_eq!(resp_a, resp_b);
            }
        }
    }
}

// ============================================================================
// Property tests for rusqlite backend
// ============================================================================

#[cfg(feature = "backend_rusqlite")]
mod rusqlite_properties {
    use super::*;
    use hydro_transparent_replicate::backends::rusqlite::{RusqliteService, SqlCommand};
    use rusqlite_strategies::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(20))]

        /// **Validates: Requirements 10.4**
        ///
        /// Property 4: Read-only commands preserve state.
        /// Applying a read-only command does not change snapshot().
        #[test]
        fn property_4_read_only_preserves_state_rusqlite(
            setup_commands in arb_rusqlite_command_sequence(10),
            read_command in arb_rusqlite_read_only_command(),
        ) {
            let mut service = RusqliteService::default();

            // Apply some mutating commands to build up state
            for cmd in setup_commands {
                service.apply(cmd);
            }

            // Take snapshot before read-only command
            let snapshot_before = service.snapshot();

            // Apply read-only command (SELECT on possibly non-existent table is fine,
            // it returns an error response but doesn't mutate state)
            let _ = service.apply(read_command);

            // Take snapshot after read-only command
            let snapshot_after = service.snapshot();

            // Snapshots must be identical
            prop_assert_eq!(snapshot_before, snapshot_after);
        }

        /// **Validates: Requirements 10.3, 8.3**
        ///
        /// Property 9: Observational determinism.
        /// Same command sequence on two fresh instances produces identical responses.
        #[test]
        fn property_9_observational_determinism_rusqlite(
            commands in arb_rusqlite_command_sequence(20),
        ) {
            let mut service_a = RusqliteService::default();
            let mut service_b = RusqliteService::default();

            for cmd in commands {
                let resp_a = service_a.apply(cmd.clone());
                let resp_b = service_b.apply(cmd);
                prop_assert_eq!(resp_a, resp_b);
            }
        }

        /// **Validates: Requirements 10.3, 10.4**
        ///
        /// Property 10: Snapshot/restore round trip.
        /// snapshot() then restore() on a fresh instance produces equivalent state.
        #[test]
        fn property_10_snapshot_restore_round_trip_rusqlite(
            setup_commands in arb_rusqlite_command_sequence(15),
            verify_ids in prop::collection::vec(0..1000i32, 1..5),
        ) {
            // Apply setup commands to instance A
            let mut service_a = RusqliteService::default();
            for cmd in &setup_commands {
                service_a.apply(cmd.clone());
            }

            // Take snapshot of A
            let snapshot = service_a.snapshot();

            // Create fresh instance B, restore from snapshot
            let mut service_b = RusqliteService::default();
            service_b.restore(&snapshot);

            // Apply the same verify commands to both (SELECT queries on the table)
            let verify_commands: Vec<SqlCommand> = verify_ids
                .into_iter()
                .map(|id| SqlCommand(format!("SELECT * FROM test_tbl WHERE id = {id}")))
                .collect();

            for cmd in verify_commands {
                let resp_a = service_a.apply(cmd.clone());
                let resp_b = service_b.apply(cmd);
                prop_assert_eq!(resp_a, resp_b);
            }
        }
    }
}
