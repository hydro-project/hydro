use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use clap::Parser;
use reqwest::Client;
use tokio::sync::Mutex;

#[derive(Parser)]
#[command(about = "Load test a KVS REST endpoint")]
struct Args {
    /// KVS endpoint URLs (e.g. http://localhost:32816 http://localhost:32817)
    #[arg(required = true)]
    endpoints: Vec<String>,
    /// Number of concurrent workers
    #[arg(short, long, default_value_t = 10)]
    concurrency: usize,
    /// Test duration in seconds
    #[arg(short, long, default_value_t = 10)]
    duration: u64,
    /// Per-request timeout in seconds
    #[arg(short, long, default_value_t = 5)]
    timeout: u64,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let endpoints = &args.endpoints;
    let concurrency = args.concurrency;
    let duration_secs = args.duration;

    println!("Endpoints:   {}", endpoints.join(", "));
    println!("Concurrency: {concurrency}");
    println!("Duration:    {duration_secs}s");
    println!();

    let client = Client::builder()
        .pool_max_idle_per_host(concurrency)
        .timeout(Duration::from_secs(args.timeout))
        .build()
        .unwrap();

    let puts = Arc::new(AtomicU64::new(0));
    let gets = Arc::new(AtomicU64::new(0));
    let errors = Arc::new(AtomicU64::new(0));
    let put_latencies = Arc::new(Mutex::new(Vec::<f64>::new()));
    let get_latencies = Arc::new(Mutex::new(Vec::<f64>::new()));
    // (latency_ms, trace_id, "PUT"/"GET", key)
    let all_requests = Arc::new(Mutex::new(Vec::<(f64, String, &'static str, String)>::new()));
    let start = Instant::now();
    let deadline = start + Duration::from_secs(duration_secs);

    let mut handles = Vec::new();
    for worker in 0..concurrency {
        let client = client.clone();
        let endpoints = endpoints.clone();
        let puts = puts.clone();
        let gets = gets.clone();
        let errors = errors.clone();
        let put_latencies = put_latencies.clone();
        let get_latencies = get_latencies.clone();

        let all_requests = all_requests.clone();

        handles.push(tokio::spawn(async move {
            let mut local_put_lats = Vec::new();
            let mut local_get_lats = Vec::new();
            let mut local_requests: Vec<(f64, String, &'static str, String)> = Vec::new();
            let mut i = 0u64;
            while Instant::now() < deadline {
                let endpoint = &endpoints[(worker + i as usize) % endpoints.len()];
                let key = format!("w{worker}_k{i}");
                let value = format!("val_{i}");

                // PUT
                let t = Instant::now();
                match client
                    .put(format!("{endpoint}/{key}"))
                    .body(value.clone())
                    .send()
                    .await
                {
                    Ok(resp) if resp.status().is_success() => {
                        let lat = t.elapsed().as_secs_f64() * 1000.0;
                        let trace_id = resp
                            .headers()
                            .get("x-trace-id")
                            .and_then(|v| v.to_str().ok())
                            .unwrap_or("unknown")
                            .to_string();
                        local_put_lats.push(lat);
                        local_requests.push((lat, trace_id, "PUT", key.clone()));
                        puts.fetch_add(1, Ordering::Relaxed);
                    }
                    Ok(resp) => {
                        errors.fetch_add(1, Ordering::Relaxed);
                        eprintln!("[w{worker}] PUT /{key} status={}", resp.status());
                    }
                    Err(e) => {
                        errors.fetch_add(1, Ordering::Relaxed);
                        eprintln!("[w{worker}] PUT /{key} error: {e}");
                    }
                }

                // GET
                let t = Instant::now();
                match client.get(format!("{endpoint}/{key}")).send().await {
                    Ok(resp) if resp.status().is_success() => {
                        let trace_id = resp
                            .headers()
                            .get("x-trace-id")
                            .and_then(|v| v.to_str().ok())
                            .unwrap_or("unknown")
                            .to_string();
                        let body = resp.text().await.unwrap_or_default();
                        let lat = t.elapsed().as_secs_f64() * 1000.0;
                        local_get_lats.push(lat);
                        local_requests.push((lat, trace_id, "GET", key.clone()));
                        assert!(
                            body.contains(&value),
                            "[w{worker}] GET /{key}: expected value '{value}' in response: {body}"
                        );
                        gets.fetch_add(1, Ordering::Relaxed);
                    }
                    Ok(resp) => {
                        errors.fetch_add(1, Ordering::Relaxed);
                        eprintln!("[w{worker}] GET /{key} status={}", resp.status());
                    }
                    Err(e) => {
                        errors.fetch_add(1, Ordering::Relaxed);
                        eprintln!("[w{worker}] GET /{key} error: {e}");
                    }
                }

                i += 1;
            }
            put_latencies.lock().await.extend(local_put_lats);
            get_latencies.lock().await.extend(local_get_lats);
            all_requests.lock().await.extend(local_requests);
        }));
    }

    // Progress reporter
    let puts_r = puts.clone();
    let gets_r = gets.clone();
    let errors_r = errors.clone();
    let (stop_tx, mut stop_rx) = tokio::sync::oneshot::channel::<()>();
    let reporter = tokio::spawn(async move {
        let mut last_total = 0u64;
        loop {
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_secs(1)) => {}
                _ = &mut stop_rx => break,
            }
            let p = puts_r.load(Ordering::Relaxed);
            let g = gets_r.load(Ordering::Relaxed);
            let e = errors_r.load(Ordering::Relaxed);
            let total = p + g;
            let rps = total - last_total;
            last_total = total;
            let elapsed = start.elapsed().as_secs();
            println!("[{elapsed:>3}s] puts={p} gets={g} errors={e} rps={rps}");
        }
    });

    for h in handles {
        if let Err(e) = h.await {
            eprintln!("Worker error: {e}");
        }
    }
    let _ = stop_tx.send(());
    let _ = reporter.await;

    let elapsed = start.elapsed();
    let total_puts = puts.load(Ordering::Relaxed);
    let total_gets = gets.load(Ordering::Relaxed);
    let total_errors = errors.load(Ordering::Relaxed);
    let total_ops = total_puts + total_gets;
    let rps = total_ops as f64 / elapsed.as_secs_f64();

    let mut put_lats = put_latencies.lock().await;
    let mut get_lats = get_latencies.lock().await;
    put_lats.sort_by(|a, b| a.partial_cmp(b).unwrap());
    get_lats.sort_by(|a, b| a.partial_cmp(b).unwrap());

    println!();
    println!("=== Results ===");
    println!("Duration: {:.1}s", elapsed.as_secs_f64());
    println!("Puts:     {total_puts}");
    println!("Gets:     {total_gets}");
    println!("Errors:   {total_errors}");
    println!("Total:    {total_ops}");
    println!("RPS:      {rps:.0}");

    if !put_lats.is_empty() {
        println!();
        println!("PUT latency (ms):");
        print_percentiles(&put_lats);
    }
    if !get_lats.is_empty() {
        println!();
        println!("GET latency (ms):");
        print_percentiles(&get_lats);
    }

    let mut reqs = all_requests.lock().await;
    if !reqs.is_empty() {
        reqs.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        println!();
        println!("Top 5 slowest requests:");
        for (lat, trace_id, op, key) in reqs.iter().take(5) {
            println!("  {lat:>8.1}ms  {op} /{key}  trace_id={trace_id}");
        }
    }
}

fn print_percentiles(sorted: &[f64]) {
    let pcts = [50.0, 75.0, 90.0, 95.0, 99.0, 99.9];
    println!("  min    {:.1}", sorted[0]);
    for p in pcts {
        let idx = ((p / 100.0) * sorted.len() as f64) as usize;
        let idx = idx.min(sorted.len() - 1);
        println!("  p{p:<5} {:.1}", sorted[idx]);
    }
    println!("  max    {:.1}", sorted[sorted.len() - 1]);
    let avg = sorted.iter().sum::<f64>() / sorted.len() as f64;
    println!();
    println!("  avg    {avg:.1}");
}
