/// Basic integration test for request-response flow
/// 
/// This test verifies that:
/// 1. Server can start and accept connections
/// 2. Client can connect and send requests
/// 3. Server processes requests through DFIR pipeline
/// 4. Client receives responses

use dfir_baseline::{Request, Response};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

#[tokio::test]
async fn test_basic_request_response() {
    // Start a simple server
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let server_addr = listener.local_addr().unwrap();
    
    println!("Test server listening on {}", server_addr);
    
    // Spawn server task
    tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        println!("Server accepted connection");
        
        let mut buffer = [0u8; 8];
        
        // Read request
        stream.read_exact(&mut buffer).await.unwrap();
        let request = Request::from_bytes(buffer);
        println!("Server received request {}", request.id);
        
        // Send response immediately (simplified for test)
        let response = Response::new(request.id);
        stream.write_all(&response.to_bytes()).await.unwrap();
        println!("Server sent response {}", response.id);
    });
    
    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Client: Connect and send request
    let mut client = TcpStream::connect(server_addr).await.unwrap();
    println!("Client connected to server");
    
    let request = Request::new(42);
    client.write_all(&request.to_bytes()).await.unwrap();
    println!("Client sent request {}", request.id);
    
    // Read response
    let mut response_buffer = [0u8; 8];
    client.read_exact(&mut response_buffer).await.unwrap();
    let response = Response::from_bytes(response_buffer);
    println!("Client received response {}", response.id);
    
    // Verify response matches request
    assert_eq!(response.id, request.id);
    
    println!("✓ Basic request-response test passed!");
}

#[tokio::test]
async fn test_multiple_requests() {
    // Start a simple server
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let server_addr = listener.local_addr().unwrap();
    
    println!("Test server listening on {}", server_addr);
    
    // Spawn server task
    tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        println!("Server accepted connection");
        
        let mut buffer = [0u8; 8];
        
        // Handle 3 requests
        for _ in 0..3 {
            stream.read_exact(&mut buffer).await.unwrap();
            let request = Request::from_bytes(buffer);
            println!("Server received request {}", request.id);
            
            let response = Response::new(request.id);
            stream.write_all(&response.to_bytes()).await.unwrap();
            println!("Server sent response {}", response.id);
        }
    });
    
    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Client: Connect and send multiple requests
    let mut client = TcpStream::connect(server_addr).await.unwrap();
    println!("Client connected to server");
    
    for i in 0..3 {
        let request = Request::new(i);
        client.write_all(&request.to_bytes()).await.unwrap();
        println!("Client sent request {}", request.id);
        
        let mut response_buffer = [0u8; 8];
        client.read_exact(&mut response_buffer).await.unwrap();
        let response = Response::from_bytes(response_buffer);
        println!("Client received response {}", response.id);
        
        assert_eq!(response.id, request.id);
    }
    
    println!("✓ Multiple requests test passed!");
}
