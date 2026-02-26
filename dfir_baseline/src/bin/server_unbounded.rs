/// Single-Stage DFIR Server with UNBOUNDED Channel (No Backpressure)
/// 
/// This server has a single DFIR stage but uses an unbounded channel
/// between TCP ingress and the DFIR pipeline. This should collapse
/// under load because there's no backpressure mechanism.
/// 
/// Compare with server_single_stage.rs which has admission control.

use dfir_baseline::{Request, Response};
use dfir_rs::dfir_syntax;
use dfir_rs::tokio_stream::StreamExt;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::select;
use dfir_rs::tokio_stream::StreamMap;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

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
        eprintln!("UNBOUNDED server on {} (think={}ms)", server_address, think_time_ms);
    }

    let (request_tx, request_rx) = dfir_rs::util::unbounded_channel::<Request>();
    let (response_tx, response_rx) = tokio::sync::mpsc::unbounded_channel::<Response>();

    let think_time = Duration::from_millis(think_time_ms);
    
    // Single stage DFIR - NO admission control, unbounded channel
    let mut flow = dfir_syntax! {
        source_stream(request_rx)
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

    let bind_address = server_address.clone();
    tokio::spawn(async move {
        if let Err(e) = run_tcp_server(&bind_address, request_tx, response_rx, quiet).await {
            eprintln!("TCP error: {}", e);
        }
    });

    loop {
        flow.run_tick().await;
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}

async fn run_tcp_server(
    bind_address: &str,
    request_tx: dfir_rs::tokio::sync::mpsc::UnboundedSender<Request>,
    mut response_rx: tokio::sync::mpsc::UnboundedReceiver<Response>,
    quiet: bool,
) -> anyhow::Result<()> {
    let listener = TcpListener::bind(bind_address).await?;
    if !quiet {
        eprintln!("Listening on {}", listener.local_addr()?);
    }

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
                let (read_half, write_half) = stream.into_split();
                peers_send.insert(peer_addr, write_half);
                peers_recv.insert(peer_addr, PeerReadStream::new(read_half));
            }
            
            response = response_rx.recv() => {
                let Some(response) = response else { break; };
                let Some(peer_addr) = request_to_peer.remove(&response.id) else { continue; };
                let Some(write_half) = peers_send.get_mut(&peer_addr) else { continue; };
                let _ = write_half.write_all(&response.to_bytes()).await;
            }
            
            request = peers_recv.next(), if !peers_recv.is_empty() => {
                let Some((peer_addr, request_result)) = request else { continue; };
                match request_result {
                    Ok(request) => {
                        request_to_peer.insert(request.id, peer_addr);
                        // NO admission control - always accept
                        let _ = request_tx.send(request);
                    }
                    Err(_) => { peers_send.remove(&peer_addr); }
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
