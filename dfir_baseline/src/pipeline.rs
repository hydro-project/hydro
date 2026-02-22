/// DFIR Pipeline Implementation
/// 
/// This module implements a three-stratum DFIR pipeline with explicit handoff
/// buffers between strata. The pipeline processes request/response patterns
/// with configurable simulated latency.

use crate::{Request, Response};
use dfir_rs::dfir_syntax;
use std::collections::HashMap;
use std::time::Duration;

/// Configuration for the DFIR pipeline
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    /// Simulated processing delay per request (think time)
    pub think_time: Duration,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            think_time: Duration::from_millis(10),
        }
    }
}

impl PipelineConfig {
    pub fn new(think_time_ms: u64) -> Self {
        Self {
            think_time: Duration::from_millis(think_time_ms),
        }
    }
}

/// Simulate work by busy-waiting for the specified duration
/// This is used instead of sleep to avoid blocking the async runtime
fn simulate_work(duration: Duration) {
    let duration_ms = duration.as_millis() as u64;
    // Calibrated: ~1000 iterations ≈ 1μs on typical hardware
    // For N milliseconds, do N * 1000 iterations
    let iterations = duration_ms * 1000;
    let mut sum = 0u64;
    for i in 0..iterations {
        sum = sum.wrapping_add(i);
    }
    // Prevent optimization
    std::hint::black_box(sum);
}

/// DFIR Pipeline that processes requests with handoff buffers
/// 
/// The pipeline has three strata:
/// 1. Input stratum: Receives requests from clients
/// 2. Processing stratum: Applies simulated delay
/// 3. Output stratum: Sends responses back to clients
pub struct DfirPipeline {
    config: PipelineConfig,
}

impl DfirPipeline {
    /// Create a new DFIR pipeline
    pub fn new(config: PipelineConfig) -> Self {
        Self { config }
    }

    /// Build and return a DFIR flow with the three-stratum pipeline
    /// 
    /// This creates a pipeline with explicit handoff buffers using `next_stratum()`.
    /// 
    /// # Arguments
    /// * `request_receiver` - Stream to receive requests from all clients
    /// * `response_senders` - Map of client ID to response sender channels
    /// 
    /// # Returns
    /// A DFIR flow that can be run with `run_available()` or `run_tick()`
    pub fn build_flow(
        &self,
        request_receiver: dfir_rs::tokio_stream::wrappers::UnboundedReceiverStream<Request>,
        response_senders: HashMap<u64, dfir_rs::tokio::sync::mpsc::UnboundedSender<Response>>,
    ) -> dfir_rs::scheduled::graph::Dfir<'static> {
        let think_time = self.config.think_time;
        
        // Get the first sender (for now, we route all responses to the first client)
        // In a real implementation, we'd route based on client_id in the request
        let sender = response_senders.into_values().next()
            .expect("At least one response sender must be provided");

        dfir_syntax! {
            // Stage 1: Receive requests from TCP connections
            source_stream(request_receiver)
            
            // Handoff Buffer 1: Explicit stratum boundary
            // This creates an unbounded buffer between stage 1 and stage 2
            -> next_stratum()
            
            // Stage 2: Process with simulated think time
            -> map(|request: Request| {
                simulate_work(think_time);
                request
            })
            
            // Handoff Buffer 2: Explicit stratum boundary
            // This creates an unbounded buffer between stage 2 and stage 3
            -> next_stratum()
            
            // Stage 3: Send responses back via channels
            -> for_each(|request: Request| {
                let response = Response::new(request.id);
                let _ = sender.send(response);
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_config_default() {
        let config = PipelineConfig::default();
        assert_eq!(config.think_time, Duration::from_millis(10));
    }

    #[test]
    fn test_pipeline_config_new() {
        let config = PipelineConfig::new(50);
        assert_eq!(config.think_time, Duration::from_millis(50));
    }

    #[test]
    fn test_pipeline_creation() {
        let config = PipelineConfig::default();
        let _pipeline = DfirPipeline::new(config);
    }
}

