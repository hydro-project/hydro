//! The amplification checker on [`super::rpc_retry`]: the primary acceptance test of the port.
//!
//! Configuration mirrors the `three_attempts` / `one_attempt` checks of the metastable-3 corpus:
//! two requests per round for the first 125 rounds, a horizon of 1000 rounds, timeout 40,
//! server capacity 5 per tick. Expected: the unheld run has the client send 250 messages; holding
//! the server's request-arrival batch for 128 rounds or more raises that to 746 (two re-sends of
//! each of the 248 requests stuck behind the hold); the first reaction is at a hold of 64, the
//! first doubling past the 40-tick timeout; one attempt never reacts.
//!
//! Runs with `cargo test -p hydro_test --features cress --lib rpc_retry_check`; the module is
//! compiled only with the `cress` feature, which links the checker.

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use hydro_lang::live_collections::stream::{ExactlyOnce, TotalOrder};
    use hydro_lang::prelude::*;
    use cress::{CheckConfig, Report, Verdict, check};

    use crate::cluster::rpc_retry::*;

    const ROUNDS: usize = 1000;
    const DATA_ROUNDS: usize = 125;

    /// `rpc_retry.rs:<line>` of the server's request-arrival batch, found in the source so that
    /// editing the file above it does not break the test. Read at run time: a compile-time
    /// `include_str!` would also be expanded in the staged copy of this crate, where the path
    /// does not resolve.
    fn arrivals_batch_location() -> String {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/src/cluster/rpc_retry.rs");
        let source = std::fs::read_to_string(path).expect("read rpc_retry.rs");
        let line = source
            .lines()
            .position(|l| l.contains("use::batch(incoming,"))
            .expect("rpc_retry.rs has a `use::batch(incoming, ...)`")
            + 1;
        format!("rpc_retry.rs:{line}")
    }

    fn run(policy: RetryPolicy) -> Report {
        let mut flow = FlowBuilder::new();
        let client = flow.process::<Client>();
        let server = flow.process::<Server>();

        let (request_send, requests) = client.sim_input::<u64, TotalOrder, ExactlyOnce>();
        let (clock_send, client_clock) = client.sim_input::<(), TotalOrder, ExactlyOnce>();
        let (_c, client_report_tick) = client.sim_input::<(), TotalOrder, ExactlyOnce>();
        let (_s, server_report_tick) = server.sim_input::<(), TotalOrder, ExactlyOnce>();

        let outputs = rpc_with_retries(
            &client,
            &server,
            requests,
            client_clock,
            client_report_tick,
            server_report_tick,
            policy,
            ServerConfig {
                max_per_tick: 5,
                service_time: Duration::ZERO,
            },
        );
        outputs.completed.sim_output();
        outputs.abandoned.sim_output();
        outputs.outgoing.sim_output();
        outputs.processed.sim_output();
        outputs.backlog_trace.sim_output();
        outputs.client_metrics.sim_output();
        outputs.server_metrics.sim_output();

        let config = CheckConfig::new(ROUNDS).with_workload_rounds(DATA_ROUNDS);
        let report = check(flow.sim(), &config, async |round| {
            clock_send.send(());
            if round < DATA_ROUNDS {
                request_send.send(round as u64);
                request_send.send(round as u64);
            }
        });
        println!("{report}");
        report
    }

    #[test]
    fn three_attempts_is_hazardous() {
        let report = run(RetryPolicy {
            timeout_ticks: 40,
            max_attempts: 3,
        });
        assert_eq!(report.verdict, Verdict::Hazardous);
        let location = report.location.as_ref().unwrap();
        let arrivals_location = arrivals_batch_location();
        assert!(
            location.source_location.ends_with(&arrivals_location),
            "expected the server's request-arrival batch ({arrivals_location}), got {}",
            location.source_location
        );
        assert_eq!(location.first_reaction_at, 64);

        // The client's sends under a hold of the arrivals batch that outlasts the input.
        let arrivals = report
            .curves
            .iter()
            .find(|c| c.hook == location.hook)
            .unwrap();
        let client_sends_at = |k: usize| {
            let (_, counts) = arrivals.counts.iter().find(|(kk, _)| *kk == k).unwrap();
            *counts
                .sends_by_member()
                .iter()
                .find(|(place, _)| place.contains("loc1"))
                .map(|(_, n)| n)
                .unwrap()
        };
        assert_eq!(client_sends_at(0), 250);
        assert_eq!(client_sends_at(128), 746);
        assert_eq!(client_sends_at(256), 746);
        assert_eq!(client_sends_at(512), 746);
        assert_eq!(client_sends_at(999), 746);
        assert_eq!(report.not_holdable.len(), 1, "{:?}", report.not_holdable);
    }

    #[test]
    fn one_attempt_is_benign() {
        let report = run(RetryPolicy {
            timeout_ticks: 40,
            max_attempts: 1,
        });
        assert_eq!(report.verdict, Verdict::Benign);
    }
}
