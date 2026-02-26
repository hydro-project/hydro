/// Single-Stage DFIR Server (Control/Sanity Check)
/// 
/// This is a DFIR server with NO handoff buffers - just one stage.
/// TCP backpressure should prevent metastable collapse here.
/// 
/// If this server DOES NOT collapse with the same configuration that
/// causes the multi-stage server to collapse, we've proven the failure
/// is due to DFIR's handoff buffers, not our code or TCP.

use dfir_baseline::{BaselineConfig, MetricEvent, Request, Response};
use dfir_baseline::metrics::MetricsWriter;
use dfir_rs::dfir_syntax;
use dfir_rs::tokio_stream::StreamExt;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::net::TcpListener;
use tokio::select;
use dfir_rs::tokio_stream::StreamMap;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let server_address = std::env::var("SERVER_ADDRESS")
        .unwrap_or_else(|_| "127.0.0.1:8081".to_string());
    let think_time_ms = std::env::var("THINK_TIME_MS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);
    let max_queue_depth = std::env::var("MAX_QUEUE_DEPTH")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);
    let metrics_path = std::env::var("METRICS_PATH")
        .unwrap_or_else(|_| "/tmp/dfir_baseline_metrics/server_single_stage_metrics.jsonl".to_string());
    
    let config = BaselineConfig {
        server_address: server_address.clone(),
        think_time_ms,
        num_clients: 5,
        requests_per_second: 11.0,
        duration_secs: 30,
        ipc_directory: "/tmp/dfir_baseline_metrics".to_string(),
    };

    println!("Starting SINGLE-STAGE DFIR server (control) on {}", config.server_address);
    println!("Think time: {}ms", config.think_time_ms);
    println!("Expected capacity: {:.1} req/s", config.server_capacity());
    println!("Max queue depth: {}", max_queue_depth);
    println!("NO HANDOFF BUFFERS - single stage only");
    println!("Writing metrics to: {}", metrics_path);

    let (request_tx, request_rx) = dfir_rs::util::unbounded_channel::<Request>();
    let (response_tx, response_rx) = tokio::sync::mpsc::unbounded_channel::<Response>();

    // Track channel depth with two separate atomic counters
    // Depth = requests_sent - requests_received
    let requests_sent = Arc::new(AtomicUsize::new(0));
    let requests_received = Arc::new(AtomicUsize::new(0));
    let stale_responses = Arc::new(AtomicU64::new(0));
    let rejected_requests = Arc::new(AtomicU64::new(0));
    
    // Create ALL clones upfront before any usage
    let requests_received_for_flow = requests_received.clone();
    let requests_sent_for_tcp = requests_sent.clone();
    let requests_received_for_tcp = requests_received.clone();
    let stale_responses_for_tcp = stale_responses.clone();
    let rejected_requests_for_tcp = rejected_requests.clone();
    let requests_sent_for_main = requests_sent.clone();
    let requests_received_for_main = requests_received;
    let stale_responses_for_main = stale_responses;
    let rejected_requests_for_main = rejected_requests;

    // SINGLE STAGE - NO next_stratum() calls, NO handoff buffers
    let think_time = Duration::from_millis(config.think_time_ms);
    let mut flow = dfir_syntax! {
        source_stream(request_rx)
        -> inspect(|_| {
            // Increment received counter when DFIR pipeline receives from channel
            requests_received_for_flow.fetch_add(1, Ordering::Relaxed);
        })
        -> map(|request: Request| {
            async move {
                tokio::time::sleep(think_time).await;
                request
            }
        })
        -> resolve_futures_blocking_ordered()
        -> for_each(|request: Request| {
            let response = Response::new(request.id);
            let _ = response_tx.send(response);
        });
    };

    let metrics_handle = flow.metrics();
    let mut metrics_writer = MetricsWriter::new(&metrics_path)?;
    let mut last_sample_time = std::time::Instant::now();

    // Spawn TCP server with proper request/response handling
    let bind_address = config.server_address.clone();
    tokio::spawn(async move {
        if let Err(e) = run_tcp_server(&bind_address, request_tx, response_rx, requests_sent_for_tcp, max_queue_depth, requests_received_for_tcp, stale_responses_for_tcp, rejected_requests_for_tcp).await {
            eprintln!("TCP server error: {}", e);
        }
    });

    println!("Starting single-stage DFIR pipeline...");
    loop {
        flow.run_tick().await;
        
        if last_sample_time.elapsed() >= Duration::from_secs(1) {
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs_f64() * 1000.0;
            
            // Calculate channel depth: sent - received
            let sent = requests_sent_for_main.load(Ordering::Relaxed);
            let received = requests_received_for_main.load(Ordering::Relaxed);
            let depth = sent.saturating_sub(received);
            
            // Should be ZERO handoff buffers (no next_stratum() calls)
            let num_handoff_buffers = metrics_handle.handoffs.len();
            
            // Log both measurements
            println!("Channel depth: {} (sent: {}, received: {}), Handoff buffers: {}", 
                     depth, sent, received, num_handoff_buffers);
            
            // Log stale responses (reset counter each interval)
            let stale = stale_responses_for_main.swap(0, Ordering::Relaxed);
            if stale > 0 {
                println!("Stale responses this interval: {}", stale);
            }
            
            // Log rejected requests (reset counter each interval)
            let rejected = rejected_requests_for_main.swap(0, Ordering::Relaxed);
            if rejected > 0 {
                println!("Rejected requests this interval: {}", rejected);
            }
            
            metrics_writer.write_event(MetricEvent::BufferDepth {
                timestamp,
                buffer_id: 0,
                depth,
            })?;
            
            metrics_writer.flush()?;
            last_sample_time = std::time::Instant::now();
        }
        
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

/// TCP server following DFIR networking pattern with admission control
/// 
/// Single event loop handles ALL connections using select!:
/// - Accepts new connections
/// - Reads from any peer (non-blocking, multiplexed via StreamMap)
/// - Writes to any peer (non-blocking, triggered by pipeline responses)
/// - No head-of-line blocking - can read/write to different peers concurrently
/// - Rejects requests when queue depth exceeds MAX_QUEUE_DEPTH
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

    // Track write halves for each peer (to send responses)
    let mut peers_send: HashMap<SocketAddr, tokio::net::tcp::OwnedWriteHalf> = HashMap::new();
    
    // Track read halves for each peer (to receive requests)
    // StreamMap automatically removes streams when they close
    let mut peers_recv: StreamMap<SocketAddr, PeerReadStream> = StreamMap::new();
    
    // Track pending responses: request_id -> peer_addr
    // When we read a request, we remember which peer sent it
    // When pipeline sends response, we look up which peer to send it to
    let mut request_to_peer: HashMap<u64, SocketAddr> = HashMap::new();

    loop {
        select! {
            // Priority order: accept new connections, send responses, receive requests
            biased;
            
            // Accept new connections
            new_peer = listener.accept() => {
                let Ok((stream, peer_addr)): Result<(tokio::net::TcpStream, SocketAddr), _> = new_peer else {
                    continue;
                };
                println!("Accepted connection from {}", peer_addr);
                
                let (read_half, write_half) = stream.into_split();
                peers_send.insert(peer_addr, write_half);
                peers_recv.insert(peer_addr, PeerReadStream::new(read_half));
            }
            
            // Send responses from pipeline to clients
            response = response_rx.recv() => {
                let Some(response) = response else {
                    // Pipeline closed, shut down
                    break;
                };
                
                // Look up which peer this response goes to
                let Some(peer_addr) = request_to_peer.remove(&response.id) else {
                    // Count stale responses - expected during overload
                    stale_responses.fetch_add(1, Ordering::Relaxed);
                    continue;
                };
                
                let Some(write_half) = peers_send.get_mut(&peer_addr) else {
                    eprintln!("Peer {} disconnected before response could be sent", peer_addr);
                    continue;
                };
                
                // Send response
                let response_bytes = response.to_bytes();
                if let Err(e) = write_half.write_all(&response_bytes).await {
                    eprintln!("Error writing response to {}: {}", peer_addr, e);
                    peers_send.remove(&peer_addr);
                }
            }
            
            // Receive requests from clients
            request = peers_recv.next(), if !peers_recv.is_empty() => {
                let Some((peer_addr, request_result)) = request else {
                    continue;
                };
                
                match request_result {
                    Ok(request) => {
                        // Check queue depth before accepting request
                        let sent = requests_sent.load(Ordering::Relaxed);
                        let received = requests_received.load(Ordering::Relaxed);
                        let depth = sent.saturating_sub(received);
                        
                        if depth >= max_queue_depth {
                            // Server overloaded - reject request
                            let Some(write_half) = peers_send.get_mut(&peer_addr) else {
                                continue;
                            };
                            
                            let rejection = Response::rejected(request.id);
                            let rejection_bytes = rejection.to_bytes();
                            
                            if let Err(e) = write_half.write_all(&rejection_bytes).await {
                                eprintln!("Error writing rejection to {}: {}", peer_addr, e);
                                peers_send.remove(&peer_addr);
                            } else {
                                // Count rejections instead of logging each one
                                rejected_requests.fetch_add(1, Ordering::Relaxed);
                            }
                            continue;
                        }
                        
                        // Remember which peer sent this request
                        request_to_peer.insert(request.id, peer_addr);
                        
                        // Increment sent counter when TCP server sends to channel
                        requests_sent.fetch_add(1, Ordering::Relaxed);
                        
                        // Send to DFIR pipeline
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

/// Stream wrapper for reading requests from a TCP connection
struct PeerReadStream {
    read_half: tokio::net::tcp::OwnedReadHalf,
    buffer: [u8; 8],
}

impl PeerReadStream {
    fn new(read_half: tokio::net::tcp::OwnedReadHalf) -> Self {
        Self {
            read_half,
            buffer: [0u8; 8],
        }
    }
}

impl dfir_rs::futures::Stream for PeerReadStream {
    type Item = Result<Request, std::io::Error>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        use std::future::Future;
        
        // Get mutable references to the fields
        let this = self.get_mut();
        
        // Pin the future in place
        let read_fut = this.read_half.read_exact(&mut this.buffer);
        tokio::pin!(read_fut);
        
        match read_fut.poll(cx) {
            std::task::Poll::Ready(Ok(_bytes_read)) => {
                let request = Request::from_bytes(this.buffer);
                std::task::Poll::Ready(Some(Ok(request)))
            }
            std::task::Poll::Ready(Err(e)) => {
                if e.kind() == std::io::ErrorKind::UnexpectedEof {
                    // Connection closed cleanly
                    std::task::Poll::Ready(None)
                } else {
                    std::task::Poll::Ready(Some(Err(e)))
                }
            }
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
}
