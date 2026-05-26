//! Interactive demo: primary/backup replication with redb backend.
//!
//! Simulates a 3-replica cluster (1 primary + 2 backups) using the actual
//! RedbService with real snapshot/restore. Demonstrates write durability
//! across primary failover.
//!
//! Run with: cargo run -p hydro_transparent_replicate --features backend_redb --example demo_failover

use hydro_transparent_replicate::backends::redb::{RedbCommand, RedbResponse, RedbService};
use hydro_transparent_replicate::ReplicableService;

/// A simulated replica in the cluster.
struct Replica {
    id: usize,
    service: RedbService,
    alive: bool,
}

/// A simulated 3-replica cluster with primary/backup replication.
struct Cluster {
    replicas: Vec<Replica>,
    primary_idx: usize,
}

impl Cluster {
    fn new() -> Self {
        let replicas = (0..3)
            .map(|id| Replica {
                id,
                service: RedbService::default(),
                alive: true,
            })
            .collect();
        println!("╔══════════════════════════════════════════════════════════╗");
        println!("║  Transparent Replication Demo (redb backend)            ║");
        println!("║  3 replicas: [0]=Primary, [1]=Backup, [2]=Backup       ║");
        println!("╚══════════════════════════════════════════════════════════╝");
        println!();
        Self {
            replicas,
            primary_idx: 0,
        }
    }

    fn status(&self) {
        for r in &self.replicas {
            let role = if r.id == self.primary_idx {
                "PRIMARY"
            } else {
                "backup "
            };
            let state = if r.alive { "alive" } else { "DEAD " };
            println!("  Replica {}: [{}] {}", r.id, state, role);
        }
        println!();
    }

    /// Write a key-value pair. Replicates to all alive replicas.
    fn put(&mut self, key: &str, value: &str) {
        let cmd = RedbCommand::Put {
            key: key.as_bytes().to_vec(),
            value: value.as_bytes().to_vec(),
        };

        print!("  PUT {}={} → ", key, value);

        // Primary must be alive to accept writes.
        if !self.replicas[self.primary_idx].alive {
            println!("ERROR: primary is dead!");
            return;
        }

        // Apply on all alive replicas (simulates replicate + ack + commit).
        for r in self.replicas.iter_mut() {
            if r.alive {
                r.service.apply(cmd.clone());
            }
        }
        println!("committed (replicated to {} replicas)", self.replicas.iter().filter(|r| r.alive).count());
    }

    /// Read a key from the current primary.
    fn get(&mut self, key: &str) {
        let cmd = RedbCommand::Get {
            key: key.as_bytes().to_vec(),
        };

        // Find an alive replica to read from (prefer primary).
        let reader_idx = if self.replicas[self.primary_idx].alive {
            self.primary_idx
        } else {
            // Fallback to any alive replica.
            self.replicas.iter().position(|r| r.alive).unwrap_or(0)
        };

        let resp = self.replicas[reader_idx].service.apply(cmd);
        match resp {
            RedbResponse::Value(Some(v)) => {
                println!("  GET {} → \"{}\" (from replica {})", key, String::from_utf8_lossy(&v), reader_idx);
            }
            RedbResponse::Value(None) => {
                println!("  GET {} → (not found) (from replica {})", key, reader_idx);
            }
            other => println!("  GET {} → unexpected: {:?}", key, other),
        }
    }

    /// Kill the primary. Simulates a crash.
    fn crash_primary(&mut self) {
        let old = self.primary_idx;
        println!("  💥 CRASH: Replica {} (primary) is dead!", old);
        self.replicas[old].alive = false;
        println!();
    }

    /// Elect a new primary from surviving replicas via snapshot-based state transfer.
    fn failover(&mut self) {
        // Find the first alive replica that isn't the old primary.
        let new_primary = self
            .replicas
            .iter()
            .position(|r| r.alive && r.id != self.primary_idx)
            .expect("no alive replicas to failover to!");

        println!("  🔄 FAILOVER: Electing replica {} as new primary...", new_primary);

        // Take snapshot from the new primary (it was a backup with up-to-date state).
        let snapshot = self.replicas[new_primary].service.snapshot();
        println!("     Snapshot taken from replica {} ({} bytes)", new_primary, snapshot.len());

        // In a real system, the new primary would restore from a peer's snapshot.
        // Here the new primary already has the state (it was applying as a backup).
        // But let's demonstrate restore anyway by creating a fresh instance and restoring:
        let mut fresh = RedbService::default();
        fresh.restore(&snapshot);
        self.replicas[new_primary].service = fresh;
        println!("     State restored on new primary (replica {})", new_primary);

        self.primary_idx = new_primary;
        println!("  ✅ Replica {} is now PRIMARY", new_primary);
        println!();
    }
}

fn main() {
    let mut cluster = Cluster::new();
    cluster.status();

    // Step 1: Write x=1 and y=7
    println!("── Step 1: Write x=1 and y=7 ──────────────────────────────");
    cluster.put("x", "1");
    cluster.put("y", "7");
    println!();

    // Step 2: Read y
    println!("── Step 2: Read y ─────────────────────────────────────────");
    cluster.get("y");
    println!();

    // Step 3: Crash the primary
    println!("── Step 3: Crash the primary ──────────────────────────────");
    cluster.crash_primary();
    cluster.status();

    // Step 4: Failover to a new primary
    println!("── Step 4: Failover ───────────────────────────────────────");
    cluster.failover();
    cluster.status();

    // Step 5: Read x and y from the new primary
    println!("── Step 5: Read x and y (from new primary) ────────────────");
    cluster.get("x");
    cluster.get("y");
    println!();

    println!("── Done! Writes survived primary failure. ─────────────────");
}
