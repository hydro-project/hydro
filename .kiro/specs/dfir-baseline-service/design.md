# Design Document: DFIR Baseline Service

## Overview

This design specifies a minimal DFIR-based request/response system for establishing a stable performance baseline at 55% utilization. The system consists of a simple DFIR server with a multi-stage pipeline and external client processes that generate configurable load. The design intentionally excludes retry logic, timeout handling, and metastability testing features to focus solely on baseline establishment.

The system uses TCP sockets for client-server communication (avoiding UDP buffer issues from previous implementations) and follows DFIR-faithful patterns observed in the Hydroflow ecosystem.

## Architecture

### System Components

```
┌─────────────┐         TCP          ┌──────────────────────────────┐
│   Client 1  │────────────────────▶│                              │
└─────────────┘                      │                              │
                                     │      DFIR Server             │
┌─────────────┐         TCP          │                              │
│   Client 2  │────────────────────▶│  ┌────────────────────────┐  │
└─────────────┘                      │  │  Stage 1: Receive      │  │
                                     │  └──────────┬─────────────┘  │
┌─────────────┐         TCP          │             │                │
│   Client N  │────────────────────▶│  ┌──────────▼─────────────┐  │
└─────────────┘                      │  │  Handoff Buffer        │  │
                                     │  └──────────┬─────────────┘  │
                                     │             │                │
                                     │  ┌──────────▼─────────────┐  │
                                     │  │  Stage 2: Process      │  │
                                     │  │  (with think time)     │  │
                                     │  └──────────┬─────────────┘  │
                                     │             │                │
                                     │  ┌──────────▼─────────────┐  │
                                     │  │  Handoff Buffer        │  │
                                     │  └──────────┬─────────────┘  │
                                     │             │                │
                                     │  ┌──────────▼─────────────┐  │
                                     │  │  Stage 3: Respond      │  │
                                     │  └────────────────────────┘  │
                                     │                              │
                                     └──────────────────────────────┘
```

### Process Model

- **Server Process**: Single OS process running the DFIR dataflow
- **Client Processes**: Multiple independent OS processes (N clients)
- **Communication**: TCP sockets (non-blocking I/O)
- **Metrics**: Shared file-based IPC for metrics collection

## Components and Interfaces

### 1. DFIR Server

**Responsibility**: Process requests through a multi-stage pipeline with handoff buffers.

**Implementation**:
```rust
// Pseudo-Rust showing DFIR structure
dfir_syntax! {
    // Stage 1: Receive requests from TCP connections
    source_stream(request_receiver)
    
    // Handoff Buffer 1: Explicit stratum boundary
    -> next_stratum()
    
    // Stage 2: Process with simulated think time
    -> map(|request| {
        tokio::time::sleep(think_time).await;
        request
    })
    
    // Handoff Buffer 2: Explicit stratum boundary  
    -> next_stratum()
    
    // Stage 3: Send responses back via TCP
    -> for_each(|request| {
        send_response(request.id);
    });
}
```

**Key Design Decisions**:
- Use `next_stratum()` to create explicit handoff buffers (unbounded queues)
- Use `tokio::time::sleep()` for think time (not `std::thread::sleep`)
- Non-blocking TCP I/O with manual polling loop
- Single-threaded tokio runtime (`new_current_thread()`)

**Interface**:
- **Input**: TCP connections on configurable address (e.g., `127.0.0.1:8080`)
- **Output**: TCP responses back to clients
- **Configuration**: Think time duration (milliseconds)

### 2. Client Process

**Responsibility**: Generate requests at a fixed rate and measure latency.

**Implementation Pattern** (based on existing DFIR client patterns):
```rust
struct ClientState {
    req_id_counter: u64,
    pending_requests: HashMap<u64, Instant>,  // ID -> sent_at
    next_send_time: Instant,
    request_interval: Duration,
    server_connection: TcpStream,
    metrics_writer: MetricsWriter,
}
```

**Behavior**:
1. Open TCP connection to server (non-blocking)
2. Send requests at fixed rate (open-loop generation)
3. Track pending requests with send timestamps
4. Receive responses and calculate latency
5. Write metrics to shared IPC location

**Key Design Decisions**:
- Open-loop request generation (not closed-loop)
- No retry logic (baseline only)
- No timeout handling (baseline only)
- Non-blocking I/O with 1ms sleep loop

**Interface**:
- **Input**: Configuration (rate, server address)
- **Output**: Metrics via IPC (latency, success/failure)

### 3. Request/Response Protocol

**Wire Format**:
```
Request:  [8 bytes: request_id (u64, big-endian)]
Response: [8 bytes: request_id (u64, big-endian)]
```

**Matching**: Clients match responses to requests by ID.

### 4. Metrics Collection

**Metrics Tracked**:
- Request sent timestamp
- Response received timestamp
- Latency (response_time - send_time)
- Success/failure status

**Storage Format** (JSON lines in IPC directory):
```json
{"timestamp": 1234567890.123, "type": "request_sent", "req_id": 42}
{"timestamp": 1234567890.145, "type": "response_received", "req_id": 42, "latency_ms": 22.0}
```

**Aggregation**:
- Compute p50 and p99 latencies from collected samples
- Compute success rate as (responses / requests) * 100
- Compute offered rate as requests per second

### 5. Configuration System

**Configuration Parameters**:
```rust
struct BaselineConfig {
    // Server configuration
    server_address: String,        // e.g., "127.0.0.1:8080"
    think_time_ms: u64,            // Processing delay per request
    
    // Client configuration
    num_clients: usize,            // Number of client processes
    requests_per_second: f64,      // Per-client request rate
    
    // Test duration
    duration_secs: u64,            // How long to run baseline test
    
    // IPC
    ipc_directory: String,         // Shared directory for metrics
}
```

**Tuning for 55% Utilization**:
- Server capacity = 1000 / think_time_ms requests/sec
- Offered load = num_clients * requests_per_second
- Target: offered_load = 0.55 * server_capacity

Example:
- think_time_ms = 10 → capacity = 100 req/s
- Target load = 55 req/s
- With 5 clients: 11 req/s per client

## Data Models

### Request
```rust
struct Request {
    id: u64,
}
```

### Response
```rust
struct Response {
    id: u64,
}
```

### MetricEvent
```rust
enum MetricEvent {
    RequestSent { timestamp: f64, req_id: u64 },
    ResponseReceived { timestamp: f64, req_id: u64, latency_ms: f64 },
}
```

### AggregatedMetrics
```rust
struct AggregatedMetrics {
    timestamp: f64,
    p50_latency_ms: f64,
    p99_latency_ms: f64,
    success_rate: f64,
    offered_rate: f64,
}
```

## Correctness Properties

*A property is a characteristic or behavior that should hold true across all valid executions of a system—essentially, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.*


### Property 1: Request-Response Round Trip
*For any* request sent by a client, the server should eventually return a response with the same request ID, and the response latency should be at least the configured think time.
**Validates: Requirements 1.3, 1.5, 7.2**

### Property 2: Unique Request Identifiers
*For any* set of requests generated by a client during a test run, all request IDs should be unique.
**Validates: Requirements 7.1**

### Property 3: Request Rate Accuracy
*For any* configured request rate R and measurement window W, the actual number of requests sent should be within 5% of (R × W).
**Validates: Requirements 2.3, 4.2**

### Property 4: Metrics Recording Completeness
*For any* request-response pair that completes, the metrics system should record both the latency and success status.
**Validates: Requirements 2.5, 2.6**

### Property 5: Success Rate Calculation
*For any* set of completed and failed requests, the calculated success rate should equal (completed / total) × 100.
**Validates: Requirements 3.3**

### Property 6: Offered Rate Without Amplification
*For any* baseline test run without retries, the recorded offered arrival rate should match the configured client rate within 5%.
**Validates: Requirements 3.4**

### Property 7: Metrics Timestamp Ordering
*For any* sequence of metric events from a single client, the timestamps should be monotonically increasing.
**Validates: Requirements 3.5, 3.6**

### Property 8: Think Time Proportionality
*For any* two configurations with think times T1 and T2 where T2 = 2×T1, the median latency with T2 should be approximately twice the median latency with T1.
**Validates: Requirements 4.3**

### Property 9: Utilization Range Stability
*For any* utilization level between 0% and 70%, the system should maintain a success rate above 95%.
**Validates: Requirements 4.5**

### Property 10: Baseline Stability
*For any* 30-second baseline test at 55% utilization, the success rate should exceed 99%, p50 latency should remain within 50% variance (CV < 0.50), and p99 latency should remain within 50% variance (CV < 0.50).
**Validates: Requirements 5.4, 5.5, 5.6**

### Property 11: Response Routing Correctness
*For any* multi-client system, responses should only be received by the client that sent the corresponding request.
**Validates: Requirements 7.5**

## Error Handling

### Server Error Handling

**TCP Connection Errors**:
- Non-blocking accept: Handle `WouldBlock` by continuing to next iteration
- Connection drops: Detect via read/write errors, remove from client list
- Bind failures: Exit with error message at startup

**Pipeline Errors**:
- Channel send failures: Log error and continue (client may have disconnected)
- Runtime errors: Panic and exit (unrecoverable)

**Configuration Errors**:
- Invalid think time: Exit with error at startup
- Invalid bind address: Exit with error at startup

### Client Error Handling

**Connection Errors**:
- Initial connection failure: Exit with error message
- Connection drops during operation: Exit with error message
- Write failures: Log and continue (may be transient)

**Metrics Errors**:
- IPC write failures: Log error and continue
- Flush failures: Log error and continue

**Configuration Errors**:
- Invalid rate: Exit with error at startup
- Invalid server address: Exit with error at startup

### Metrics Collection Error Handling

**File I/O Errors**:
- Cannot create metrics file: Exit with error
- Cannot write metrics: Log error and continue
- Cannot read metrics during aggregation: Skip corrupted entries

**Parsing Errors**:
- Invalid JSON in metrics file: Skip entry and continue
- Missing required fields: Skip entry and continue

## Testing Strategy

### Dual Testing Approach

This system requires both unit tests and property-based tests for comprehensive coverage:

**Unit Tests**: Focus on specific examples, edge cases, and integration points
- Server accepts TCP connections
- Client connects to server
- Request serialization/deserialization
- Metrics file creation and writing
- Configuration loading from files
- p50/p99 calculation with known datasets

**Property-Based Tests**: Verify universal properties across randomized inputs
- Use `proptest` crate for Rust property-based testing
- Minimum 100 iterations per property test
- Each test tagged with feature name and property number

### Property Test Configuration

All property tests must:
- Run for at least 100 iterations (due to randomization)
- Reference the design document property in a comment
- Use tag format: `// Feature: dfir-baseline-service, Property N: <property text>`
- Generate randomized inputs appropriate to the property

### Test Organization

**Unit Tests** (`tests/unit/`):
- `test_tcp_connection.rs`: TCP connection handling
- `test_protocol.rs`: Request/response serialization
- `test_metrics.rs`: Metrics recording and aggregation
- `test_config.rs`: Configuration loading and validation

**Property Tests** (`tests/properties/`):
- `prop_request_response.rs`: Property 1 (round trip)
- `prop_unique_ids.rs`: Property 2 (unique IDs)
- `prop_request_rate.rs`: Property 3 (rate accuracy)
- `prop_metrics_recording.rs`: Property 4 (metrics completeness)
- `prop_success_rate.rs`: Property 5 (success rate calculation)
- `prop_offered_rate.rs`: Property 6 (no amplification)
- `prop_timestamp_ordering.rs`: Property 7 (timestamp monotonicity)
- `prop_think_time.rs`: Property 8 (think time proportionality)
- `prop_utilization_stability.rs`: Property 9 (utilization range)
- `prop_response_routing.rs`: Property 11 (response routing)

**Integration Tests** (`tests/integration/`):
- `test_baseline_stability.rs`: Property 10 (baseline validation)
  - This is the most critical test
  - Must run full system (server + multiple clients)
  - Must validate 55% utilization baseline
  - Must verify 99%+ success rate and stable latencies
  - Should fail until proper configuration is found

### Testing Priority

1. **First**: Implement baseline stability integration test (Property 10)
   - This test drives the tuning process
   - Should fail initially, pass once baseline is established
2. **Second**: Implement request-response round trip (Property 1)
   - Validates basic functionality
3. **Third**: Implement remaining properties in any order

### Test Execution

Run all tests with:
```bash
cargo test --release
```

Run only property tests:
```bash
cargo test --release --test 'prop_*'
```

Run only integration tests:
```bash
cargo test --release --test 'test_*' --test-threads=1
```

Note: Integration tests should run serially (`--test-threads=1`) to avoid port conflicts.
