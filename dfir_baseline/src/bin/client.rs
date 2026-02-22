/// DFIR Baseline Client
/// 
/// A simple TCP client that sends requests at a fixed rate
/// and measures latency. No retry logic or timeout handling
/// (baseline only).

use dfir_baseline::{Request, Response, MetricEvent};
use dfir_baseline::metrics::MetricsWriter;
use std::collections::HashMap;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Read configuration from environment variables or use defaults
    let server_address = std::env::var("SERVER_ADDRESS")
        .unwrap_or_else(|_| "127.0.0.1:8080".to_string());
    let requests_per_second = std::env::var("REQUESTS_PER_SECOND")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(11.0);
    let duration_secs = std::env::var("DURATION_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(30);
    let metrics_file = std::env::var("METRICS_FILE")
        .unwrap_or_else(|_| "/tmp/dfir_baseline_client.jsonl".to_string());

    println!("Starting DFIR baseline client");
    println!("Server: {}", server_address);
    println!("Rate: {} req/s", requests_per_second);
    println!("Duration: {}s", duration_secs);
    println!("Metrics file: {}", metrics_file);

    // Create metrics writer
    let mut metrics_writer = MetricsWriter::new(&metrics_file)?;

    // Connect to server
    let mut stream = TcpStream::connect(&server_address).await?;
    println!("Connected to server");

    // Calculate request interval
    let request_interval = Duration::from_secs_f64(1.0 / requests_per_second);
    
    let mut next_request_id = 0u64;
    let mut next_send_time = Instant::now();
    let start_time = Instant::now();
    let end_time = start_time + Duration::from_secs(duration_secs);
    
    let mut pending_requests: HashMap<u64, Instant> = HashMap::new();
    let mut latencies: Vec<f64> = Vec::new();
    let mut total_sent = 0u64;
    let mut total_received = 0u64;

    loop {
        let now = Instant::now();
        
        // Check if we should stop
        if now >= end_time {
            break;
        }
        
        // Send new request if it's time
        if now >= next_send_time {
            let request = Request::new(next_request_id);
            let request_bytes = request.to_bytes();
            
            stream.write_all(&request_bytes).await?;
            pending_requests.insert(next_request_id, now);
            total_sent += 1;
            
            // Record request sent metric
            let timestamp_ms = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs_f64() * 1000.0;
            metrics_writer.write_event(MetricEvent::RequestSent {
                timestamp: timestamp_ms,
                req_id: next_request_id,
            })?;
            
            println!("Sent request {}", next_request_id);
            next_request_id += 1;
            next_send_time += request_interval;
        }
        
        // Try to read response (non-blocking)
        let mut response_buffer = [0u8; 8];
        match tokio::time::timeout(Duration::from_millis(1), stream.read_exact(&mut response_buffer)).await {
            Ok(Ok(_)) => {
                let response = Response::from_bytes(response_buffer);
                
                if let Some(sent_at) = pending_requests.remove(&response.id) {
                    let latency_ms = sent_at.elapsed().as_secs_f64() * 1000.0;
                    latencies.push(latency_ms);
                    total_received += 1;
                    
                    // Record response received metric
                    let timestamp_ms = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs_f64() * 1000.0;
                    metrics_writer.write_event(MetricEvent::ResponseReceived {
                        timestamp: timestamp_ms,
                        req_id: response.id,
                        latency_ms,
                    })?;
                    
                    println!("Received response {} (latency: {:.1}ms)", response.id, latency_ms);
                }
            }
            Ok(Err(e)) => {
                eprintln!("Error reading response: {}", e);
                break;
            }
            Err(_) => {
                // Timeout - no response available yet, continue
            }
        }
        
        // Small sleep to avoid busy-waiting
        tokio::time::sleep(Duration::from_micros(100)).await;
    }
    
    // Wait a bit for in-flight requests to complete
    println!("\nWaiting for in-flight requests...");
    let wait_until = Instant::now() + Duration::from_secs(2);
    while Instant::now() < wait_until && !pending_requests.is_empty() {
        let mut response_buffer = [0u8; 8];
        match tokio::time::timeout(Duration::from_millis(100), stream.read_exact(&mut response_buffer)).await {
            Ok(Ok(_)) => {
                let response = Response::from_bytes(response_buffer);
                
                if let Some(sent_at) = pending_requests.remove(&response.id) {
                    let latency_ms = sent_at.elapsed().as_secs_f64() * 1000.0;
                    latencies.push(latency_ms);
                    total_received += 1;
                    
                    // Record response received metric
                    let timestamp_ms = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs_f64() * 1000.0;
                    metrics_writer.write_event(MetricEvent::ResponseReceived {
                        timestamp: timestamp_ms,
                        req_id: response.id,
                        latency_ms,
                    })?;
                    
                    println!("Received response {} (latency: {:.1}ms)", response.id, latency_ms);
                }
            }
            Ok(Err(_)) => break,
            Err(_) => continue,
        }
    }
    
    // Flush metrics to disk
    metrics_writer.flush()?;
    
    // Print final metrics
    println!("\n=== Client Metrics ===");
    println!("Total sent: {}", total_sent);
    println!("Total received: {}", total_received);
    println!("Success rate: {:.1}%", (total_received as f64 / total_sent as f64) * 100.0);
    
    if !latencies.is_empty() {
        latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let p50_idx = (latencies.len() as f64 * 0.5) as usize;
        let p99_idx = (latencies.len() as f64 * 0.99) as usize;
        println!("Latency p50: {:.1}ms", latencies[p50_idx]);
        println!("Latency p99: {:.1}ms", latencies[p99_idx]);
    }

    Ok(())
}
