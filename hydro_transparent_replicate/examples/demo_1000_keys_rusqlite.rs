//! Demo: write 1000 keys to rusqlite, crash primary, read back and verify.
//!
//! Run with: cargo run -p hydro_transparent_replicate --features backend_rusqlite --example demo_1000_keys_rusqlite

use hydro_transparent_replicate::backends::rusqlite::{RusqliteService, SqlCommand, SqlResponse};
use hydro_transparent_replicate::ReplicableService;
use std::collections::HashMap;

struct Cluster {
    replicas: Vec<Option<RusqliteService>>,
    primary_idx: usize,
}

impl Cluster {
    fn new(n: usize) -> Self {
        let mut replicas: Vec<Option<RusqliteService>> = (0..n).map(|_| Some(RusqliteService::default())).collect();
        // Create the table on all replicas.
        let create = SqlCommand("CREATE TABLE kv (key TEXT PRIMARY KEY, value TEXT NOT NULL)".to_string());
        for r in replicas.iter_mut().flatten() {
            r.apply(create.clone());
        }
        Self { replicas, primary_idx: 0 }
    }

    fn put(&mut self, key: &str, value: &str) {
        let cmd = SqlCommand(format!("INSERT OR REPLACE INTO kv (key, value) VALUES ('{}', '{}')", key, value));
        for r in self.replicas.iter_mut().flatten() {
            r.apply(cmd.clone());
        }
    }

    fn get(&mut self, key: &str) -> Option<String> {
        let cmd = SqlCommand(format!("SELECT value FROM kv WHERE key = '{}'", key));
        let r = self.replicas[self.primary_idx].as_mut().unwrap();
        match r.apply(cmd) {
            SqlResponse::Query { rows, .. } => {
                if rows.is_empty() { None }
                else { rows[0][0].clone() }
            }
            _ => panic!("unexpected response"),
        }
    }

    fn crash_primary(&mut self) {
        self.replicas[self.primary_idx] = None;
    }

    fn failover(&mut self) {
        let new = self.replicas.iter().position(|r| r.is_some()).unwrap();
        let snapshot = self.replicas[new].as_ref().unwrap().snapshot();
        let mut fresh = RusqliteService::default();
        fresh.restore(&snapshot);
        self.replicas[new] = Some(fresh);
        self.primary_idx = new;
    }
}

fn main() {
    let mut rng_state: u64 = 0xcafebabe;
    let mut expected: HashMap<String, String> = HashMap::new();
    let mut cluster = Cluster::new(3);

    println!("Writing 1000 keys to rusqlite...");
    for i in 0..1000 {
        rng_state ^= rng_state << 13;
        rng_state ^= rng_state >> 7;
        rng_state ^= rng_state << 17;
        let value = (rng_state % 1_000_000).to_string();
        let key = format!("k{}", i);
        cluster.put(&key, &value);
        expected.insert(key, value);
    }
    println!("  Done. 1000 keys written to 3 replicas (SQLite in-memory).");

    println!("\nSpot-check before crash:");
    for i in [0, 42, 500, 999] {
        let key = format!("k{}", i);
        let val = cluster.get(&key).unwrap();
        assert_eq!(&val, expected.get(&key).unwrap());
        println!("  SELECT value FROM kv WHERE key='{}' → {} ✓", key, val);
    }

    println!("\n💥 CRASH: Primary (replica 0) is dead.");
    cluster.crash_primary();

    println!("🔄 FAILOVER: Electing new primary via snapshot/restore...");
    cluster.failover();
    println!("  New primary: replica {} (restored from SQLite backup API snapshot)", cluster.primary_idx);

    println!("\nVerifying 20 keys after failover:");
    let mut verified = 0;
    for i in (0..1000).step_by(50) {
        let key = format!("k{}", i);
        let val = cluster.get(&key).unwrap();
        let expected_val = expected.get(&key).unwrap();
        assert_eq!(&val, expected_val);
        println!("  SELECT value FROM kv WHERE key='{}' → {} ✓", key, val);
        verified += 1;
    }

    let snapshot_size = cluster.replicas[cluster.primary_idx].as_ref().unwrap().snapshot().len();
    println!("\n✅ All {} keys verified correct after primary failover.", verified);
    println!("   (1000 rows in SQLite, snapshot={} bytes)", snapshot_size);
}
