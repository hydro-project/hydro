/// Test to verify retry mechanism works when timeouts are triggered
/// This validates that the retry logic is functional before we attempt
/// to demonstrate metastability.

use std::process::{Command, Stdio};
use std::time::Duration;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
#[ignore] // Run with: cargo test --package dfir_baseline --test retry_mechanism -- --ignored --nocapture
fn test_retry_mechanism_triggers_with_low_timeout() {
    println!("\n=== Testing Retry Mechanism ===");
    println!("This test verifies retries trigger when timeout is set very low");
    
    // Create temp directory for metrics
    let temp_dir = TempDir::new().unwrap();
    let metrics_dir = temp_dir.path().to_path_buf();
    
    // Start server
    println!("\nStarting server...");
    let mut server = Command::new("cargo")
        .args(&["run", "--package", "dfir_baseline", "--bin", "server"])
        .env("SERVER_ADDRESS", "127.0.0.1:8080")
        .env("THINK_TIME_MS", "20") // 20ms think time
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to start server");
    
    // Wait for server to start
    std::thread::sleep(Duration::from_secs(2));
    
    // Start single client with VERY LOW timeout to trigger retries
    println!("Starting client with 0.1ms timeout (should trigger retries)...");
    let metrics_file = metrics_dir.join("client_0.jsonl");
    let client = Command::new("cargo")
        .args(&["run", "--package", "dfir_baseline", "--bin", "client"])
        .env("SERVER_ADDRESS", "127.0.0.1:8080")
        .env("REQUESTS_PER_SECOND", "5.5")
        .env("DURATION_SECS", "10")
        .env("TIMEOUT_MS", "0.1") // Very low timeout - should trigger retries
        .env("MAX_RETRIES", "3")
        .env("METRICS_FILE", metrics_file.to_str().unwrap())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("Failed to run client");
    
    println!("\nClient output:");
    println!("{}", String::from_utf8_lossy(&client.stdout));
    
    // Stop server
    println!("\nStopping server...");
    server.kill().expect("Failed to kill server");
    server.wait().expect("Failed to wait for server");
    
    // Read metrics and verify retries occurred
    println!("\nAnalyzing metrics...");
    let metrics_content = fs::read_to_string(&metrics_file)
        .expect("Failed to read metrics file");
    
    let mut retry_count = 0;
    let mut timeout_count = 0;
    let mut failed_count = 0;
    
    for line in metrics_content.lines() {
        if line.contains("request_retried") {
            retry_count += 1;
        }
        if line.contains("request_timeout") {
            timeout_count += 1;
        }
        if line.contains("request_failed") {
            failed_count += 1;
        }
    }
    
    println!("\n=== Results ===");
    println!("Retries: {}", retry_count);
    println!("Timeouts: {}", timeout_count);
    println!("Failed: {}", failed_count);
    
    // Verify that retries were triggered
    assert!(retry_count > 0, "Expected retries to be triggered with 0.1ms timeout, but got 0");
    assert!(timeout_count > 0, "Expected timeouts to occur with 0.1ms timeout, but got 0");
    
    println!("\n✓ Retry mechanism is working correctly!");
}
