//! A request/response service with a client-side timeout-and-retry policy.
//!
//! The client forwards application requests to the server, tagging each with an id, and keeps
//! a table of the requests still awaiting a response. A request that has not been answered
//! within [`RetryPolicy::timeout_ticks`] is sent again, up to [`RetryPolicy::max_attempts`]
//! sends in total, after which the client gives up on it and reports it as abandoned. The
//! server keeps incoming requests in a FIFO and serves at most [`ServerConfig::max_per_tick`] of
//! them per tick, answering every request it serves. Both sides export per-window metrics.
//!
//! # Time is an input
//!
//! This is one program that runs both deployed and in the simulator. Nothing in the dataflow
//! reads a wall clock; all time enters through three stream parameters (the pattern used by
//! [`super::raft::raft`] for its election and heartbeat timers):
//!
//! - `client_clock`: one element per logical client tick. Each element advances the client's
//!   logical clock by one; timeouts and latencies are measured in these ticks.
//! - `client_report_tick`, `server_report_tick`: one element per metrics window.
//!
//! A deployment wires `source_interval` streams into these parameters, so a tick is a fixed
//! period of wall time; a simulation feeds them with `sim_input` and thereby controls time
//! itself. The only deployment-specific piece of the program is the server's optional CPU burn
//! per request ([`ServerConfig::service_time`]), which is what makes its capacity real on a
//! machine and is zero in simulation, where `max_per_tick` alone is the capacity.

use std::time::Duration;

use hydro_lang::live_collections::stream::{ExactlyOnce, NoOrder, TotalOrder};
use hydro_lang::prelude::*;
#[cfg(feature = "cress")]
use cress::{SimOutputs, amplification_check};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

pub struct Client;
pub struct Server;

/// What the client puts on the wire: the application's request body under a client-assigned id.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Request<T> {
    pub id: u64,
    pub body: T,
}

/// The server's answer, echoing the request body.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Response<T> {
    pub id: u64,
    pub body: T,
}

/// Client-side bookkeeping for a request that has been sent but not yet answered. Times are
/// logical client ticks (the index of the `client_clock` element).
#[derive(Clone, Debug)]
pub struct InFlight<T> {
    pub body: T,
    pub first_sent: u64,
    pub last_sent: u64,
    /// Sends so far: 1 after the first send.
    pub attempt: u32,
}

#[derive(Clone, Debug)]
enum Verdict<T> {
    Keep(InFlight<T>),
    Retry(InFlight<T>),
    Abandon(InFlight<T>),
}

/// A request that received its first response.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Completion {
    pub id: u64,
    /// Client ticks from the first send to the first response.
    pub latency_ticks: u64,
}

/// The client's retry policy.
#[derive(Clone, Copy, Debug)]
pub struct RetryPolicy {
    /// How many client ticks the client waits for a response before re-sending.
    pub timeout_ticks: u64,
    /// Total number of sends per request (1 = never retry).
    pub max_attempts: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct ServerConfig {
    /// Upper bound on requests served per server tick: the server's capacity. On a deployment
    /// this also bounds tick length so the server keeps draining its socket while it works
    /// through its backlog.
    pub max_per_tick: u32,
    /// CPU time the server burns per request. Deployment only: makes capacity real on a
    /// machine; must be `Duration::ZERO` in simulation.
    pub service_time: Duration,
}

/// Per-window metrics exported by the client.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClientMetrics {
    pub completed: u64,
    pub latency_sum_ticks: u64,
    pub latency_max_ticks: u64,
    /// Requests put on the wire, including re-sends.
    pub sent: u64,
    /// Of `sent`, how many were re-sends.
    pub retried: u64,
    pub abandoned: u64,
}

impl ClientMetrics {
    pub fn mean_latency_ticks(&self) -> f64 {
        if self.completed == 0 {
            0.0
        } else {
            self.latency_sum_ticks as f64 / self.completed as f64
        }
    }
}

/// Per-window metrics exported by the server.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServerMetrics {
    pub processed: u64,
}

/// Everything observable about a run. A deployment prints the two `*_metrics` streams; tests
/// read whichever of these they need.
#[cfg_attr(feature = "cress", derive(SimOutputs))]
pub struct RpcOutputs<'a, T> {
    /// Requests that received their first response, at the client.
    pub completed: Stream<Completion, Process<'a, Client>, Unbounded, NoOrder, ExactlyOnce>,
    /// Ids given up on after `max_attempts` sends.
    pub abandoned: Stream<u64, Process<'a, Client>, Unbounded, NoOrder, ExactlyOnce>,
    /// Every request the client puts on the wire (first sends and re-sends).
    pub outgoing: Stream<Request<T>, Process<'a, Client>, Unbounded, TotalOrder, ExactlyOnce>,
    /// Every request the server serves.
    pub processed: Stream<Request<T>, Process<'a, Server>, Unbounded, TotalOrder, ExactlyOnce>,
    /// Depth of the server's backlog at the end of each server tick.
    pub backlog_trace: Stream<usize, Process<'a, Server>, Unbounded, TotalOrder, ExactlyOnce>,
    /// `(window index, metrics)` once per `client_report_tick`.
    pub client_metrics:
        Stream<(usize, ClientMetrics), Process<'a, Client>, Unbounded, TotalOrder, ExactlyOnce>,
    /// `(window index, metrics, backlog depth)` once per `server_report_tick`.
    pub server_metrics: Stream<
        (usize, ServerMetrics, usize),
        Process<'a, Server>,
        Unbounded,
        TotalOrder,
        ExactlyOnce,
    >,
}

/// Builds the client/server program. `requests` is the application's request stream at the
/// client; see the module docs for the meaning of the three clock parameters.
///
/// Request bodies must be `Ord` because re-sends are sorted so that the client's outgoing
/// stream is totally ordered; over a single connection the server then receives requests in
/// exactly that order.
///
/// The two `amplification_check` attributes are the corpus's two labeled configurations; run
/// them with `./scripts/cress hydro_test rpc_with_retries`. Both confine the data to
/// the first 125 rounds of the run so that the report's growth label can read the holds that
/// outlast the input; the three-attempt configuration is expected to read as stopping at a ceiling of twice the
/// input, since every request is re-sent at most twice.
#[cfg_attr(feature = "cress", amplification_check(
    name = three_attempts,
    T = u64,
    workload(requests = 2),
    workload_rounds = 125,
    policy = RetryPolicy { timeout_ticks: 40, max_attempts: 3 },
    server_config = ServerConfig { max_per_tick: 5, service_time: Duration::ZERO },
))]
#[cfg_attr(feature = "cress", amplification_check(
    name = one_attempt,
    T = u64,
    workload(requests = 2),
    workload_rounds = 125,
    policy = RetryPolicy { timeout_ticks: 40, max_attempts: 1 },
    server_config = ServerConfig { max_per_tick: 5, service_time: Duration::ZERO },
))]
#[expect(
    clippy::too_many_arguments,
    reason = "every input stream is a parameter so that the checker can feed each one"
)]
pub fn rpc_with_retries<'a, T>(
    client: &Process<'a, Client>,
    server: &Process<'a, Server>,
    requests: Stream<T, Process<'a, Client>, Unbounded, TotalOrder, ExactlyOnce>,
    client_clock: Stream<(), Process<'a, Client>, Unbounded>,
    client_report_tick: Stream<(), Process<'a, Client>, Unbounded>,
    server_report_tick: Stream<(), Process<'a, Server>, Unbounded>,
    policy: RetryPolicy,
    server_config: ServerConfig,
) -> RpcOutputs<'a, T>
where
    T: Clone + Ord + std::hash::Hash + std::fmt::Debug + Serialize + DeserializeOwned + 'a,
{
    let RetryPolicy {
        timeout_ticks,
        max_attempts,
    } = policy;
    let ServerConfig {
        max_per_tick,
        service_time,
    } = server_config;
    // `q!` closures can only capture primitives, so the duration crosses into runtime code as
    // nanos.
    let service_nanos = service_time.as_nanos() as u64;

    // Responses come back from the server, which is downstream of `outgoing` (below).
    let (responses_complete, responses) = client
        .forward_ref::<Stream<Response<T>, Process<'a, Client>, Unbounded, TotalOrder, ExactlyOnce>>(
        );

    // ---- Client: logical clock, outstanding table, timeouts, re-sends ------------------------
    let (outgoing, retried_out, completed, abandoned) = sliced! {
        // Each clock element is one logical tick, numbered from 0.
        let clock = use::batch(client_clock.enumerate(), nondet!(/** batching only shifts which client tick observes a clock element; every element still advances the clock */));
        // Ids are assigned in arrival order.
        let new_requests = use::batch(requests.enumerate(), nondet!(/** batching only shifts which client tick first sends a request, i.e. its timestamps */));
        let responses = use::batch(responses, nondet!(/** batching only affects when a completion is observed and when a timeout fires */));
        let mut outstanding = use::state_null::<KeyedSingleton<u64, InFlight<T>, Tick<_>, Bounded>>();
        // The client's logical clock: the highest tick number seen so far. A tick of this
        // block that receives no clock element (e.g. one triggered by responses alone) keeps
        // the previous value.
        let mut now = use::state(|l| l.singleton(q!(0u64)));
        let now_cur = clock.map(q!(|(i, _)| i as u64)).max().unwrap_or(now.clone());
        now = now_cur.clone();

        let new_requests = new_requests.map(q!(|(i, body)| Request { id: i as u64, body }));

        // A response completes an outstanding request. Responses to ids that are no longer
        // outstanding (duplicate answers to re-sends, or already-abandoned requests) are ignored.
        let responded_ids = responses.map(q!(|r| r.id)).unique();
        let completed = responded_ids
            .clone()
            .map(q!(|id| (id, ())))
            .into_keyed()
            .join_keyed_singleton(outstanding.clone())
            .entries()
            .cross_singleton(now_cur.clone())
            .map(q!(|((id, (_, inflight)), now)| Completion {
                id,
                latency_ticks: now - inflight.first_sent,
            }));

        // Everything still waiting is judged against the timeout exactly once per tick.
        let judged = outstanding
            .filter_key_not_in(responded_ids)
            .into_keyed_stream()
            .cross_singleton(now_cur.clone())
            .map(q!(move |(inflight, now)| {
                if now - inflight.last_sent < timeout_ticks {
                    Verdict::Keep(inflight)
                } else if inflight.attempt < max_attempts {
                    Verdict::Retry(InFlight {
                        body: inflight.body,
                        first_sent: inflight.first_sent,
                        last_sent: now,
                        attempt: inflight.attempt + 1,
                    })
                } else {
                    Verdict::Abandon(inflight)
                }
            }));

        let kept = judged.clone().filter_map(q!(|v| match v {
            Verdict::Keep(f) => Some(f),
            _ => None,
        }));
        let retried = judged.clone().filter_map(q!(|v| match v {
            Verdict::Retry(f) => Some(f),
            _ => None,
        }));
        let abandoned = judged
            .filter_map(q!(|v| match v {
                Verdict::Abandon(f) => Some(f),
                _ => None,
            }))
            .keys();

        // Re-sends are sorted so the client's outgoing stream is totally ordered.
        let retries_out = retried
            .clone()
            .entries()
            .map(q!(|(id, f)| Request { id, body: f.body }))
            .sort();
        // What the client knows about its own re-sends, for its metrics.
        let retried_out = retried.clone().entries().map(q!(|(id, f)| (id, f.attempt)));

        let newly_sent = new_requests.clone().cross_singleton(now_cur.clone()).map(q!(|(r, now)| (
            r.id,
            InFlight {
                body: r.body,
                first_sent: now,
                last_sent: now,
                attempt: 1,
            }
        )));

        // Keys of `kept`, `retried` and `newly_sent` are pairwise disjoint, so `first()` is exact.
        outstanding = kept
            .chain(retried)
            .chain(newly_sent.into_keyed())
            .first();

        (
            new_requests.chain(retries_out),
            retried_out,
            completed,
            abandoned,
        )
    };

    // ---- Server: FIFO backlog, bounded work per tick ------------------------------------
    // Requests wait in `backlog`, a Hydro-level queue that persists across ticks. Each tick
    // dequeues at most `max_per_tick` requests (and, when deployed, burns `service_time` of CPU
    // on each), so a tick never runs longer than `max_per_tick * service_time`. Keeping ticks
    // short matters on a deployment: a DFIR node only reads its sockets between ticks, and a
    // node blocked writing to a full socket cannot read either.
    let incoming = outgoing.clone().send(server, TCP.fail_stop().bincode());

    let (processed, backlog_depth, backlog_trace) = sliced! {
        let arrivals = use::batch(incoming, nondet!(/** arrivals are appended to the FIFO; batching only affects how many share a tick */));
        let mut backlog = use::state_null::<Stream<Request<T>, Tick<_>, Bounded, TotalOrder>>();

        let queued = backlog.chain(arrivals).enumerate();
        let served = queued
            .clone()
            .filter_map(q!(move |(i, req)| if i < max_per_tick as usize { Some(req) } else { None }));
        backlog = queued.filter_map(q!(move |(i, req)| if i >= max_per_tick as usize { Some(req) } else { None }));

        let processed = served.map(q!(move |req| {
            if service_nanos > 0 {
                let deadline = std::time::Instant::now() + Duration::from_nanos(service_nanos);
                while std::time::Instant::now() < deadline {
                    std::hint::spin_loop();
                }
            }
            req
        }));

        let depth = backlog.clone().count();
        (processed, depth.clone(), depth.into_stream())
    };

    responses_complete.complete(
        processed
            .clone()
            .map(q!(|req| Response {
                id: req.id,
                body: req.body,
            }))
            .send(client, TCP.fail_stop().bincode()),
    );

    // ---- Server metrics -----------------------------------------------------------------
    let server_metrics = sliced! {
        let punctuation = use::batch(server_report_tick.enumerate(), nondet!(/** metrics window boundary */));
        let processed = use::batch(processed.clone(), nondet!(/** metrics window boundary */));
        let backlog_depth = use::snapshot(backlog_depth, nondet!(/** metrics window boundary */));
        let mut acc = use::state(|l| l.singleton(q!(ServerMetrics::default())));

        let window_end = punctuation.first();
        let in_batch = processed.count();
        let merged = acc.clone().zip(in_batch).map(q!(|(a, n)| ServerMetrics {
            processed: a.processed + n as u64,
        }));

        let report = merged
            .clone()
            .filter_if(window_end.clone().is_some())
            .zip(window_end.clone())
            .zip(backlog_depth)
            .map(q!(|((m, (i, _)), backlog)| (i, m, backlog)));
        acc = merged
            .filter_if(window_end.is_none())
            .unwrap_or(acc.clone().map(q!(|_| ServerMetrics::default())));

        report.into_stream()
    };

    // ---- Client metrics -----------------------------------------------------------------
    let client_metrics = sliced! {
        let punctuation = use::batch(client_report_tick.enumerate(), nondet!(/** metrics window boundary */));
        let completed = use::batch(completed.clone(), nondet!(/** metrics window boundary */));
        let sent = use::batch(outgoing.clone(), nondet!(/** metrics window boundary */));
        let retried = use::batch(retried_out, nondet!(/** metrics window boundary */));
        let abandoned = use::batch(abandoned.clone(), nondet!(/** metrics window boundary */));
        let mut acc = use::state(|l| l.singleton(q!(ClientMetrics::default())));

        let window_end = punctuation.first();
        let from_completed = completed.fold(
            q!(ClientMetrics::default),
            q!(|m, c: Completion| {
                m.completed += 1;
                m.latency_sum_ticks += c.latency_ticks;
                m.latency_max_ticks = m.latency_max_ticks.max(c.latency_ticks);
            }, commutative = manual_proof!(/** sum, count and max are commutative */)),
        );
        let from_sent = sent.count();
        let from_retried = retried.count();
        let from_abandoned = abandoned.count();

        let merged = acc
            .clone()
            .zip(from_completed)
            .zip(from_sent)
            .zip(from_retried)
            .zip(from_abandoned)
            .map(q!(|((((a, c), s), r), ab)| ClientMetrics {
                completed: a.completed + c.completed,
                latency_sum_ticks: a.latency_sum_ticks + c.latency_sum_ticks,
                latency_max_ticks: a.latency_max_ticks.max(c.latency_max_ticks),
                sent: a.sent + s as u64,
                retried: a.retried + r as u64,
                abandoned: a.abandoned + ab as u64,
            }));

        let report = merged
            .clone()
            .filter_if(window_end.clone().is_some())
            .zip(window_end.clone())
            .map(q!(|(m, (i, _))| (i, m)));
        acc = merged
            .filter_if(window_end.is_none())
            .unwrap_or(acc.clone().map(q!(|_| ClientMetrics::default())));

        report.into_stream()
    };

    RpcOutputs {
        completed,
        abandoned,
        outgoing,
        processed,
        backlog_trace,
        client_metrics,
        server_metrics,
    }
}

/// An open-loop workload for the tests: a fixed number of requests per client tick, with a
/// window in which the rate is higher. This lives in the harness, not in the program.
#[derive(Clone, Copy, Debug)]
pub struct Workload {
    /// Requests per client tick outside the trigger window.
    pub baseline_per_tick: u32,
    /// Requests per client tick inside the trigger window.
    pub trigger_per_tick: u32,
    /// Trigger window, in client ticks since start: `[trigger_start_tick, trigger_end_tick)`.
    pub trigger_start_tick: u64,
    pub trigger_end_tick: u64,
}

impl Workload {
    pub fn requests_at(&self, tick: u64) -> u32 {
        if tick >= self.trigger_start_tick && tick < self.trigger_end_tick {
            self.trigger_per_tick
        } else {
            self.baseline_per_tick
        }
    }
}

/// Simulation of the program under a fixed, fair schedule. A *round* is one element on
/// `client_clock` plus that tick's requests, followed by letting the simulation quiesce, so per
/// round the client runs one tick (send, judge timeouts, re-send), the server one tick (serve up
/// to `max_per_tick`), and the client one more tick to observe the responses. Capacity is
/// therefore `max_per_tick` per round and the timeout is `timeout_ticks` rounds. There is no
/// wall clock anywhere. Request bodies carry the tick they were issued in.
#[cfg(test)]
mod sim_tests {
    use std::collections::HashSet;

    use hydro_lang::sim::quiesce;

    use super::*;

    #[derive(Debug, Clone, Default)]
    struct Round {
        completed: u64,
        latency_sum: u64,
        abandoned: u64,
        /// Requests on the wire this round whose id had not been seen on the wire before.
        sent_first: u64,
        /// Requests on the wire this round whose id had been sent before: re-sends.
        sent_retry: u64,
        /// Requests served this round whose id had not been served before.
        served_first: u64,
        /// Requests served this round whose id had been served before: the server re-doing work.
        served_again: u64,
        /// Server backlog after this round's server tick (carried over if it did not run).
        backlog: usize,
    }

    /// Runs `rounds` rounds and returns one record per round.
    fn run(workload: Workload, policy: RetryPolicy, server_config: ServerConfig, rounds: usize) -> Vec<Round> {
        let mut flow = FlowBuilder::new();
        let client = flow.process::<Client>();
        let server = flow.process::<Server>();

        let (request_send, requests) = client.sim_input::<u64, TotalOrder, ExactlyOnce>();
        let (clock_send, client_clock) = client.sim_input::<(), TotalOrder, ExactlyOnce>();
        // Metrics windows are a deployment concern; these never fire.
        let (_client_report_send, client_report_tick) =
            client.sim_input::<(), TotalOrder, ExactlyOnce>();
        let (_server_report_send, server_report_tick) =
            server.sim_input::<(), TotalOrder, ExactlyOnce>();

        let outputs = rpc_with_retries(
            &client,
            &server,
            requests,
            client_clock,
            client_report_tick,
            server_report_tick,
            policy,
            server_config,
        );
        // Unordered outputs are read sorted, so map them to something `Ord`.
        let completed = outputs
            .completed
            .map(q!(|c| (c.id, c.latency_ticks)))
            .sim_output();
        let abandoned = outputs.abandoned.sim_output();
        let outgoing = outputs.outgoing.sim_output();
        let processed = outputs.processed.sim_output();
        let backlog_trace = outputs.backlog_trace.sim_output();

        let mut trace: Vec<Round> = Vec::with_capacity(rounds);
        let trace_ref = &mut trace;

        flow.sim().run_prompt(async move || {
            let mut backlog = 0usize;
            let mut sent_ids: HashSet<u64> = HashSet::new();
            let mut served_ids: HashSet<u64> = HashSet::new();
            for tick in 0..rounds as u64 {
                clock_send.send(());
                for _ in 0..workload.requests_at(tick) {
                    request_send.send(tick);
                }
                quiesce().await;

                let mut r = Round::default();
                for (_, latency) in completed.collect_sorted::<Vec<_>>().await {
                    r.completed += 1;
                    r.latency_sum += latency;
                }
                r.abandoned = abandoned.collect_sorted::<Vec<_>>().await.len() as u64;
                while let Some(req) = outgoing.try_next().await {
                    if sent_ids.insert(req.id) {
                        r.sent_first += 1
                    } else {
                        r.sent_retry += 1
                    }
                }
                while let Some(req) = processed.try_next().await {
                    if served_ids.insert(req.id) {
                        r.served_first += 1
                    } else {
                        r.served_again += 1
                    }
                }
                while let Some(depth) = backlog_trace.try_next().await {
                    backlog = depth;
                }
                r.backlog = backlog;
                trace_ref.push(r);
            }
        });
        trace
    }

    fn sum(trace: &[Round], from: usize, to: usize, f: impl Fn(&Round) -> u64) -> u64 {
        trace[from..to].iter().map(f).sum()
    }

    fn mean_latency(trace: &[Round], from: usize, to: usize) -> f64 {
        let completed = sum(trace, from, to, |r| r.completed);
        assert!(completed > 0, "no completions in rounds [{from}, {to})");
        sum(trace, from, to, |r| r.latency_sum) as f64 / completed as f64
    }

    /// Baseline 2 requests per round against a capacity of 5 per round (40% utilization).
    /// The trigger offers 12 per round for 60 rounds, building a backlog of ~420 -- a queueing
    /// delay of ~84 rounds, past the 40-round timeout. From then on every request is sent
    /// three times, so offered load is 3 x 2 = 6 per round against a capacity of 5 even
    /// after the trigger ends.
    pub(super) const WORKLOAD: Workload = Workload {
        baseline_per_tick: 2,
        trigger_per_tick: 12,
        trigger_start_tick: 100,
        trigger_end_tick: 160,
    };
    pub(super) const POLICY: RetryPolicy = RetryPolicy {
        timeout_ticks: 40,
        max_attempts: 3,
    };
    pub(super) const SERVER: ServerConfig = ServerConfig {
        max_per_tick: 5,
        service_time: Duration::ZERO,
    };

    pub(super) const ROUNDS: usize = 800;
    const TAIL_START: usize = 600;

    fn print_trajectory(trace: &[Round]) {
        for i in [0, 50, 99, 130, 159, 200, 250, 300, 400, 500, 600, 700, 799] {
            if i < trace.len() {
                let r = &trace[i];
                println!(
                    "round {i}: completed={} abandoned={} sent_first={} sent_retry={} served_first={} served_again={} backlog={}",
                    r.completed, r.abandoned, r.sent_first, r.sent_retry, r.served_first, r.served_again, r.backlog
                );
            }
        }
    }

    #[test]
    fn retries_cause_metastable_collapse() {
        let trace = run(WORKLOAD, POLICY, SERVER, ROUNDS);
        print_trajectory(&trace);

        // Baseline before the trigger: every request answered within the round it was sent.
        let pre = &trace[10..100];
        assert!(pre.iter().all(|r| r.backlog == 0), "backlog should be empty before the trigger");
        assert!(pre.iter().all(|r| r.completed == 2 && r.sent_retry == 0 && r.abandoned == 0));
        assert_eq!(mean_latency(&trace, 10, 100), 0.0);

        // Long after the trigger ended, offered load is back at baseline but the system has
        // not recovered.
        let tail = &trace[TAIL_START..];
        let tail_completed = sum(&trace, TAIL_START, ROUNDS, |r| r.completed);
        let tail_abandoned = sum(&trace, TAIL_START, ROUNDS, |r| r.abandoned);
        let tail_first = sum(&trace, TAIL_START, ROUNDS, |r| r.served_first);
        let tail_again = sum(&trace, TAIL_START, ROUNDS, |r| r.served_again);
        let baseline_goodput = 2 * (ROUNDS - TAIL_START) as u64;
        println!(
            "tail (rounds {TAIL_START}..{ROUNDS}): completed {tail_completed} (baseline would be {baseline_goodput}), abandoned {tail_abandoned}, served first={tail_first} again={tail_again}, backlog {} -> {}",
            tail.first().unwrap().backlog,
            tail.last().unwrap().backlog
        );
        assert!(
            tail_completed < baseline_goodput / 4,
            "goodput should have collapsed: {tail_completed} vs baseline {baseline_goodput}"
        );
        assert!(
            tail_again > tail_first,
            "the server should spend most of its capacity serving ids it has already served (first={tail_first}, again={tail_again})"
        );
        assert!(
            tail.last().unwrap().backlog > tail.first().unwrap().backlog,
            "the backlog should still be growing at the end of the run"
        );
        assert!(tail.windows(2).all(|w| w[1].backlog >= w[0].backlog), "backlog never shrinks in the tail");
    }

    /// Control: retries armed, no trigger. The loop never fires.
    #[test]
    fn without_a_trigger_the_system_stays_healthy() {
        let trace = run(
            Workload {
                trigger_per_tick: WORKLOAD.baseline_per_tick,
                ..WORKLOAD
            },
            POLICY,
            SERVER,
            ROUNDS,
        );
        print_trajectory(&trace);
        assert!(trace.iter().all(|r| r.sent_retry == 0 && r.abandoned == 0 && r.served_again == 0));
        assert!(trace.iter().all(|r| r.backlog == 0));
        assert!(trace[1..].iter().all(|r| r.completed == 2));
        assert_eq!(mean_latency(&trace, 0, ROUNDS), 0.0);
    }

    /// Control: same trigger, no retries. The backlog drains and latency returns to zero.
    #[test]
    fn without_retries_the_system_recovers() {
        let trace = run(WORKLOAD, RetryPolicy { max_attempts: 1, ..POLICY }, SERVER, ROUNDS);
        print_trajectory(&trace);
        assert!(trace.iter().all(|r| r.sent_retry == 0 && r.served_again == 0));
        let peak = trace.iter().map(|r| r.backlog).max().unwrap();
        println!("peak backlog {peak}; tail backlog {}", trace.last().unwrap().backlog);
        assert!(peak > 200, "the trigger should have built a backlog past the timeout, got {peak}");
        let tail = &trace[TAIL_START..];
        assert!(tail.iter().all(|r| r.backlog == 0 && r.completed == 2 && r.abandoned == 0));
        assert_eq!(mean_latency(&trace, TAIL_START, ROUNDS), 0.0);
    }
}

/// Wall-clock parameters of a deployment.
#[derive(Clone, Copy, Debug)]
pub struct Timing {
    /// Period of `client_clock`, and of the workload generator.
    pub clock_period: Duration,
    /// How often both sides print a metrics window.
    pub report_interval: Duration,
}

/// Deployment harness: wires a [`Workload`] generator and `source_interval` clocks into the
/// program and prints one metrics line per window on each side, which is how a deployed run is
/// observed. (Lives here rather than in the test module because `q!` closures must be part of
/// the crate that is staged onto the deployed hosts.)
#[cfg(feature = "tokio")]
pub fn deploy_with_workload<'a>(
    client: &Process<'a, Client>,
    server: &Process<'a, Server>,
    workload: Workload,
    policy: RetryPolicy,
    server_config: ServerConfig,
    timing: Timing,
) {
    let clock_nanos = timing.clock_period.as_nanos() as u64;
    let report_nanos = timing.report_interval.as_nanos() as u64;
    let Workload {
        baseline_per_tick,
        trigger_per_tick,
        trigger_start_tick,
        trigger_end_tick,
    } = workload;

    // One timer drives both the clock and the load generator, so a tick's requests are
    // issued in that tick. Request bodies carry the tick they were issued in.
    let ticks = client.source_interval(q!(Duration::from_nanos(clock_nanos)));
    let requests = ticks.clone().enumerate().flat_map_ordered(q!(move |(i, _)| {
        let tick = i as u64;
        let n = if tick >= trigger_start_tick && tick < trigger_end_tick {
            trigger_per_tick
        } else {
            baseline_per_tick
        };
        std::iter::repeat_n(tick, n as usize)
    }));
    let client_report_tick = client.source_interval(q!(Duration::from_nanos(report_nanos)));
    let server_report_tick = server.source_interval(q!(Duration::from_nanos(report_nanos)));

    let outputs = rpc_with_retries(
        client,
        server,
        requests,
        ticks,
        client_report_tick,
        server_report_tick,
        policy,
        server_config,
    );

    outputs.server_metrics.for_each(q!(|(i, m, backlog)| println!(
        "server t={}s processed={} backlog={}",
        i + 1,
        m.processed,
        backlog
    )));

    outputs.client_metrics.for_each(q!(|(i, m)| println!(
        "client t={}s completed={} mean_latency_ticks={:.2} max_latency_ticks={} sent={} retried={} abandoned={}",
        i + 1,
        m.completed,
        m.mean_latency_ticks(),
        m.latency_max_ticks,
        m.sent,
        m.retried,
        m.abandoned
    )));
}
