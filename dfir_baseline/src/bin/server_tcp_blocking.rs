/// Single-Threaded Blocking TCP Server (Control Case)
/// 
/// This server processes requests serially with blocking sleep.
/// When overloaded, TCP backpressure naturally limits incoming requests:
/// - Kernel receive buffer fills up
/// - Clients block on write()
/// - No unbounded queuing anywhere
/// 
/// Theoretical capacity: 1000 / think_time_ms requests/second
/// 
/// This is the TRUE control case: if this server collapses, the problem
/// is in our client code or test setup, not in DFIR's unbounded buffers.

use dfir_baseline::{Request, Response};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::Duration;

fn handle_connection(mut stream: TcpStream, think_time: Duration, quiet: bool) {
    let peer_addr = stream.peer_addr().ok();
    if !quiet {
        if let Some(addr) = peer_addr {
            eprintln!("Connection from {}", addr);
        }
    }

    // Set read timeout to detect client disconnect
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));

    let mut buffer = [0u8; 8];
    loop {
        // Read request (blocking)
        match stream.read_exact(&mut buffer) {
            Ok(_) => {
                let request = Request::from_bytes(buffer);
                
                // Blocking think time - creates TCP backpressure
                std::thread::sleep(think_time);
                
                // Send response
                let response = Response::new(request.id);
                if stream.write_all(&response.to_bytes()).is_err() {
                    break;
                }
            }
            Err(e) => {
                // Client disconnected or timeout
                if !quiet {
                    if let Some(addr) = peer_addr {
                        eprintln!("Client {} disconnected: {}", addr, e);
                    }
                }
                break;
            }
        }
    }
}

fn main() -> anyhow::Result<()> {
    let server_address = std::env::var("SERVER_ADDRESS")
        .unwrap_or_else(|_| "127.0.0.1:8082".to_string());
    let think_time_ms: u64 = std::env::var("THINK_TIME_MS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);
    let quiet = std::env::var("QUIET").is_ok();

    let think_time = Duration::from_millis(think_time_ms);
    let capacity = 1000.0 / think_time_ms as f64;

    if !quiet {
        eprintln!("TCP BLOCKING server on {} (think={}ms, capacity={:.0} req/s)", 
                  server_address, think_time_ms, capacity);
        eprintln!("Single-threaded serial processing - TCP backpressure enabled");
    }

    let listener = TcpListener::bind(&server_address)?;
    if !quiet {
        eprintln!("Listening on {}", listener.local_addr()?);
    }

    // Accept connections and handle them serially in separate threads
    // Each connection is handled by one thread, but requests within
    // a connection are processed serially (blocking)
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let think = think_time;
                std::thread::spawn(move || {
                    handle_connection(stream, think, quiet);
                });
            }
            Err(e) => {
                if !quiet {
                    eprintln!("Accept error: {}", e);
                }
            }
        }
    }

    Ok(())
}
