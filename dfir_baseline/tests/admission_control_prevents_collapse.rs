/// Test that admission control prevents metastable collapse
/// 
/// This test demonstrates that server-side admission control successfully
/// prevents metastable failures by rejecting requests when queue depth
/// exceeds a threshold, rather than accepting unbounded work.

use std::process::{Command, Stdio};
use std::time::Duration;
use std::fs;
use std::path::Path;

#[test]
fn test_admission_control_prevents_collapse() {
    // Clean up any previous test artifacts
    let metrics_dir = "/tmp/dfir_admission_control_test";
    let _ = fs::remove_dir_all(metrics_dir);
    fs::create_dir_all(metrics_dir).expect("Failed to create metrics directory");

    // Configuration matching the metastability demo
    let server_address = "127.0.0.1:8082";
    let think_time_ms = 10;
    let max_queue_depth = 10;
    let timeout_ms = 70.0;
    let max_retries = 3;
    
    // Rate schedule: baseline -> trigger -> recovery
    // Baseline: 5.5 req/s for 30s (55% utilization)
    // Trigger: 16.5 req/s for 15s (165% utilization - 3x burst)
    // Recovery: 5.5 req/s for 90s (55% utilization)
    let rate_schedule = "5.5:30,16.5:15,5.5:90";
    
    println!("\n=== Testing Admission Control ===");
    println!("Server: single-stage DFIR with admission control");
    println!("Max queue depth: {}", max_queue_depth);
    println!("Rate schedule: {}", rate_schedule);
    println!("Expected: System should NOT collapse (rejections prevent overload)");
    
    // Start server with admission control
    let server_metrics = format!("{}/server_metrics.jsonl", metrics_dir);
    let mut server = Command::new("cargo")
        .args(&["run", "--bin", "server_single_stage"])
        .env("SERVER_ADDRESS", server_address)
        .env("THINK_TIME_MS", think_time_ms.to_string())
        .env("MAX_QUEUE_DEPTH", max_queue_depth.to_string())
        .env("METRICS_PATH", &server_metrics)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start server");
    
    // Wait for server to start
    std::thread::sleep(Duration::from_secs(2));
    
    // Start client
    let client_metrics = format!("{}/client_metrics.jsonl", metrics_dir);
    let client_output = Command::new("cargo")
        .args(&["run", "--bin", "client_openloop"])
        .env("SERVER_ADDRESS", server_address)
        .env("RATE_SCHEDULE", rate_schedule)
        .env("TIMEOUT_MS", timeout_ms.to_string())
        .env("MAX_RETRIES", max_retries.to_string())
        .env("METRICS_FILE", &client_metrics)
        .output()
        .expect("Failed to run client");
    
    println!("\n=== Client Output ===");
    println!("{}", String::from_utf8_lossy(&client_output.stdout));
    
    if !client_output.status.success() {
        eprintln!("Client stderr: {}", String::from_utf8_lossy(&client_output.stderr));
    }
    
    // Kill server
    server.kill().expect("Failed to kill server");
    let _ = server.wait();
    
    // Analyze metrics to verify no collapse
    println!("\n=== Analyzing Metrics ===");
    
    let analysis_output = Command::new("cargo")
        .args(&["run", "--bin", "analyze_metrics"])
        .arg(&client_metrics)
        .output()
        .expect("Failed to run analysis");
    
    println!("{}", String::from_utf8_lossy(&analysis_output.stdout));
    
    // Generate plot
    let plot_path = format!("{}/admission_control_test.png", metrics_dir);
    let plot_output = Command::new("cargo")
        .args(&["run", "--bin", "plot_metrics"])
        .arg(&client_metrics)
        .arg(&plot_path)
        .output()
        .expect("Failed to generate plot");
    
    if plot_output.status.success() {
        println!("\n=== Plot Generated ===");
        println!("Absolute path: {}", fs::canonicalize(&plot_path).unwrap().display());
    } else {
        eprintln!("Plot generation failed: {}", String::from_utf8_lossy(&plot_output.stderr));
    }
    
    // Verify success: parse the analysis output to check recovery phase success rate
    let analysis_text = String::from_utf8_lossy(&analysis_output.stdout);
    
    // Look for recovery phase success rate
    let mut recovery_success_rate = 0.0;
    for line in analysis_text.lines() {
        if line.contains("Recovery phase") && line.contains("success rate") {
            // Extract percentage from line like "Recovery phase (90-135s): success rate 95.2%"
            if let Some(pct_str) = line.split("success rate").nth(1) {
                if let Some(num_str) = pct_str.trim().strip_suffix('%') {
                    recovery_success_rate = num_str.parse().unwrap_or(0.0);
                }
            }
        }
    }
    
    println!("\n=== Test Results ===");
    println!("Recovery phase success rate: {:.1}%", recovery_success_rate);
    
    // With admission control, success rate should remain high (>80%) during recovery
    // This proves the system did NOT enter a metastable state
    assert!(
        recovery_success_rate > 80.0,
        "Expected recovery success rate >80% with admission control, got {:.1}%",
        recovery_success_rate
    );
    
    println!("✓ Admission control successfully prevented metastable collapse!");
    println!("✓ System recovered after trigger phase (success rate: {:.1}%)", recovery_success_rate);
}
