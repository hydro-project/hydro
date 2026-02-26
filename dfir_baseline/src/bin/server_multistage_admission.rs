/// Multi-Stage DFIR Server with Admission Control
/// 
/// This server combines:
/// - The 3-stage DFIR pipeline with handoff buffers (from server.rs)
/// - Admission control at TCP ingress (from server_single_stage.rs)
/// 
/// This demonstrates that admission control can prevent metastable collapse
/// even when the underlying pipeline has unbounded handoff buffers.

use dfir_baseline::pipeline::{DfirPipeline, PipelineConfig};
use dfir_baseline::{BaselineConfig, MetricEvent, Request, Response};
use dfir_baseline::metrics::MetricsWriter;
use dfir_rs::tokio_stream::StreamExt;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::select;
use dfir_rs::tokio_stream::StreamMap;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let server_address = std::env::var("SERVER_ADDRESS")
        .unwrap_or_else(|_| "127.0.0.1:8084".to_string());
    let think_time_ms = std::env::var("THINK_TIME_MS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);
    let max_queue_depth = std::env::var("MAX_QUEUE_DEPTH")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);
    let metrics_path = std::env::var("METRICS_PATH")
        .unwrap_or_else(|_| "/tmp/dfir_baseline_metrics/server_multistage_admission_metrics.jsonl".to_string());
    
    let config = BaselineConfig {
        server_address: server_address.clone(),
        think_time_ms,
        num_clients: 5,
        requests_per_second: 11.0,
        duration_secs: 30,
        ipc_directory: "/tmp/dfir_baseline_metrics".to_string(),
    };

    println!("Starting MULTI-STAGE DFIR server with ADMISSION CONTROL on {}", config.server_address);
    println!("Think time: {}ms", config.think_time_ms);
    println!("Expected capacity: {:.1} req/s", config.server_capacity());
    println!("Max queue depth: {}", max_queue_depth);
    println!("3-stage pipeline WITH handoff buffers + admission control at ingress");
    println!("Writing metrics to: {}", metrics_path);

    // Create channels connecting TCP <-> DFIR pipeline
    let (request_tx, request_rx) = dfir_rs::util::unbounded_channel::<Request>();
    let (response_tx, response_rx) = tokio::sync::mpsc::unbounded_channel::<Response>();

    // Track queue depth with atomic counters
    let requests_sent = Arc::new(AtomicUsize::new(0));
    let requests_received = Arc::new(AtomicUsize::new(0));
    let stale_responses = Arc::new(AtomicU64::new(0));
    let rejected_requests = Arc::new(AtomicU64::new(0));
    
    // Clone for different tasks
    let requests_sent_for_tcp = requests_sent.clone();
    let requests_received_for_tcp = requests_received.clone();
    let stale_responses_for_tcp = stale_responses.clone();
    let rejected_requests_for_tcp = rejected_requests.clone();
    let requests_sent_for_main = requests_sent.clone();
    let requests_received_for_main = requests_received;
    let stale_responses_for_main = stale_responses;
    let rejected_requests_for_main = rejected_requests;

    // Build 3-stage DFIR pipeline (same as server.rs)
    let pipeline_config = PipelineConfig::new(config.think_time_ms);
    let pipeline = DfirPipeline::new(pipeline_config);
    
    let mut response_senders = HashMap::new();
    response_senders.insert(0, response_tx);
    
    let mut flow = pipeline.build_flow(request_rx, response_senders);

    // Get metrics handle for buffer depth sampling
    let metrics_handle = flow.metrics();
    let mut metrics_writer = MetricsWriter::new(&metrics_path)?;
    let mut last_sample_time = std::time::Instant::now();

    // Spawn TCP server with admission control
    let bind_address = config.server_address.clone();
    tokio::spawn(async move {
        if let Err(e) = run_tcp_server(
            &bind_address, 
            request_tx, 
            response_rx, 
            requests_sent_for_tcp, 
            max_queue_depth, 
            requests_received_for_tcp, 
            stale_responses_for_tcp, 
            rejected_requests_for_tcp
        ).await {
            eprintln!("TCP server error: {}", e);
        }
    });

    println!("Starting 3-stage DFIR pipeline with admission control...");
    loop {
        flow.run_tick().await;
        
        // Increment received counter based on pipeline progress
        // We track this by checking handoff buffer changes
        
        if last_sample_time.elapsed() >= Duration::from_secs(1) {
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs_f64() * 1000.0;
            
            // Calculate ingress queue depth
            let sent = requests_sent_for_main.load(Ordering::Relaxed);
            let received = requests_received_for_main.load(Ordering::Relaxed);
            let ingress_depth = sent.saturating_sub(received);
            
            // Sample handoff buffer depths (should have 2 buffers from next_stratum() calls)
            let num_handoff_buffers = metrics_handle.handoffs.len();
            let mut total_buffer_depth = 0;
            
            for (buffer_id, (_handoff_id, handoff_metrics)) in metrics_handle.handoffs.iter().enumerate() {
                let depth = handoff_metrics.curr_items_count();
                total_buffer_depth += depth;
                
                metrics_writer.write_event(MetricEvent::BufferDepth {
                    timestamp,
                    buffer_id,
                    depth,
                })?;
            }
            
            println!("Ingress depth: {}, Handoff buffers: {} (total depth: {})", 
                     ingress_depth, num_handoff_buffers, total_buffer_depth);
            
            // Log stale responses
            let stale = stale_responses_for_main.swap(0, Ordering::Relaxed);
            if stale > 0 {
                println!("Stale responses this interval: {}", stale);
            }
            
            // Log rejected requests
            let rejected = rejected_requests_for_main.swap(0, Ordering::Relaxed);
            if rejected > 0 {
                println!("Rejected requests this interval: {}", rejected);
            }
            
            metrics_writer.flush()?;
            last_sample_time = std::time::Instant::now();
        }
        
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

/// TCP server with admission control (same as server_single_stage.rs)
async fn run_tcp_server(
    bind_address: &str,
    request_tx: dfir_rs::tokio::sync::mpsc::UnboundedSender<Request>,
    mut response_rx: tokio::sync::mpsc::UnboundedReceiver<Response>,
    requests_sent: Arc<AtomicUsize>,
    max_queue_depth: usize,
    requests_received: Arc<AtomicUsize>,
    stale_responses: Arc<AtomicU64>,
    rejected_requests: Arc<AtomicU64>,
) -> anyhow::Result<()> {
    let listener = TcpListener::bind(bind_address).await?;
    let local_addr = listener.local_addr()?;
    println!("Server listening on {}", local_addr);

    let mut peers_send: HashMap<SocketAddr, tokio::net::tcp::OwnedWriteHalf> = HashMap::new();
    let mut peers_recv: StreamMap<SocketAddr, PeerReadStream> = StreamMap::new();
    let mut request_to_peer: HashMap<u64, SocketAddr> = HashMap::new();

    loop {
        select! {
            biased;
            
            new_peer = listener.accept() => {
                let Ok((stream, peer_addr)): Result<(tokio::net::TcpStream, SocketAddr), _> = new_peer else {
                    continue;
                };
                println!("Accepted connection from {}", peer_addr);
                
                let (read_half, write_half) = stream.into_split();
                peers_send.insert(peer_addr, write_half);
                peers_recv.insert(peer_addr, PeerReadStream::new(read_half));
            }
            
            response = response_rx.recv() => {
                let Some(response) = response else {
                    break;
                };
                
                // Track that pipeline completed a request
                requests_received.fetch_add(1, Ordering::Relaxed);
                
                let Some(peer_addr) = request_to_peer.remove(&response.id) else {
                    stale_responses.fetch_add(1, Ordering::Relaxed);
                    continue;
                };
                
                let Some(write_half) = peers_send.get_mut(&peer_addr) else {
                    continue;
                };
                
                let response_bytes = response.to_bytes();
                if let Err(e) = write_half.write_all(&response_bytes).await {
                    eprintln!("Error writing response to {}: {}", peer_addr, e);
                    peers_send.remove(&peer_addr);
                }
            }
            
            request = peers_recv.next(), if !peers_recv.is_empty() => {
                let Some((peer_addr, request_result)) = request else {
                    continue;
                };
                
                match request_result {
                    Ok(request) => {
                        // ADMISSION CONTROL: Check queue depth before accepting
                        let sent = requests_sent.load(Ordering::Relaxed);
                        let received = requests_received.load(Ordering::Relaxed);
                        let depth = sent.saturating_sub(received);
                        
                        if depth >= max_queue_depth {
                            // Reject request - server overloaded
                            let Some(write_half) = peers_send.get_mut(&peer_addr) else {
                                continue;
                            };
                            
                            let rejection = Response::rejected(request.id);
                            let rejection_bytes = rejection.to_bytes();
                            
                            if let Err(e) = write_half.write_all(&rejection_bytes).await {
                                eprintln!("Error writing rejection to {}: {}", peer_addr, e);
                                peers_send.remove(&peer_addr);
                            } else {
                                rejected_requests.fetch_add(1, Ordering::Relaxed);
                            }
                            continue;
                        }
                        
                        // Accept request
                        request_to_peer.insert(request.id, peer_addr);
                        requests_sent.fetch_add(1, Ordering::Relaxed);
                        
                        if request_tx.send(request).is_err() {
                            eprintln!("Failed to send request to pipeline");
                            break;
                        }
                    }
                    Err(e) => {
                        eprintln!("Error reading from {}: {}", peer_addr, e);
                        peers_send.remove(&peer_addr);
                    }
                }
            }
        }
    }

    Ok(())
}

struct PeerReadStream {
    read_half: tokio::net::tcp::OwnedReadHalf,
    buffer: [u8; 8],
}

impl PeerReadStream {
    fn new(read_half: tokio::net::tcp::OwnedReadHalf) -> Self {
        Self { read_half, buffer: [0u8; 8] }
    }
}

impl dfir_rs::futures::Stream for PeerReadStream {
    type Item = Result<Request, std::io::Error>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        use std::future::Future;
        let this = self.get_mut();
        let read_fut = this.read_half.read_exact(&mut this.buffer);
        tokio::pin!(read_fut);
        
        match read_fut.poll(cx) {
            std::task::Poll::Ready(Ok(_)) => {
                std::task::Poll::Ready(Some(Ok(Request::from_bytes(this.buffer))))
            }
            std::task::Poll::Ready(Err(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                std::task::Poll::Ready(None)
            }
            std::task::Poll::Ready(Err(e)) => std::task::Poll::Ready(Some(Err(e))),
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
}
