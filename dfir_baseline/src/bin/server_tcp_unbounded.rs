/// Pure TCP Server with UNBOUNDED Internal Queue (No Backpressure)
/// 
/// This server uses tokio async TCP but has an unbounded internal queue
/// between TCP ingress and the worker. This removes TCP backpressure
/// and should collapse under load, but may have nonzero success rate
/// (unlike DFIR unbounded which has 0% recovery).
/// 
/// Compare with:
/// - server_sync_rust.rs: blocking TCP with natural backpressure (recovers)
/// - server_unbounded.rs: DFIR with unbounded channel (collapses to 0%)

use dfir_baseline::{Request, Response};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::select;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let server_address = std::env::var("SERVER_ADDRESS")
        .unwrap_or_else(|_| "127.0.0.1:8080".to_string());
    let think_time_ms: u64 = std::env::var("THINK_TIME_MS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);
    let quiet = std::env::var("QUIET").is_ok();

    if !quiet {
        eprintln!("TCP UNBOUNDED server on {} (think={}ms)", server_address, think_time_ms);
    }

    // Unbounded channel - NO backpressure
    let (request_tx, mut request_rx) = mpsc::unbounded_channel::<(Request, SocketAddr)>();
    let (response_tx, mut response_rx) = mpsc::unbounded_channel::<(Response, SocketAddr)>();

    let think_time = Duration::from_millis(think_time_ms);
    
    // Worker task - processes requests from unbounded queue
    let worker_response_tx = response_tx.clone();
    tokio::spawn(async move {
        while let Some((request, peer_addr)) = request_rx.recv().await {
            let tx = worker_response_tx.clone();
            // Spawn a task per request to allow concurrent processing
            tokio::spawn(async move {
                tokio::time::sleep(think_time).await;
                let response = Response::new(request.id);
                let _ = tx.send((response, peer_addr));
            });
        }
    });

    // TCP server
    let listener = TcpListener::bind(&server_address).await?;
    if !quiet {
        eprintln!("Listening on {}", listener.local_addr()?);
    }

    let mut peers_send: HashMap<SocketAddr, tokio::net::tcp::OwnedWriteHalf> = HashMap::new();

    // Spawn reader tasks for each connection
    let (read_tx, mut read_rx) = mpsc::unbounded_channel::<(SocketAddr, Result<Request, ()>)>();

    loop {
        select! {
            biased;
            
            new_peer = listener.accept() => {
                let Ok((stream, peer_addr)) = new_peer else { continue; };
                let (read_half, write_half) = stream.into_split();
                peers_send.insert(peer_addr, write_half);
                
                // Spawn reader task
                let tx = read_tx.clone();
                tokio::spawn(async move {
                    let mut read_half = read_half;
                    let mut buffer = [0u8; 8];
                    loop {
                        match read_half.read_exact(&mut buffer).await {
                            Ok(_) => {
                                let request = Request::from_bytes(buffer);
                                if tx.send((peer_addr, Ok(request))).is_err() {
                                    break;
                                }
                            }
                            Err(_) => {
                                let _ = tx.send((peer_addr, Err(())));
                                break;
                            }
                        }
                    }
                });
            }
            
            response = response_rx.recv() => {
                let Some((response, peer_addr)) = response else { break; };
                if let Some(write_half) = peers_send.get_mut(&peer_addr) {
                    let _ = write_half.write_all(&response.to_bytes()).await;
                }
            }
            
            read_result = read_rx.recv() => {
                let Some((peer_addr, result)) = read_result else { continue; };
                match result {
                    Ok(request) => {
                        // NO admission control - always accept into unbounded queue
                        let _ = request_tx.send((request, peer_addr));
                    }
                    Err(_) => {
                        peers_send.remove(&peer_addr);
                    }
                }
            }
        }
    }
    Ok(())
}
