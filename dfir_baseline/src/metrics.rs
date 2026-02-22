use crate::{AggregatedMetrics, MetricEvent};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

/// Writer for metric events to IPC file
pub struct MetricsWriter {
    writer: BufWriter<File>,
    path: PathBuf,
}

impl MetricsWriter {
    /// Create a new MetricsWriter that writes to the specified file
    pub fn new<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        
        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;
        
        let writer = BufWriter::new(file);
        
        Ok(Self { writer, path })
    }
    
    /// Write a metric event (event already contains timestamp)
    pub fn write_event(&mut self, event: MetricEvent) -> std::io::Result<()> {
        let json = serde_json::to_string(&event)?;
        writeln!(self.writer, "{}", json)?;
        
        Ok(())
    }
    
    /// Flush buffered data to disk
    pub fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }
    
    /// Get the path this writer is writing to
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for MetricsWriter {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader};
    use tempfile::TempDir;
    
    #[test]
    fn test_metrics_writer_creates_file() {
        let temp_dir = TempDir::new().unwrap();
        let metrics_path = temp_dir.path().join("metrics.jsonl");
        
        let mut writer = MetricsWriter::new(&metrics_path).unwrap();
        writer.write_event(MetricEvent::RequestSent { 
            timestamp: 1000.0,
            req_id: 1 
        }).unwrap();
        writer.flush().unwrap();
        
        assert!(metrics_path.exists());
    }
    
    #[test]
    fn test_metrics_writer_writes_json_lines() {
        let temp_dir = TempDir::new().unwrap();
        let metrics_path = temp_dir.path().join("metrics.jsonl");
        
        let mut writer = MetricsWriter::new(&metrics_path).unwrap();
        writer.write_event(MetricEvent::RequestSent { 
            timestamp: 1000.0,
            req_id: 1 
        }).unwrap();
        writer.write_event(MetricEvent::ResponseReceived { 
            timestamp: 1050.0,
            req_id: 1,
            latency_ms: 50.0
        }).unwrap();
        writer.flush().unwrap();
        
        // Read back and verify
        let file = File::open(&metrics_path).unwrap();
        let reader = BufReader::new(file);
        let lines: Vec<String> = reader.lines().map(|l| l.unwrap()).collect();
        
        assert_eq!(lines.len(), 2);
        
        // Verify first line is valid JSON with timestamp
        let event1: MetricEvent = serde_json::from_str(&lines[0]).unwrap();
        assert!(matches!(event1, MetricEvent::RequestSent { timestamp: 1000.0, req_id: 1 }));
        
        // Verify second line
        let event2: MetricEvent = serde_json::from_str(&lines[1]).unwrap();
        assert!(matches!(event2, MetricEvent::ResponseReceived { timestamp: 1050.0, req_id: 1, latency_ms: 50.0 }));
    }
    
    #[test]
    fn test_metrics_writer_creates_parent_directory() {
        let temp_dir = TempDir::new().unwrap();
        let metrics_path = temp_dir.path().join("subdir").join("metrics.jsonl");
        
        let mut writer = MetricsWriter::new(&metrics_path).unwrap();
        writer.write_event(MetricEvent::RequestSent { 
            timestamp: 1000.0,
            req_id: 1 
        }).unwrap();
        writer.flush().unwrap();
        
        assert!(metrics_path.exists());
        assert!(metrics_path.parent().unwrap().exists());
    }
    
    #[test]
    fn test_metrics_writer_appends_to_existing_file() {
        let temp_dir = TempDir::new().unwrap();
        let metrics_path = temp_dir.path().join("metrics.jsonl");
        
        // Write first event
        {
            let mut writer = MetricsWriter::new(&metrics_path).unwrap();
            writer.write_event(MetricEvent::RequestSent { 
                timestamp: 1000.0,
                req_id: 1 
            }).unwrap();
        }
        
        // Write second event (should append)
        {
            let mut writer = MetricsWriter::new(&metrics_path).unwrap();
            writer.write_event(MetricEvent::RequestSent { 
                timestamp: 2000.0,
                req_id: 2 
            }).unwrap();
        }
        
        // Verify both events are present
        let file = File::open(&metrics_path).unwrap();
        let reader = BufReader::new(file);
        let lines: Vec<String> = reader.lines().map(|l| l.unwrap()).collect();
        
        assert_eq!(lines.len(), 2);
    }
}

/// Reader for aggregating metric events from IPC files
pub struct MetricsReader {
    events: Vec<MetricEvent>,
}

impl MetricsReader {
    /// Create a new MetricsReader
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
        }
    }
    
    /// Read metric events from a file
    pub fn read_from_file<P: AsRef<Path>>(&mut self, path: P) -> std::io::Result<()> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            
            match serde_json::from_str::<MetricEvent>(&line) {
                Ok(event) => self.events.push(event),
                Err(e) => eprintln!("Warning: Failed to parse metric event: {}", e),
            }
        }
        
        Ok(())
    }
    
    /// Calculate p50 latency in milliseconds
    pub fn p50_latency(&self) -> Option<f64> {
        self.percentile_latency(0.50)
    }
    
    /// Calculate p99 latency in milliseconds
    pub fn p99_latency(&self) -> Option<f64> {
        self.percentile_latency(0.99)
    }
    
    /// Calculate percentile latency
    fn percentile_latency(&self, percentile: f64) -> Option<f64> {
        let mut latencies: Vec<f64> = self.events.iter()
            .filter_map(|event| {
                if let MetricEvent::ResponseReceived { latency_ms, .. } = event {
                    Some(*latency_ms)
                } else {
                    None
                }
            })
            .collect();
        
        if latencies.is_empty() {
            return None;
        }
        
        latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let index = ((latencies.len() as f64 - 1.0) * percentile) as usize;
        Some(latencies[index])
    }
    
    /// Calculate success rate (responses received / requests sent)
    pub fn success_rate(&self) -> f64 {
        let requests_sent = self.events.iter()
            .filter(|e| matches!(e, MetricEvent::RequestSent { .. }))
            .count();
        
        let responses_received = self.events.iter()
            .filter(|e| matches!(e, MetricEvent::ResponseReceived { .. }))
            .count();
        
        if requests_sent == 0 {
            return 0.0;
        }
        
        responses_received as f64 / requests_sent as f64
    }
    
    /// Calculate offered arrival rate (requests per second, without amplification)
    pub fn offered_rate(&self) -> f64 {
        if self.events.is_empty() {
            return 0.0;
        }
        
        let request_timestamps: Vec<f64> = self.events.iter()
            .filter_map(|event| {
                if let MetricEvent::RequestSent { timestamp, .. } = event {
                    Some(*timestamp)
                } else {
                    None
                }
            })
            .collect();
        
        if request_timestamps.len() < 2 {
            return 0.0;
        }
        
        let min_time = request_timestamps.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_time = request_timestamps.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        
        let duration_secs = (max_time - min_time) / 1000.0;
        
        if duration_secs <= 0.0 {
            return 0.0;
        }
        
        request_timestamps.len() as f64 / duration_secs
    }
    
    /// Export aggregated metrics as time-series data
    /// Groups events into time windows and calculates metrics for each window
    pub fn export_time_series(&self, window_size_ms: f64) -> Vec<AggregatedMetrics> {
        if self.events.is_empty() {
            return Vec::new();
        }
        
        // Find time range
        let all_timestamps: Vec<f64> = self.events.iter()
            .map(|event| match event {
                MetricEvent::RequestSent { timestamp, .. } => *timestamp,
                MetricEvent::ResponseReceived { timestamp, .. } => *timestamp,
            })
            .collect();
        
        let min_time = all_timestamps.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_time = all_timestamps.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        
        // Create time windows
        let mut windows = Vec::new();
        let mut current_time = min_time;
        
        while current_time <= max_time {
            let window_end = current_time + window_size_ms;
            
            // Filter events in this window
            let window_events: Vec<&MetricEvent> = self.events.iter()
                .filter(|event| {
                    let timestamp = match event {
                        MetricEvent::RequestSent { timestamp, .. } => *timestamp,
                        MetricEvent::ResponseReceived { timestamp, .. } => *timestamp,
                    };
                    timestamp >= current_time && timestamp < window_end
                })
                .collect();
            
            if !window_events.is_empty() {
                // Create temporary reader for this window
                let mut window_reader = MetricsReader::new();
                window_reader.events = window_events.into_iter().cloned().collect();
                
                let metrics = AggregatedMetrics {
                    timestamp: current_time,
                    p50_latency_ms: window_reader.p50_latency().unwrap_or(0.0),
                    p99_latency_ms: window_reader.p99_latency().unwrap_or(0.0),
                    success_rate: window_reader.success_rate(),
                    offered_rate: window_reader.offered_rate(),
                };
                
                windows.push(metrics);
            }
            
            current_time = window_end;
        }
        
        windows
    }
    
    /// Get total number of events
    pub fn event_count(&self) -> usize {
        self.events.len()
    }
}

impl Default for MetricsReader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod reader_tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_metrics_reader_reads_file() {
        let temp_dir = TempDir::new().unwrap();
        let metrics_path = temp_dir.path().join("metrics.jsonl");
        
        // Write some events
        let mut writer = MetricsWriter::new(&metrics_path).unwrap();
        writer.write_event(MetricEvent::RequestSent { 
            timestamp: 1000.0,
            req_id: 1 
        }).unwrap();
        writer.write_event(MetricEvent::ResponseReceived { 
            timestamp: 1050.0,
            req_id: 1,
            latency_ms: 50.0
        }).unwrap();
        writer.flush().unwrap();
        drop(writer);
        
        // Read back
        let mut reader = MetricsReader::new();
        reader.read_from_file(&metrics_path).unwrap();
        
        assert_eq!(reader.event_count(), 2);
    }
    
    #[test]
    fn test_metrics_reader_calculates_latencies() {
        let mut reader = MetricsReader::new();
        reader.events = vec![
            MetricEvent::ResponseReceived { timestamp: 1000.0, req_id: 1, latency_ms: 10.0 },
            MetricEvent::ResponseReceived { timestamp: 1100.0, req_id: 2, latency_ms: 20.0 },
            MetricEvent::ResponseReceived { timestamp: 1200.0, req_id: 3, latency_ms: 30.0 },
            MetricEvent::ResponseReceived { timestamp: 1300.0, req_id: 4, latency_ms: 40.0 },
            MetricEvent::ResponseReceived { timestamp: 1400.0, req_id: 5, latency_ms: 50.0 },
        ];
        
        assert_eq!(reader.p50_latency(), Some(30.0));
        // p99 with 5 values: index = (5-1) * 0.99 = 3.96 -> rounds to 3 -> 40.0
        assert_eq!(reader.p99_latency(), Some(40.0));
    }
    
    #[test]
    fn test_metrics_reader_calculates_success_rate() {
        let mut reader = MetricsReader::new();
        reader.events = vec![
            MetricEvent::RequestSent { timestamp: 1000.0, req_id: 1 },
            MetricEvent::RequestSent { timestamp: 1100.0, req_id: 2 },
            MetricEvent::RequestSent { timestamp: 1200.0, req_id: 3 },
            MetricEvent::RequestSent { timestamp: 1300.0, req_id: 4 },
            MetricEvent::ResponseReceived { timestamp: 1050.0, req_id: 1, latency_ms: 50.0 },
            MetricEvent::ResponseReceived { timestamp: 1150.0, req_id: 2, latency_ms: 50.0 },
            MetricEvent::ResponseReceived { timestamp: 1250.0, req_id: 3, latency_ms: 50.0 },
        ];
        
        // 3 responses / 4 requests = 0.75
        assert_eq!(reader.success_rate(), 0.75);
    }
    
    #[test]
    fn test_metrics_reader_calculates_offered_rate() {
        let mut reader = MetricsReader::new();
        // 5 requests over 4 seconds (1000ms to 5000ms) = 1.25 req/sec
        reader.events = vec![
            MetricEvent::RequestSent { timestamp: 1000.0, req_id: 1 },
            MetricEvent::RequestSent { timestamp: 2000.0, req_id: 2 },
            MetricEvent::RequestSent { timestamp: 3000.0, req_id: 3 },
            MetricEvent::RequestSent { timestamp: 4000.0, req_id: 4 },
            MetricEvent::RequestSent { timestamp: 5000.0, req_id: 5 },
        ];
        
        let rate = reader.offered_rate();
        assert!((rate - 1.25).abs() < 0.01);
    }
    
    #[test]
    fn test_metrics_reader_exports_time_series() {
        let mut reader = MetricsReader::new();
        reader.events = vec![
            MetricEvent::RequestSent { timestamp: 1000.0, req_id: 1 },
            MetricEvent::ResponseReceived { timestamp: 1050.0, req_id: 1, latency_ms: 50.0 },
            MetricEvent::RequestSent { timestamp: 2000.0, req_id: 2 },
            MetricEvent::ResponseReceived { timestamp: 2100.0, req_id: 2, latency_ms: 100.0 },
        ];
        
        let time_series = reader.export_time_series(1000.0);
        
        // Should have 2 windows
        assert_eq!(time_series.len(), 2);
        assert_eq!(time_series[0].timestamp, 1000.0);
        assert_eq!(time_series[1].timestamp, 2000.0);
    }
}
