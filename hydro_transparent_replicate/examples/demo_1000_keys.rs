//! Demo: write 1000 keys with random values, crash primary, read back and verify.
//!
//! Run with: cargo run -p hydro_transparent_replicate --features backend_redb --example demo_1000_keys

use hydro_transparent_replicate::backends::redb::{RedbCommand, RedbResponse, RedbService};
use hydro_transparent_replicate::ReplicableService;
use std::collections::HashMap;

struct Cluster {
    replicas: Vec<Option<RedbService>>,
    primary_idx: usize,
}

impl Cluster {
    fn new(n: usize) -> Self {
        Self {
            replicas: (0..n).map(|_| Some(RedbService::default())).collect(),
            primary_idx: 0,
        }
    }

    fn put(&mut self, key: &[u8], value: &[u8]) {
        let cmd = RedbCommand::Put { key: key.to_vec(), value: value.to_vec() };
        for r in self.replicas.iter_mut().flatten() {
            r.apply(cmd.clone());
        }
    }

    fn get(&mut self, key: &[u8]) -> Option<Vec<u8>> {
        let cmd = RedbCommand::Get { key: key.to_vec() };
        let r = self.replicas[self.primary_idx].as_mut().unwrap();
        match r.apply(cmd) {
            RedbResponse::Value(v) => v,
            _ => panic!("unexpected response"),
        }
    }

    fn crash_primary(&mut self) {
        self.replicas[self.primary_idx] = None;
    }

    fn failover(&mut self) {
        let new = self.replicas.iter().position(|r| r.is_some()).unwrap();
        let snapshot = self.replicas[new].as_ref().unwrap().snapshot();
        let mut fresh = RedbService::default();
        fresh.restore(&snapshot);
        self.replicas[new] = Some(fresh);
        self.primary_idx = new;
    }
}

fn main() {
    // Generate 1000 key-value pairs with random values.
    let mut rng_state: u64 = 0xdeadbeef;
    let mut expected: HashMap<String, String> = HashMap::new();

    let mut cluster = Cluster::new(3);

    println!("Writing 1000 keys...");
    for i in 0..1000 {
        // Simple xorshift PRNG (deterministic, no external deps).
        rng_state ^= rng_state << 13;
        rng_state ^= rng_state >> 7;
        rng_state ^= rng_state << 17;
        let value = (rng_state % 1_000_000).to_string();
        let key = format!("k{}", i);

        cluster.put(key.as_bytes(), value.as_bytes());
        expected.insert(key, value);
    }
    println!("  Done. 1000 keys written to 3 replicas.");

    // Verify a few reads before crash.
    println!("\nSpot-check before crash:");
    for i in [0, 42, 500, 999] {
        let key = format!("k{}", i);
        let val = cluster.get(key.as_bytes()).unwrap();
        let val_str = String::from_utf8(val).unwrap();
        assert_eq!(&val_str, expected.get(&key).unwrap());
        println!("  GET {} → {} ✓", key, val_str);
    }

    // Crash primary.
    println!("\n💥 CRASH: Primary (replica 0) is dead.");
    cluster.crash_primary();

    // Failover.
    println!("🔄 FAILOVER: Electing new primary via snapshot/restore...");
    cluster.failover();
    println!("  New primary: replica {}", cluster.primary_idx);

    // Read back 20 random keys and verify.
    println!("\nVerifying 20 keys after failover:");
    let mut verified = 0;
    for i in (0..1000).step_by(50) {
        let key = format!("k{}", i);
        let val = cluster.get(key.as_bytes()).unwrap();
        let val_str = String::from_utf8(val).unwrap();
        let expected_val = expected.get(&key).unwrap();
        assert_eq!(&val_str, expected_val, "MISMATCH on {}: got {}, expected {}", key, val_str, expected_val);
        println!("  GET {} → {} ✓", key, val_str);
        verified += 1;
    }

    println!("\n✅ All {} keys verified correct after primary failover.", verified);
    println!("   (1000 keys written, snapshot={} bytes)", cluster.replicas[cluster.primary_idx].as_ref().unwrap().snapshot().len());
}
