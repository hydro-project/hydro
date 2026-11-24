#[cfg(feature = "tokio")]
pub mod compartmentalized_paxos;
#[cfg(feature = "tokio")]
pub mod compute_pi;
pub mod kv_replica;
pub mod many_to_many;
pub mod map_reduce;
#[cfg(feature = "tokio")]
pub mod paxos;
#[cfg(feature = "tokio")]
pub mod paxos_bench;
#[cfg(feature = "tokio")]
pub mod paxos_log_bench;
#[cfg(feature = "tokio")]
pub mod paxos_with_client;
pub mod raft;
#[cfg(feature = "tokio")]
pub mod raft_bench;
pub mod simple_cluster;
#[cfg(feature = "tokio")]
pub mod two_pc;
#[cfg(feature = "tokio")]
pub mod two_pc_bench;

/// Shared assertions for the `*_some_throughput` tests.
#[cfg(all(test, feature = "tokio"))]
#[cfg(stageleft_runtime)]
pub(crate) mod bench_test_util {
    use std::str::FromStr;
    use std::time::{Duration, Instant};

    use regex::Regex;
    use tokio::sync::mpsc::UnboundedReceiver;

    /// Max wait for the first positive sample; Paxos leader election takes 10-15s.
    pub(crate) const FIRST_THROUGHPUT_TIMEOUT: Duration = Duration::from_secs(60);

    /// Returns once `required_samples` `Throughput: N requests/s` lines with `N > 0` are read from
    /// `aggregator_stdout`. Panics if `deadline` elapses or the receiver closes first.
    pub(crate) async fn assert_positive_throughput(
        mut aggregator_stdout: UnboundedReceiver<String>,
        required_samples: usize,
        deadline: Duration,
    ) -> Vec<f64> {
        let re = Regex::new(r"Throughput: ([^ ]+) requests/s").unwrap();
        let start = Instant::now();
        let mut zero_samples = 0usize;
        let mut positive = Vec::new();
        while positive.len() < required_samples {
            let remaining = deadline.saturating_sub(start.elapsed());
            let line = match tokio::time::timeout(remaining, aggregator_stdout.recv()).await {
                Ok(Some(line)) => line,
                Ok(None) => panic!(
                    "aggregator stdout closed after {:.1?} with {} positive and {} zero throughput samples (needed {} positive)",
                    start.elapsed(),
                    positive.len(),
                    zero_samples,
                    required_samples,
                ),
                Err(_elapsed) => panic!(
                    "no {} positive throughput samples within {:?}: saw {} positive and {} zero samples",
                    required_samples,
                    deadline,
                    positive.len(),
                    zero_samples,
                ),
            };
            let Some(caps) = re.captures(&line) else {
                continue;
            };
            let throughput = f64::from_str(&caps[1])
                .unwrap_or_else(|e| panic!("unparseable throughput in {line:?}: {e}"));
            if 0.0 < throughput {
                println!(
                    "Found positive throughput after {:.1?}: {throughput} requests/s",
                    start.elapsed()
                );
                positive.push(throughput);
            } else {
                zero_samples += 1;
            }
        }
        positive
    }
}
