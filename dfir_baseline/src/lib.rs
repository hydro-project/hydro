// Core types for DFIR baseline service

pub mod pipeline;
pub mod metrics;

use serde::{Deserialize, Serialize};

/// Request sent from client to server
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct Request {
    pub id: u64,
}

impl Request {
    pub fn new(id: u64) -> Self {
        Self { id }
    }

    /// Serialize to bytes (big-endian u64)
    pub fn to_bytes(&self) -> [u8; 8] {
        self.id.to_be_bytes()
    }

    /// Deserialize from bytes (big-endian u64)
    pub fn from_bytes(bytes: [u8; 8]) -> Self {
        Self {
            id: u64::from_be_bytes(bytes),
        }
    }
}

/// Response status codes
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[repr(u8)]
pub enum ResponseStatus {
    Success = 0x00,
    Rejected = 0x01,
}

/// Response sent from server to client
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct Response {
    pub id: u64,
    pub status: ResponseStatus,
}

impl Response {
    pub fn new(id: u64) -> Self {
        Self { 
            id,
            status: ResponseStatus::Success,
        }
    }

    pub fn success(id: u64) -> Self {
        Self {
            id,
            status: ResponseStatus::Success,
        }
    }

    pub fn rejected(id: u64) -> Self {
        Self {
            id,
            status: ResponseStatus::Rejected,
        }
    }

    /// Serialize to bytes (1 byte status + 8 bytes big-endian u64)
    pub fn to_bytes(&self) -> [u8; 9] {
        let mut bytes = [0u8; 9];
        bytes[0] = self.status as u8;
        bytes[1..9].copy_from_slice(&self.id.to_be_bytes());
        bytes
    }

    /// Deserialize from bytes (1 byte status + 8 bytes big-endian u64)
    pub fn from_bytes(bytes: [u8; 9]) -> Self {
        let status = match bytes[0] {
            0x00 => ResponseStatus::Success,
            0x01 => ResponseStatus::Rejected,
            _ => ResponseStatus::Success, // Default to success for unknown status
        };
        let id = u64::from_be_bytes(bytes[1..9].try_into().unwrap());
        Self { id, status }
    }
}

/// Metric event recorded by clients and server
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MetricEvent {
    #[serde(rename = "request_sent")]
    RequestSent {
        timestamp: f64,
        req_id: u64,
    },
    #[serde(rename = "response_received")]
    ResponseReceived {
        timestamp: f64,
        req_id: u64,
        latency_ms: f64,
    },
    #[serde(rename = "request_rejected")]
    RequestRejected {
        timestamp: f64,
        req_id: u64,
    },
    #[serde(rename = "request_retried")]
    RequestRetried {
        timestamp: f64,
        req_id: u64,
        retry_count: u32,
    },
    #[serde(rename = "request_timeout")]
    RequestTimeout {
        timestamp: f64,
        req_id: u64,
    },
    #[serde(rename = "request_failed")]
    RequestFailed {
        timestamp: f64,
        req_id: u64,
    },
    #[serde(rename = "buffer_depth")]
    BufferDepth {
        timestamp: f64,
        buffer_id: usize,
        depth: usize,
    },
    #[serde(rename = "stale_response")]
    StaleResponse {
        timestamp: f64,
        req_id: u64,
    },
}

/// Aggregated metrics computed from metric events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedMetrics {
    pub timestamp: f64,
    pub p50_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub success_rate: f64,
    pub offered_rate: f64,
    pub effective_rate: f64,
    pub retry_amplification: f64,
}

/// Configuration for baseline test
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineConfig {
    /// Server bind address (e.g., "127.0.0.1:8080")
    pub server_address: String,
    
    /// Processing delay per request in milliseconds
    pub think_time_ms: u64,
    
    /// Number of client processes
    pub num_clients: usize,
    
    /// Request rate per client (requests per second)
    pub requests_per_second: f64,
    
    /// Test duration in seconds
    pub duration_secs: u64,
    
    /// Directory for IPC metrics files
    pub ipc_directory: String,
}

impl BaselineConfig {
    /// Load configuration from a JSON file
    pub fn from_file<P: AsRef<std::path::Path>>(path: P) -> anyhow::Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        let config: Self = serde_json::from_str(&contents)?;
        config.validate()?;
        Ok(config)
    }
    
    /// Save configuration to a JSON file
    pub fn to_file<P: AsRef<std::path::Path>>(&self, path: P) -> anyhow::Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }
    
    /// Validate configuration parameters
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.server_address.is_empty() {
            anyhow::bail!("server_address cannot be empty");
        }
        
        if self.think_time_ms == 0 {
            anyhow::bail!("think_time_ms must be greater than 0");
        }
        
        if self.num_clients == 0 {
            anyhow::bail!("num_clients must be greater than 0");
        }
        
        if self.requests_per_second <= 0.0 {
            anyhow::bail!("requests_per_second must be greater than 0");
        }
        
        if self.duration_secs == 0 {
            anyhow::bail!("duration_secs must be greater than 0");
        }
        
        if self.ipc_directory.is_empty() {
            anyhow::bail!("ipc_directory cannot be empty");
        }
        
        Ok(())
    }
    
    /// Calculate server capacity (requests per second)
    pub fn server_capacity(&self) -> f64 {
        1000.0 / self.think_time_ms as f64
    }

    /// Calculate offered load (requests per second)
    pub fn offered_load(&self) -> f64 {
        self.num_clients as f64 * self.requests_per_second
    }

    /// Calculate utilization percentage
    pub fn utilization(&self) -> f64 {
        (self.offered_load() / self.server_capacity()) * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_serialization() {
        let req = Request::new(42);
        let bytes = req.to_bytes();
        let decoded = Request::from_bytes(bytes);
        assert_eq!(req, decoded);
    }

    #[test]
    fn test_response_serialization() {
        let resp = Response::success(123);
        let bytes = resp.to_bytes();
        let decoded = Response::from_bytes(bytes);
        assert_eq!(resp, decoded);
        assert_eq!(decoded.status, ResponseStatus::Success);

        let rejected = Response::rejected(456);
        let bytes = rejected.to_bytes();
        let decoded = Response::from_bytes(bytes);
        assert_eq!(decoded.id, 456);
        assert_eq!(decoded.status, ResponseStatus::Rejected);
    }

    #[test]
    fn test_baseline_config_calculations() {
        let config = BaselineConfig {
            server_address: "127.0.0.1:8080".to_string(),
            think_time_ms: 10,
            num_clients: 5,
            requests_per_second: 11.0,
            duration_secs: 30,
            ipc_directory: "/tmp/metrics".to_string(),
        };

        // Server capacity = 1000 / 10 = 100 req/s
        assert_eq!(config.server_capacity(), 100.0);

        // Offered load = 5 * 11 = 55 req/s
        assert_eq!(config.offered_load(), 55.0);

        // Utilization = 55 / 100 = 55%
        assert!((config.utilization() - 55.0).abs() < 0.01);
    }
}

    #[test]
    fn test_baseline_config_from_file() {
        use tempfile::NamedTempFile;
        use std::io::Write;
        
        let mut temp_file = NamedTempFile::new().unwrap();
        let config_json = r#"{
            "server_address": "127.0.0.1:8080",
            "think_time_ms": 10,
            "num_clients": 5,
            "requests_per_second": 11.0,
            "duration_secs": 30,
            "ipc_directory": "/tmp/metrics"
        }"#;
        
        temp_file.write_all(config_json.as_bytes()).unwrap();
        temp_file.flush().unwrap();
        
        let config = BaselineConfig::from_file(temp_file.path()).unwrap();
        assert_eq!(config.server_address, "127.0.0.1:8080");
        assert_eq!(config.think_time_ms, 10);
        assert_eq!(config.num_clients, 5);
    }
    
    #[test]
    fn test_baseline_config_to_file() {
        use tempfile::NamedTempFile;
        
        let config = BaselineConfig {
            server_address: "127.0.0.1:8080".to_string(),
            think_time_ms: 10,
            num_clients: 5,
            requests_per_second: 11.0,
            duration_secs: 30,
            ipc_directory: "/tmp/metrics".to_string(),
        };
        
        let temp_file = NamedTempFile::new().unwrap();
        config.to_file(temp_file.path()).unwrap();
        
        // Read back and verify
        let loaded = BaselineConfig::from_file(temp_file.path()).unwrap();
        assert_eq!(loaded.server_address, config.server_address);
        assert_eq!(loaded.think_time_ms, config.think_time_ms);
    }
    
    #[test]
    fn test_baseline_config_validation_empty_address() {
        let config = BaselineConfig {
            server_address: "".to_string(),
            think_time_ms: 10,
            num_clients: 5,
            requests_per_second: 11.0,
            duration_secs: 30,
            ipc_directory: "/tmp/metrics".to_string(),
        };
        
        assert!(config.validate().is_err());
    }
    
    #[test]
    fn test_baseline_config_validation_zero_think_time() {
        let config = BaselineConfig {
            server_address: "127.0.0.1:8080".to_string(),
            think_time_ms: 0,
            num_clients: 5,
            requests_per_second: 11.0,
            duration_secs: 30,
            ipc_directory: "/tmp/metrics".to_string(),
        };
        
        assert!(config.validate().is_err());
    }
    
    #[test]
    fn test_baseline_config_validation_zero_clients() {
        let config = BaselineConfig {
            server_address: "127.0.0.1:8080".to_string(),
            think_time_ms: 10,
            num_clients: 0,
            requests_per_second: 11.0,
            duration_secs: 30,
            ipc_directory: "/tmp/metrics".to_string(),
        };
        
        assert!(config.validate().is_err());
    }
    
    #[test]
    fn test_baseline_config_validation_valid() {
        let config = BaselineConfig {
            server_address: "127.0.0.1:8080".to_string(),
            think_time_ms: 10,
            num_clients: 5,
            requests_per_second: 11.0,
            duration_secs: 30,
            ipc_directory: "/tmp/metrics".to_string(),
        };
        
        assert!(config.validate().is_ok());
    }
