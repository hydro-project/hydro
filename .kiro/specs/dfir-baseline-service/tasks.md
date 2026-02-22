# Implementation Plan: DFIR Baseline Service

## Overview

This plan implements a minimal DFIR baseline service for establishing stable 55% utilization. The implementation focuses on a simple multi-stage DFIR server with TCP-based clients, metrics collection, and baseline validation testing. The system intentionally excludes retry logic and timeout handling to focus solely on baseline establishment.

## Tasks

- [x] 1. Set up project structure and core types
  - Create new Rust project or module within existing workspace
  - Define Request and Response types with serialization
  - Define MetricEvent and AggregatedMetrics types
  - Define BaselineConfig structure
  - Set up dependencies: tokio, serde, serde_json, dfir_rs
  - _Requirements: 7.1, 7.2_

- [x] 2. Implement DFIR pipeline with handoff buffers
  - [x] 2.1 Create DfirPipeline struct with configuration
    - Implement three-stage pipeline using dfir_syntax!
    - Stage 1: source_stream for receiving requests
    - Use next_stratum() to create first handoff buffer
    - Stage 2: map with tokio::time::sleep() for think time
    - Use next_stratum() to create second handoff buffer
    - Stage 3: for_each to send responses
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 6.1, 6.2, 6.3, 6.4, 6.5, 6.6_

  - [ ]* 2.2 Write property test for request-response round trip
    - **Property 1: Request-Response Round Trip**
    - **Validates: Requirements 1.3, 1.5, 7.2**

  - [ ]* 2.3 Write property test for think time proportionality
    - **Property 8: Think Time Proportionality**
    - **Validates: Requirements 4.3**

- [x] 3. Implement DFIR server process
  - [x] 3.1 Create server binary with TCP listener
    - Bind to configurable address (non-blocking)
    - Accept client connections in main loop
    - Read requests from all connected clients
    - Send requests to DFIR pipeline via channel
    - Receive responses from pipeline
    - Write responses back to clients via TCP
    - Use single-threaded tokio runtime
    - _Requirements: 1.5, 1.6_

  - [ ]* 3.2 Write unit tests for TCP connection handling
    - Test server accepts connections
    - Test server reads request data
    - Test server writes response data
    - _Requirements: 1.6_

- [x] 4. Implement client process
  - [x] 4.1 Create client binary with request generation
    - Connect to server via TCP (non-blocking)
    - Generate requests at fixed rate (open-loop)
    - Track pending requests with send timestamps
    - Read responses from server
    - Match responses to requests by ID
    - Calculate latency for each request-response pair
    - No retry logic (baseline only)
    - No timeout handling (baseline only)
    - _Requirements: 2.3, 2.4, 2.5, 2.6, 7.3, 7.4_

  - [ ]* 4.2 Write property test for unique request identifiers
    - **Property 2: Unique Request Identifiers**
    - **Validates: Requirements 7.1**

  - [ ]* 4.3 Write property test for request rate accuracy
    - **Property 3: Request Rate Accuracy**
    - **Validates: Requirements 2.3, 4.2**

- [x] 5. Checkpoint - Ensure basic request-response works
  - Ensure all tests pass, ask the user if questions arise.

- [x] 6. Implement metrics collection system
  - [x] 6.1 Create MetricsWriter for IPC
    - Write metric events to JSON lines file
    - Include timestamps for all events
    - Support request_sent and response_received events
    - Flush periodically to disk
    - _Requirements: 2.5, 2.6, 3.6_

  - [x] 6.2 Create MetricsReader for aggregation
    - Read metric events from IPC files
    - Calculate p50 and p99 latencies
    - Calculate success rate
    - Calculate offered arrival rate
    - Export as time-series data
    - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5_

  - [ ]* 6.3 Write property test for metrics recording completeness
    - **Property 4: Metrics Recording Completeness**
    - **Validates: Requirements 2.5, 2.6**

  - [ ]* 6.4 Write property test for success rate calculation
    - **Property 5: Success Rate Calculation**
    - **Validates: Requirements 3.3**

  - [ ]* 6.5 Write property test for offered rate without amplification
    - **Property 6: Offered Rate Without Amplification**
    - **Validates: Requirements 3.4**

  - [ ]* 6.6 Write property test for timestamp ordering
    - **Property 7: Metrics Timestamp Ordering**
    - **Validates: Requirements 3.5, 3.6**

  - [ ]* 6.7 Write unit tests for p50/p99 calculation
    - Test with known datasets
    - Test edge cases (empty, single value, etc.)
    - _Requirements: 3.1, 3.2_

- [x] 7. Implement configuration system
  - [x] 7.1 Create BaselineConfig with file loading
    - Support JSON configuration files
    - Include server_address, think_time_ms, num_clients, requests_per_second, duration_secs, ipc_directory
    - Validate configuration parameters
    - Calculate target utilization
    - _Requirements: 4.1, 4.2, 4.3, 4.4_

  - [ ]* 7.2 Write unit tests for configuration loading
    - Test valid configuration files
    - Test invalid configurations
    - Test utilization calculations
    - _Requirements: 4.1, 4.2, 4.3_

- [x] 8. Checkpoint - Ensure metrics and configuration work
  - Ensure all tests pass, ask the user if questions arise.

- [x] 9. Implement baseline stability integration test
  - [x] 9.1 Create end-to-end integration test
    - Start server process
    - Start multiple client processes
    - Configure for 55% utilization
    - Run for 30 seconds
    - Collect and aggregate metrics
    - Verify success rate > 99%
    - Verify p50 latency stable within 10%
    - Verify p99 latency stable within 20%
    - Clean up processes
    - **Property 10: Baseline Stability**
    - **Validates: Requirements 5.1, 5.2, 5.3, 5.4, 5.5, 5.6, 5.7**

  - [x] 9.2 Create helper utilities for integration testing
    - Process spawning and management
    - IPC directory setup and cleanup
    - Metrics aggregation utilities
    - _Requirements: 5.1_

- [ ]* 10. Write property test for utilization range stability
  - **Property 9: Utilization Range Stability**
  - **Validates: Requirements 4.5**

- [ ]* 11. Write property test for response routing correctness
  - **Property 11: Response Routing Correctness**
  - **Validates: Requirements 7.5**

- [x] 12. Tune baseline configuration parameters
  - [x] 12.1 Run baseline test with initial parameters
    - think_time_ms = 10
    - num_clients = 5
    - requests_per_second = 11 (per client)
    - Observe success rate and latencies
    - _Requirements: 5.2_

  - [x] 12.2 Iteratively adjust parameters
    - If success rate < 99%, reduce load or increase think time
    - If latencies unstable, adjust think time or load
    - Document working configuration
    - _Requirements: 5.2, 5.4, 5.5, 5.6_

  - [x] 12.3 Create example configuration file
    - Save working baseline configuration as example_baseline_config.json
    - Include comments explaining parameter choices
    - _Requirements: 4.4_

- [x] 13. Final checkpoint - Validate complete baseline system
  - Ensure all tests pass, ask the user if questions arise.
  - Verify baseline stability test passes consistently
  - Confirm 55% utilization with 99%+ success rate

## Notes

- Tasks marked with `*` are optional property/unit tests and can be skipped for faster MVP
- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation
- Property tests validate universal correctness properties (min 100 iterations each)
- Unit tests validate specific examples and edge cases
- The baseline stability integration test (Task 9.1) is the most critical test and drives parameter tuning
- Integration tests should run serially to avoid port conflicts
- All tests should use the tag format: `// Feature: dfir-baseline-service, Property N: <text>`
