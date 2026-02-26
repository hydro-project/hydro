/// Quiet Open-Loop Client for Comparison Tests
/// 
/// Same as client_openloop but with minimal stdout output.
/// Writes metrics to JSONL file for plotting.

use dfir_baseline::{Request, Response, MetricEvent};
use dfir_baseline::metrics::MetricsWriter;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
struct RequestState {
    sent_at: Instant,
    retry_count: u32,
}

#[derive(Debug, Clone)]
struct RatePhase {
    rate: f64,
    duration_secs: u64,
}

fn parse_rate_schedule(schedule_str: &str) -> Vec<RatePhase> {
    schedule_str
        .split(',')
        .filter_map(|phase| {
            let parts: Vec<&str> = phase.split(':').collect();
            if parts.len() == 2 {
                Some(RatePhase {
                    rate: parts[0].parse().ok()?,
                    duration_secs: parts[1].parse().ok()?,
                })
            } else {
                None
            }
        })
        .collect()
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let server_address = std::env::var("SERVER_ADDRESS")
        .unwrap_or_else(|_| "127.0.0.1:8080".to_string());
    let metrics_file = std::env::var("METRICS_FILE")
        .unwrap_or_else(|_| "/tmp/client_metrics.jsonl".to_string());
    let timeout_ms: f64 = std::env::var("TIMEOUT_MS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(100.0);
    let max_retries: u32 = std::env::var("MAX_RETRIES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3);

    let rate_schedule = std::env::var("RATE_SCHEDULE")
        .map(|s| parse_rate_schedule(&s))
        .unwrap_or_else(|_| vec![RatePhase { rate: 55.0, duration_secs: 30 }]);

    let total_duration_secs: u64 = rate_schedule.iter().map(|p| p.duration_secs).sum();

    // Connect
    let stream = TcpStream::connect(&server_address).await?;
    let (read_half, write_half) = stream.into_split();

    let pending_requests = Arc::new(Mutex::new(HashMap::<u64, RequestState>::new()));
    let metrics_writer = Arc::new(Mutex::new(MetricsWriter::new(&metrics_file)?));
    let (retry_tx, mut retry_rx) = mpsc::unbounded_channel::<u64>();
    
    let timeout_duration = Duration::from_secs_f64(timeout_ms / 1000.0);
    let end_time = Instant::now() + Duration::from_secs(total_duration_secs);

    // Sender task
    let pending_for_sender = pending_requests.clone();
    let metrics_for_sender = metrics_writer.clone();
    let sender_handle = tokio::spawn(async move {
        let mut write_half = write_half;
        let mut next_request_id = 0u64;
        
        let start_time = Instant::now();
        let mut phase_idx = 0;
        let mut phase_start_time = start_time;
        let mut current_rate = rate_schedule[0].rate;
        let mut request_interval = Duration::from_secs_f64(1.0 / current_rate);
        let mut next_send_time = start_time;

        loop {
            let now = Instant::now();
            if now >= end_time { break; }

            // Phase transition
            if phase_idx < rate_schedule.len() {
                let phase_elapsed = now.duration_since(phase_start_time).as_secs();
                if phase_elapsed >= rate_schedule[phase_idx].duration_secs {
                    phase_idx += 1;
                    if phase_idx < rate_schedule.len() {
                        phase_start_time = now;
                        current_rate = rate_schedule[phase_idx].rate;
                        request_interval = Duration::from_secs_f64(1.0 / current_rate);
                        next_send_time = now;
                    }
                }
            }

            // Handle retries
            while let Ok(req_id) = retry_rx.try_recv() {
                let request = Request::new(req_id);
                if write_half.write_all(&request.to_bytes()).await.is_err() { break; }
                
                let retry_count;
                {
                    let mut pending = pending_for_sender.lock().await;
                    if let Some(state) = pending.get_mut(&req_id) {
                        state.sent_at = now;
                        retry_count = state.retry_count;
                    } else {
                        retry_count = 0;
                    }
                }
                
                let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs_f64() * 1000.0;
                // Record as retry, not new request
                let _ = metrics_for_sender.lock().await.write_event(MetricEvent::RequestRetried {
                    timestamp: ts, req_id, retry_count,
                });
            }

            // Send new request
            if now >= next_send_time && phase_idx < rate_schedule.len() {
                let request = Request::new(next_request_id);
                if write_half.write_all(&request.to_bytes()).await.is_err() { break; }
                
                {
                    let mut pending = pending_for_sender.lock().await;
                    pending.insert(next_request_id, RequestState { sent_at: now, retry_count: 0 });
                }
                
                let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs_f64() * 1000.0;
                let _ = metrics_for_sender.lock().await.write_event(MetricEvent::RequestSent {
                    timestamp: ts, req_id: next_request_id,
                });
                
                next_request_id += 1;
                next_send_time += request_interval;
            }

            tokio::time::sleep(Duration::from_micros(100)).await;
        }
    });

    // Receiver task
    let pending_for_receiver = pending_requests.clone();
    let metrics_for_receiver = metrics_writer.clone();
    let receiver_handle = tokio::spawn(async move {
        let mut read_half = read_half;

        loop {
            let mut buf = [0u8; 9];
            match tokio::time::timeout(Duration::from_millis(100), read_half.read_exact(&mut buf)).await {
                Ok(Ok(_)) => {
                    let response = Response::from_bytes(buf);
                    let mut pending = pending_for_receiver.lock().await;
                    if let Some(state) = pending.remove(&response.id) {
                        let latency_ms = state.sent_at.elapsed().as_secs_f64() * 1000.0;
                        let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs_f64() * 1000.0;
                        
                        match response.status {
                            dfir_baseline::ResponseStatus::Success => {
                                let _ = metrics_for_receiver.lock().await.write_event(
                                    MetricEvent::ResponseReceived { timestamp: ts, req_id: response.id, latency_ms }
                                );
                            }
                            dfir_baseline::ResponseStatus::Rejected => {
                                let _ = metrics_for_receiver.lock().await.write_event(
                                    MetricEvent::RequestRejected { timestamp: ts, req_id: response.id }
                                );
                            }
                        }
                    }
                }
                Ok(Err(_)) => break,
                Err(_) => {
                    if Instant::now() >= end_time + Duration::from_secs(2) { break; }
                }
            }
        }
    });

    // Timeout checker
    let pending_for_timeout = pending_requests.clone();
    let metrics_for_timeout = metrics_writer.clone();
    let timeout_handle = tokio::spawn(async move {
        loop {
            if Instant::now() >= end_time + Duration::from_secs(2) { break; }
            tokio::time::sleep(Duration::from_millis(10)).await;

            let mut pending = pending_for_timeout.lock().await;
            let mut to_remove = Vec::new();

            for (req_id, state) in pending.iter_mut() {
                if state.sent_at.elapsed() >= timeout_duration {
                    let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs_f64() * 1000.0;
                    let _ = metrics_for_timeout.lock().await.write_event(
                        MetricEvent::RequestTimeout { timestamp: ts, req_id: *req_id }
                    );

                    if state.retry_count >= max_retries {
                        to_remove.push(*req_id);
                        let _ = metrics_for_timeout.lock().await.write_event(
                            MetricEvent::RequestFailed { timestamp: ts, req_id: *req_id }
                        );
                    } else {
                        state.retry_count += 1;
                        let _ = retry_tx.send(*req_id);
                    }
                }
            }

            for req_id in to_remove {
                pending.remove(&req_id);
            }
        }
    });

    let _ = sender_handle.await;
    let _ = receiver_handle.await;
    let _ = timeout_handle.await;
    metrics_writer.lock().await.flush()?;

    Ok(())
}
