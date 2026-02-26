/// Simple three-phase metastability demonstration
/// 
/// Phase 1 (30s): Baseline at 55% utilization - should be stable
/// Phase 2 (15s): Trigger at 165% utilization (3× burst) - induces collapse
/// Phase 3 (90s): Recovery back to 55% - system should stay degraded
///
/// This test demonstrates that DFIR can enter a metastable failure state.
///
/// Uses dynamic rate scheduling so clients don't restart between phases.

use std::process::{Command, Stdio, Child};
use std::time::Duration;
use std::thread;
use tempfile::TempDir;

// Extension trait for thread timeout
trait JoinHandleExt<T> {
    fn join_timeout(self, timeout: Duration) -> Result<T, ()>;
}

impl<T> JoinHandleExt<T> for thread::JoinHandle<T> {
    fn join_timeout(self, timeout: Duration) -> Result<T, ()> {
        let start = std::time::Instant::now();
        loop {
            if self.is_finished() {
                return Ok(self.join().unwrap());
            }
            if start.elapsed() > timeout {
                return Err(());
            }
            thread::sleep(Duration::from_millis(100));
        }
    }
}

struct TestHarness {
    server: Child,
    clients: Vec<Child>,
    _metrics_dir: TempDir,
}

impl TestHarness {
    fn cleanup_zombie_servers() {
        println!("Cleaning up zombie servers...");
        let _ = Command::new("bash")
            .arg("dfir_baseline/kill_servers.sh")
            .output();
        thread::sleep(Duration::from_secs(1));
    }
    
    fn start_server(metrics_path: &std::path::Path) -> Child {
        println!("Starting single-stage server (control)...");
        Command::new("cargo")
            .args(&["run", "--package", "dfir_baseline", "--bin", "server_single_stage"])
            .env("SERVER_ADDRESS", "127.0.0.1:8080")
            .env("THINK_TIME_MS", "20") // 50 req/s capacity (validated baseline)
            .env("METRICS_PATH", metrics_path.to_str().unwrap())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("Failed to start server")
    }
    
    fn start_clients_with_schedule(
        num_clients: usize,
        rate_schedule: &str,
        timeout_ms: f64,
        metrics_dir: &std::path::Path,
    ) -> Vec<Child> {
        println!("Starting {} clients with rate schedule: {}", num_clients, rate_schedule);
        println!("Timeout: {}ms", timeout_ms);
        
        // Also write to persistent location
        let persistent_path = std::path::PathBuf::from("/tmp/metastability_correct_baseline");
        
        let mut clients = Vec::new();
        for i in 0..num_clients {
            let persistent_file = persistent_path.join(format!("client_{}.jsonl", i));
            
            // Add small delay between client starts to avoid connection storms
            if i > 0 {
                thread::sleep(Duration::from_millis(200));
            }
            
            let client = Command::new("cargo")
                .args(&["run", "--package", "dfir_baseline", "--bin", "client_openloop"])
                .env("SERVER_ADDRESS", "127.0.0.1:8080")
                .env("RATE_SCHEDULE", rate_schedule)
                .env("TIMEOUT_MS", timeout_ms.to_string())
                .env("MAX_RETRIES", "3") // Steering default: 3 retries
                .env("METRICS_FILE", persistent_file.to_str().unwrap())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .expect("Failed to start client");
            clients.push(client);
        }
        clients
    }
    
    fn new() -> Self {
        Self::cleanup_zombie_servers();
        
        let metrics_dir = TempDir::new().unwrap();
        
        // Clean and recreate persistent location for analysis
        let persistent_path = std::path::PathBuf::from("/tmp/metastability_correct_baseline");
        // Remove directory if it exists (force, no prompt)
        if persistent_path.exists() {
            let _ = std::fs::remove_dir_all(&persistent_path);
        }
        let _ = std::fs::create_dir_all(&persistent_path);
        println!("Metrics will be saved to: {:?}", persistent_path);
        
        let server_metrics_path = persistent_path.join("server_metrics.jsonl");
        let server = Self::start_server(&server_metrics_path);
        thread::sleep(Duration::from_secs(2)); // Wait for server to start
        
        Self {
            server,
            clients: Vec::new(),
            _metrics_dir: metrics_dir,
        }
    }
}

impl Drop for TestHarness {
    fn drop(&mut self) {
        println!("\nCleaning up...");
        for mut client in self.clients.drain(..) {
            let _ = client.kill();
            let _ = client.wait();
        }
        let _ = self.server.kill();
        let _ = self.server.wait();
    }
}

#[test]
#[ignore] // Run with: cargo test --package dfir_baseline --test metastability_demo -- --ignored --nocapture
fn test_three_phase_metastability_demonstration() {
    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║  DFIR Single-Stage Server Metastability Test              ║");
    println!("║  (Control: No handoff buffers, only input channel)        ║");
    println!("╚════════════════════════════════════════════════════════════╝");
    
    // CRITICAL: Spawn test execution in a thread with timeout to prevent hanging
    // If server gets stuck, the test MUST NOT hang indefinitely
    let test_handle = thread::spawn(|| {
        run_metastability_test()
    });
    
    // Wait for test with timeout (total_duration + 60s grace period)
    let timeout = Duration::from_secs(255 + 60); // 315 seconds total
    match test_handle.join_timeout(timeout) {
        Ok(Ok(())) => {
            println!("\n✓ Test completed successfully");
        }
        Ok(Err(e)) => {
            panic!("Test failed: {}", e);
        }
        Err(_) => {
            // Timeout - kill everything
            println!("\n✗ TEST TIMEOUT - Killing all processes");
            TestHarness::cleanup_zombie_servers();
            panic!("Test exceeded timeout of {:?}", timeout);
        }
    }
}

fn run_metastability_test() -> Result<(), String> {
    let mut harness = TestHarness::new();
    
    // Server capacity: 1000ms / 20ms = 50 req/s (validated baseline)
    // Single client configuration - much simpler!
    // Baseline: 27.5 req/s = 55% utilization
    // Trigger: 82.5 req/s = 165% utilization (3× burst)
    
    // Build rate schedule for all three phases
    // Phase 1: 30s at 27.5 req/s (baseline @ 55%)
    // Phase 2: 45s at 82.5 req/s (trigger @ 165% = 3× burst)
    // Phase 3: 180s at 27.5 req/s (recovery @ 55%, 2× longer)
    let rate_schedule = "27.5:30,82.5:45,27.5:180";
    let total_duration = 30 + 45 + 180; // 255 seconds
    
    println!("\n=== Single Client Configuration ===");
    println!("Phase 1: 27.5 req/s for 30s (baseline @ 55%)");
    println!("Phase 2: 82.5 req/s for 45s (trigger @ 165% = 3× burst)");
    println!("Phase 3: 27.5 req/s for 180s (recovery @ 55%, 2× longer)");
    println!("Total duration: {}s", total_duration);
    
    // Start single client with dynamic rate schedule
    harness.clients = TestHarness::start_clients_with_schedule(
        1,              // Single client - much simpler!
        rate_schedule,  // rate schedule
        70.0,           // timeout_ms (3× validated p50 latency)
        harness._metrics_dir.path(),
    );
    
    // Wait for all phases to complete
    println!("\nRunning experiment for {}s...", total_duration);
    thread::sleep(Duration::from_secs(total_duration));
    
    // Wait for clients to finish (with timeout to prevent hanging)
    println!("Waiting for clients to finish...");
    for mut client in harness.clients.drain(..) {
        // Give client 5 seconds to exit gracefully, then kill it
        thread::sleep(Duration::from_secs(5));
        let _ = client.kill();
        let _ = client.wait();
    }
    
    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║  Single-Stage Server Experiment Complete                  ║");
    println!("╚════════════════════════════════════════════════════════════╝");
    println!("\nMetrics saved to: /tmp/metastability_correct_baseline/");
    println!("\nTo analyze results:");
    println!("  cargo run --package dfir_baseline --bin plot_metrics -- /tmp/metastability_correct_baseline 1");
    println!("\nExpected: Single-stage server should NOT exhibit metastable collapse");
    println!("          TCP backpressure should prevent unbounded channel growth");
    
    Ok(())
}
