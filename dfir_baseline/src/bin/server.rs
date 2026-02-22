/// DFIR Baseline Server
/// 
/// A simple TCP server that processes requests through a DFIR pipeline
/// with explicit handoff buffers between stages.

use dfir_baseline::pipeline::{DfirPipeline, PipelineConfig};
use dfir_baseline::{BaselineConfig, Request, Response};
use std::collections::HashMap;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    // Read configuration from environment variables or use defaults
    let server_address = std::env::var("SERVER_ADDRESS")
        .unwrap_or_else(|_| "127.0.0.1:8080".to_string());
    let think_time_ms = std::env::var("THINK_TIME_MS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);
    
    let config = BaselineConfig {
        server_address: server_address.clone(),
        think_time_ms,
        num_clients: 5,
        requests_per_second: 11.0,
        duration_secs: 30,
        ipc_directory: "/tmp/dfir_baseline_metrics".to_string(),
    };

    println!("Starting DFIR baseline server on {}", config.server_address);
    println!("Think time: {}ms", config.think_time_ms);
    println!("Expected capacity: {:.1} req/s", config.server_capacity());

    // Create channels for DFIR pipeline
    let (request_tx, request_rx) = dfir_rs::util::unbounded_channel::<Request>();
    let (response_tx, mut response_rx) = dfir_rs::util::unbounded_channel::<Response>();

    // Build DFIR pipeline
    let pipeline_config = PipelineConfig::new(config.think_time_ms);
    let pipeline = DfirPipeline::new(pipeline_config);
    
    let mut response_senders = HashMap::new();
    response_senders.insert(0, response_tx);
    
    let mut flow = pipeline.build_flow(request_rx, response_senders);

    // Spawn TCP server
    let bind_address = config.server_address.clone();
    tokio::spawn(async move {
        if let Err(e) = run_tcp_server(&bind_address, request_tx).await {
            eprintln!("TCP server error: {}", e);
        }
    });

    // Spawn response handler (for now, just drain responses)
    tokio::spawn(async move {
        use dfir_rs::tokio_stream::StreamExt;
        while let Some(_response) = response_rx.next().await {
            // Responses are handled by the TCP connection handlers
        }
    });

    // Run DFIR pipeline
    flow.run().await;

    Ok(())
}

async fn run_tcp_server(
    bind_address: &str,
    request_tx: dfir_rs::tokio::sync::mpsc::UnboundedSender<Request>,
) -> anyhow::Result<()> {
    let listener = TcpListener::bind(bind_address).await?;
    let local_addr = listener.local_addr()?;
    println!("Server listening on {}", local_addr);

    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                println!("Accepted connection from {}", addr);
                let request_tx_clone = request_tx.clone();
                
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(stream, request_tx_clone).await {
                        eprintln!("Error handling connection from {}: {}", addr, e);
                    }
                });
            }
            Err(e) => {
                eprintln!("Error accepting connection: {}", e);
            }
        }
    }
}

async fn handle_connection(
    mut stream: TcpStream,
    request_tx: dfir_rs::tokio::sync::mpsc::UnboundedSender<Request>,
) -> anyhow::Result<()> {
    let peer_addr = stream.peer_addr()?;
    println!("Handling connection from {}", peer_addr);

    let mut buffer = [0u8; 8];

    loop {
        // Read 8-byte request
        match stream.read_exact(&mut buffer).await {
            Ok(_) => {
                let request = Request::from_bytes(buffer);
                println!("Received request {}", request.id);

                // Send to DFIR pipeline
                if request_tx.send(request).is_err() {
                    eprintln!("Failed to send request to pipeline");
                    break;
                }

                // For now, send response immediately
                // TODO: Properly route responses from pipeline
                let response = Response::new(request.id);
                let response_bytes = response.to_bytes();
                
                if let Err(e) = stream.write_all(&response_bytes).await {
                    eprintln!("Error writing response: {}", e);
                    break;
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                println!("Client {} disconnected", peer_addr);
                break;
            }
            Err(e) => {
                eprintln!("Error reading from {}: {}", peer_addr, e);
                break;
            }
        }
    }

    Ok(())
}
