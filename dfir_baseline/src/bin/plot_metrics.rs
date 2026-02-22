/// Plot metrics from baseline test
/// 
/// Reads metrics files and generates time-series plots

use dfir_baseline::metrics::MetricsReader;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() < 2 {
        eprintln!("Usage: {} <ipc_directory> [num_clients]", args[0]);
        std::process::exit(1);
    }
    
    let ipc_directory = &args[1];
    let num_clients = if args.len() > 2 {
        args[2].parse().unwrap_or(5)
    } else {
        5
    };
    
    println!("Reading metrics from: {}", ipc_directory);
    println!("Number of clients: {}", num_clients);
    
    // Aggregate metrics from all clients
    let mut reader = MetricsReader::new();
    
    for client_id in 0..num_clients {
        let metrics_file = format!("{}/client_{}.jsonl", ipc_directory, client_id);
        let path = PathBuf::from(&metrics_file);
        
        if path.exists() {
            println!("Reading: {}", metrics_file);
            reader.read_from_file(&path)?;
        } else {
            println!("Warning: {} not found", metrics_file);
        }
    }
    
    println!("\nTotal events: {}", reader.event_count());
    
    // Export time-series data (1-second windows)
    let time_series = reader.export_time_series(1000.0);
    
    if time_series.is_empty() {
        println!("No time-series data available");
        return Ok(());
    }
    
    println!("Time windows: {}", time_series.len());
    
    // Generate CSV for plotting
    let csv_path = format!("{}/metrics_timeseries.csv", ipc_directory);
    let mut csv_file = File::create(&csv_path)?;
    
    writeln!(csv_file, "timestamp_sec,p50_latency_ms,p99_latency_ms,success_rate,offered_rate")?;
    
    let start_time = time_series[0].timestamp;
    for metrics in &time_series {
        let timestamp_sec = (metrics.timestamp - start_time) / 1000.0;
        writeln!(
            csv_file,
            "{:.2},{:.2},{:.2},{:.4},{:.2}",
            timestamp_sec,
            metrics.p50_latency_ms,
            metrics.p99_latency_ms,
            metrics.success_rate,
            metrics.offered_rate
        )?;
    }
    
    println!("\nCSV written to: {}", csv_path);
    
    // Generate Python plotting script
    let plot_script = format!("{}/plot_metrics.py", ipc_directory);
    let mut script_file = File::create(&plot_script)?;
    
    // Write Python script (using write! to avoid Rust format string parsing)
    script_file.write_all(b"#!/usr/bin/env python3\n")?;
    script_file.write_all(b"import pandas as pd\n")?;
    script_file.write_all(b"import matplotlib.pyplot as plt\n")?;
    script_file.write_all(b"import sys\n")?;
    script_file.write_all(b"import os\n\n")?;
    script_file.write_all(b"# Change to script directory\n")?;
    script_file.write_all(b"os.chdir(os.path.dirname(os.path.abspath(__file__)))\n\n")?;
    script_file.write_all(b"# Read CSV\n")?;
    script_file.write_all(b"df = pd.read_csv('metrics_timeseries.csv')\n\n")?;
    script_file.write_all(b"# Create figure with subplots\n")?;
    script_file.write_all(b"fig, axes = plt.subplots(3, 1, figsize=(12, 10))\n")?;
    script_file.write_all(b"fig.suptitle('DFIR Baseline Service - Time Series Metrics', fontsize=16)\n\n")?;
    script_file.write_all(b"# Plot 1: Latency\n")?;
    script_file.write_all(b"ax1 = axes[0]\n")?;
    script_file.write_all(b"ax1.plot(df['timestamp_sec'], df['p50_latency_ms'], label='p50 latency', linewidth=2)\n")?;
    script_file.write_all(b"ax1.plot(df['timestamp_sec'], df['p99_latency_ms'], label='p99 latency', linewidth=2)\n")?;
    script_file.write_all(b"ax1.set_ylabel('Latency (ms)')\n")?;
    script_file.write_all(b"ax1.set_title('Request Latency Over Time')\n")?;
    script_file.write_all(b"ax1.legend()\n")?;
    script_file.write_all(b"ax1.grid(True, alpha=0.3)\n\n")?;
    script_file.write_all(b"# Plot 2: Success Rate\n")?;
    script_file.write_all(b"ax2 = axes[1]\n")?;
    script_file.write_all(b"ax2.plot(df['timestamp_sec'], df['success_rate'] * 100, label='Success rate', linewidth=2, color='green')\n")?;
    script_file.write_all(b"ax2.set_ylabel('Success Rate (%)')\n")?;
    script_file.write_all(b"ax2.set_title('Success Rate Over Time')\n")?;
    script_file.write_all(b"ax2.set_ylim([0, 105])\n")?;
    script_file.write_all(b"ax2.axhline(y=99, color='r', linestyle='--', label='99% threshold')\n")?;
    script_file.write_all(b"ax2.legend()\n")?;
    script_file.write_all(b"ax2.grid(True, alpha=0.3)\n\n")?;
    script_file.write_all(b"# Plot 3: Offered Rate\n")?;
    script_file.write_all(b"ax3 = axes[2]\n")?;
    script_file.write_all(b"ax3.plot(df['timestamp_sec'], df['offered_rate'], label='Offered rate', linewidth=2, color='orange')\n")?;
    script_file.write_all(b"ax3.set_xlabel('Time (seconds)')\n")?;
    script_file.write_all(b"ax3.set_ylabel('Rate (req/s)')\n")?;
    script_file.write_all(b"ax3.set_title('Offered Arrival Rate Over Time')\n")?;
    script_file.write_all(b"ax3.legend()\n")?;
    script_file.write_all(b"ax3.grid(True, alpha=0.3)\n\n")?;
    script_file.write_all(b"plt.tight_layout()\n")?;
    script_file.write_all(b"plt.savefig('metrics_timeseries.png', dpi=150, bbox_inches='tight')\n")?;
    script_file.write_all(b"print('Plot saved to: metrics_timeseries.png')\n\n")?;
    script_file.write_all(b"# Print summary statistics\n")?;
    script_file.write_all(b"print('\\n=== Summary Statistics ===')\n")?;
    script_file.write_all(b"p50_mean = df['p50_latency_ms'].mean()\n")?;
    script_file.write_all(b"p50_std = df['p50_latency_ms'].std()\n")?;
    script_file.write_all(b"p50_cv = p50_std / p50_mean\n")?;
    script_file.write_all(b"print(f'p50 latency: mean={p50_mean:.2f}ms, std={p50_std:.2f}ms, CV={p50_cv:.3f}')\n")?;
    script_file.write_all(b"\n")?;
    script_file.write_all(b"p99_mean = df['p99_latency_ms'].mean()\n")?;
    script_file.write_all(b"p99_std = df['p99_latency_ms'].std()\n")?;
    script_file.write_all(b"p99_cv = p99_std / p99_mean\n")?;
    script_file.write_all(b"print(f'p99 latency: mean={p99_mean:.2f}ms, std={p99_std:.2f}ms, CV={p99_cv:.3f}')\n")?;
    script_file.write_all(b"\n")?;
    script_file.write_all(b"sr_mean = df['success_rate'].mean() * 100\n")?;
    script_file.write_all(b"sr_min = df['success_rate'].min() * 100\n")?;
    script_file.write_all(b"print(f'Success rate: mean={sr_mean:.2f}%, min={sr_min:.2f}%')\n")?;
    script_file.write_all(b"\n")?;
    script_file.write_all(b"or_mean = df['offered_rate'].mean()\n")?;
    script_file.write_all(b"or_std = df['offered_rate'].std()\n")?;
    script_file.write_all(b"print(f'Offered rate: mean={or_mean:.2f} req/s, std={or_std:.2f} req/s')\n")?;
    
    println!("Python plot script written to: {}", plot_script);
    println!("\nTo generate plots, run:");
    println!("  cd {} && python3 plot_metrics.py", ipc_directory);
    
    // Print summary statistics
    println!("\n=== Summary Statistics ===");
    
    let p50_values: Vec<f64> = time_series.iter()
        .map(|m| m.p50_latency_ms)
        .filter(|&v| v > 0.0)
        .collect();
    
    let p99_values: Vec<f64> = time_series.iter()
        .map(|m| m.p99_latency_ms)
        .filter(|&v| v > 0.0)
        .collect();
    
    let success_rates: Vec<f64> = time_series.iter()
        .map(|m| m.success_rate)
        .collect();
    
    let offered_rates: Vec<f64> = time_series.iter()
        .map(|m| m.offered_rate)
        .filter(|&v| v > 0.0)
        .collect();
    
    if !p50_values.is_empty() {
        let mean = p50_values.iter().sum::<f64>() / p50_values.len() as f64;
        let variance = p50_values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / p50_values.len() as f64;
        let std_dev = variance.sqrt();
        println!("p50 latency: mean={:.2}ms, std={:.2}ms, CV={:.3}", mean, std_dev, std_dev / mean);
    }
    
    if !p99_values.is_empty() {
        let mean = p99_values.iter().sum::<f64>() / p99_values.len() as f64;
        let variance = p99_values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / p99_values.len() as f64;
        let std_dev = variance.sqrt();
        println!("p99 latency: mean={:.2}ms, std={:.2}ms, CV={:.3}", mean, std_dev, std_dev / mean);
    }
    
    if !success_rates.is_empty() {
        let mean = success_rates.iter().sum::<f64>() / success_rates.len() as f64;
        let min = success_rates.iter().cloned().fold(f64::INFINITY, f64::min);
        println!("Success rate: mean={:.2}%, min={:.2}%", mean * 100.0, min * 100.0);
    }
    
    if !offered_rates.is_empty() {
        let mean = offered_rates.iter().sum::<f64>() / offered_rates.len() as f64;
        let variance = offered_rates.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / offered_rates.len() as f64;
        let std_dev = variance.sqrt();
        println!("Offered rate: mean={:.2} req/s, std={:.2} req/s", mean, std_dev);
    }
    
    Ok(())
}
