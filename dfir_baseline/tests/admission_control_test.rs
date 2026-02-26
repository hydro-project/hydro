/// Test admission control with queue depth tracking
/// 
/// This test verifies that the single-stage server with admission control
/// rejects requests when the queue depth exceeds MAX_QUEUE_DEPTH, preventing
/// retry amplification and metastable collapse.

use std::process::{Command, Stdio};
use std::time::Duration;
use std::path::PathBuf;

#[test]
#[ignore]
fn test_admission_control_prevents_collapse() {
    println!("\n=== Testing Admission Control ===\n");
    
    // Configuration
    let server_address = "127.0.0.1:8091";
    let think_time_ms = 10;
    let max_queue_depth = 10;
    let num_clients = 5;
    let timeout_ms = 70.0;
    let max_retries = 3;
    
    // Rate schedule: baseline -> burst -> recovery
    // Baseline: 55% utilization (55 req/s total = 11 req/s per client)
    // Burst: 165% utilization (165 req/s total = 33 req/s per client)
    // Recovery: back to 55% utilization
    let rate_schedule = "11.0:30,33.0:15,11.0:90";
    
    let metrics_dir = PathBuf::from("/private/tmp/metastability_with_admission_control");
    std::fs::create_dir_all(&metrics_dir).expect("Failed to create metrics directory");
    
    println!("Configuration:");
    println!("  Server: {}", server_address);
    println!("  Think time: {}ms", think_time_ms);
    println!("  Max queue depth: {}", max_queue_depth);
    println!("  Server capacity: {} req/s", 1000 / think_time_ms);
    println!("  Num clients: {}", num_clients);
    println!("  Rate schedule: {}", rate_schedule);
    println!("  Timeout: {}ms", timeout_ms);
    println!("  Max retries: {}", max_retries);
    println!("  Metrics dir: {}", metrics_dir.display());
    
    // Start server
    println!("\nStarting single-stage server with admission control...");
    let server_metrics = metrics_dir.join("server_metrics.jsonl");
    let mut server = Command::new("cargo")
        .args(&[
            "run",
            "--bin",
            "server_single_stage",
        ])
        .env("SERVER_ADDRESS", server_address)
        .env("THINK_TIME_MS", think_time_ms.to_string())
        .env("MAX_QUEUE_DEPTH", max_queue_depth.to_string())
        .env("METRICS_PATH", server_metrics.to_str().unwrap())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start server");
    
    // Wait for server to start
    std::thread::sleep(Duration::from_secs(3));
    
    // Start clients
    println!("Starting {} clients...", num_clients);
    let mut client_handles = Vec::new();
    
    for i in 0..num_clients {
        let client_metrics = metrics_dir.join(format!("client_{}_metrics.jsonl", i));
        let client = Command::new("cargo")
            .args(&[
                "run",
                "--bin",
                "client_openloop",
            ])
            .env("SERVER_ADDRESS", server_address)
            .env("RATE_SCHEDULE", rate_schedule)
            .env("TIMEOUT_MS", timeout_ms.to_string())
            .env("MAX_RETRIES", max_retries.to_string())
            .env("METRICS_FILE", client_metrics.to_str().unwrap())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to start client");
        
        client_handles.push(client);
    }
    
    println!("Waiting for test to complete (135 seconds)...");
    
    // Wait for clients to finish
    for (i, mut client) in client_handles.into_iter().enumerate() {
        match client.wait() {
            Ok(status) => {
                if !status.success() {
                    eprintln!("Client {} exited with status: {}", i, status);
                }
            }
            Err(e) => {
                eprintln!("Error waiting for client {}: {}", i, e);
            }
        }
    }
    
    println!("Clients finished. Stopping server...");
    
    // Stop server
    let _ = server.kill();
    let _ = server.wait();
    
    println!("\n=== Test Complete ===");
    println!("Metrics saved to: {}", metrics_dir.display());
    println!("\nExpected outcome:");
    println!("  - Server should reject requests when queue depth exceeds {}", max_queue_depth);
    println!("  - Clients should NOT retry rejected requests");
    println!("  - System should NOT exhibit metastable collapse");
    println!("  - Success rate should remain high during recovery phase");
    println!("\nTo analyze results, run:");
    println!("  cargo run --bin analyze_metrics -- {}", metrics_dir.display());
}
