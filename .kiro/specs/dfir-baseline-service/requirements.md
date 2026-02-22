# Requirements Document

## Introduction

This document specifies requirements for a simple DFIR baseline service system designed to establish a stable performance baseline for future metastability testing. The system consists of a minimal DFIR server that processes requests through multiple stages and clients that generate configurable load. The primary goal is to achieve and validate a 55% utilization baseline with near-perfect success rates and stable latencies.

## Glossary

- **DFIR_Server**: The Hydroflow/DFIR dataflow program that processes incoming requests
- **Client**: An external process that sends requests to the DFIR_Server
- **Handoff_Buffer**: An unbounded buffer between DFIR stages that allows work to accumulate
- **Think_Time**: Simulated processing delay using tokio::time::sleep()
- **Utilization**: The percentage of server capacity being used by offered load
- **Success_Rate**: The percentage of requests that receive successful responses
- **Baseline_Configuration**: System parameters that achieve 55% utilization with stable performance
- **Integration_Test**: An automated test that validates the baseline configuration

## Requirements

### Requirement 1: Simple DFIR Server Implementation

**User Story:** As a researcher, I want a minimal DFIR server with multiple stages, so that I can establish a baseline for metastability testing.

#### Acceptance Criteria

1. THE DFIR_Server SHALL implement a dataflow with at least two processing stages
2. THE DFIR_Server SHALL include at least one Handoff_Buffer between stages
3. WHEN processing a request, THE DFIR_Server SHALL simulate work using Think_Time
4. THE DFIR_Server SHALL use tokio::time::sleep() for all Think_Time delays
5. WHEN a request arrives, THE DFIR_Server SHALL send a response after processing completes
6. THE DFIR_Server SHALL expose a network interface for receiving requests from Clients

### Requirement 2: DFIR-Faithful Client Implementation

**User Story:** As a researcher, I want clients that interface with DFIR in a realistic way, so that baseline measurements reflect actual DFIR usage patterns.

#### Acceptance Criteria

1. THE Client SHALL communicate with the DFIR_Server using standard DFIR communication patterns
2. THE Client SHALL NOT use UDP transport for communication with the DFIR_Server
3. THE Client SHALL send requests at a configurable rate
4. WHEN a Client sends a request, THE Client SHALL await a response from the DFIR_Server
5. THE Client SHALL record the latency for each request-response pair
6. THE Client SHALL record whether each request succeeded or failed

### Requirement 3: Metrics Collection

**User Story:** As a researcher, I want to collect performance metrics, so that I can validate the baseline configuration.

#### Acceptance Criteria

1. THE System SHALL collect client-observed latency at p50
2. THE System SHALL collect client-observed latency at p99
3. THE System SHALL calculate Success_Rate as a percentage
4. THE System SHALL record offered arrival rate without amplification
5. THE System SHALL export metrics in a time-series format
6. WHEN metrics are collected, THE System SHALL include timestamps for each measurement

### Requirement 4: Configurable Load Generation

**User Story:** As a researcher, I want to configure client load parameters, so that I can tune the system to achieve 55% utilization.

#### Acceptance Criteria

1. THE System SHALL allow configuration of the number of concurrent Clients
2. THE System SHALL allow configuration of request rate per Client
3. THE System SHALL allow configuration of Think_Time duration in the DFIR_Server
4. WHEN load parameters are changed, THE System SHALL apply them without requiring code changes
5. THE System SHALL support running at different utilization levels from 0% to 100%

### Requirement 5: Baseline Validation Test

**User Story:** As a researcher, I want an automated test that validates baseline performance, so that I can confirm the system is properly configured.

#### Acceptance Criteria

1. THE Integration_Test SHALL run the DFIR_Server and multiple Clients
2. THE Integration_Test SHALL configure the system for 55% utilization
3. THE Integration_Test SHALL run for at least 30 seconds at baseline load
4. THE Integration_Test SHALL verify Success_Rate is at least 99%
5. THE Integration_Test SHALL verify p50 latency remains stable within 10% variance
6. THE Integration_Test SHALL verify p99 latency remains stable within 20% variance
7. WHEN the Integration_Test passes, THE System SHALL be considered properly configured for baseline

### Requirement 6: Multi-Stage Pipeline Architecture

**User Story:** As a researcher, I want the server to have distinct processing stages, so that handoff buffers can accumulate work between stages.

#### Acceptance Criteria

1. THE DFIR_Server SHALL implement a Stage_1 that receives requests
2. THE DFIR_Server SHALL implement a Stage_2 that processes requests after Stage_1
3. THE DFIR_Server SHALL use a Handoff_Buffer to transfer work from Stage_1 to Stage_2
4. WHEN Stage_1 completes processing, THE DFIR_Server SHALL enqueue work into the Handoff_Buffer
5. WHEN Stage_2 is ready, THE DFIR_Server SHALL dequeue work from the Handoff_Buffer
6. THE Handoff_Buffer SHALL be unbounded and accept work without backpressure

### Requirement 7: Request-Response Protocol

**User Story:** As a researcher, I want a simple request-response protocol, so that clients can measure end-to-end latency.

#### Acceptance Criteria

1. THE Client SHALL send requests containing a unique request identifier
2. THE DFIR_Server SHALL include the request identifier in each response
3. WHEN a Client receives a response, THE Client SHALL match it to the original request
4. THE Client SHALL calculate latency as the time between sending a request and receiving its response
5. THE System SHALL ensure responses are delivered to the correct Client
