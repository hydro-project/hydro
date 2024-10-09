use std::collections::{HashMap, HashSet};
use std::time::{Duration, SystemTime};

use hydroflow_plus::*;
use serde::{Deserialize, Serialize};
use stageleft::*;
use tokio::time::Instant;

pub struct Client {}
pub struct Replica {}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug, Hash)]
pub struct Request {
    pub client_id: u32,
    pub operation: String,
    pub timestamp: u64,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug, Hash)]
pub struct Reply {
    pub view: u32,
    pub timestamp: u64,
    pub client_id: u32,
    pub replica_id: u32,
    pub result: String,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug, Hash)]
pub struct PrePrepare {
    pub view: u32,
    pub sequence_number: u64,
    pub request: Request,
    pub digest: String,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug, Hash)]
pub struct Prepare {
    pub view: u32,
    pub sequence_number: u64,
    pub digest: String,
    pub replica_id: u32,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug, Hash)]
pub struct Commit {
    pub view: u32,
    pub sequence_number: u64,
    pub digest: String,
    pub replica_id: u32,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug, Hash)]
pub struct ViewChange {
    pub view: u32,
    pub replica_id: u32,
    pub last_sequence_number: u64,
    pub P: Vec<Prepared>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug, Hash)]
pub struct NewView {
    pub view: u32,
    pub V: Vec<ViewChange>,
    pub O: Vec<PrePrepare>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug, Hash)]
pub struct Prepared {
    pub view: u32,
    pub sequence_number: u64,
    pub digest: String,
}

pub fn pbft<'a>(
    flow: &FlowBuilder<'a>,
    num_clients_per_node: usize,
    f: usize,
    throughput_window_size: usize,
    view_timeout: u64,
) -> (
    Cluster<Client>,
    Cluster<Replica>,
) {
    let clients = flow.cluster::<Client>();
    let replicas = flow.cluster::<Replica>();

    let client_id = flow.cluster_self_id(&clients);
    let replica_id = flow.cluster_self_id(&replicas);

    let num_replicas = 3 * f + 1;

    // Define the current view number, starting from 0
    let (current_view_cycle, current_view_stream) = flow.tick_cycle_with_initial(
        &replicas,
        flow.singleton(&replicas, q!(0u32)).latest_tick(),
    );
    let current_view = current_view_stream.latest();

    // Each replica determines if it's the primary (leader) for the current view
    let is_primary = current_view
        .map(q!(move |view| (*view % (num_replicas as u32)) == replica_id))
        .latest_tick();

    // Client sends requests
    let client_requests = flow.source_iter(&clients, q!([
        Request {
            client_id,
            operation: "operation1".to_string(),
            timestamp: 1,
        },
        Request {
            client_id,
            operation: "operation2".to_string(),
            timestamp: 2,
        },
        Request {
            client_id,
            operation: "operation3".to_string(),
            timestamp: 3,
        },
    ]));

    // Clients broadcast requests to all replicas
    let requests_to_replicas = client_requests.broadcast_bincode(&replicas);

    // Each replica filters requests intended for the current primary
    let requests_to_primary = requests_to_replicas
        .cross_singleton(current_view.clone())
        .filter(q!(move |(request, view)| {
            (*view % (num_replicas as u32)) == replica_id
        }))
        .map(q!(|(request, _)| request.clone()))
        .continue_if(is_primary.clone());

    // The primary processes the requests and creates PrePrepare messages
    let (sequence_number_cycle, sequence_number_stream) = flow.tick_cycle_with_initial(
        &replicas,
        flow.singleton(&replicas, q!(0u64)).latest_tick(),
    );
    let sequence_number = sequence_number_stream.latest();

    let pre_prepares = requests_to_primary
        .enumerate()
        .cross_singleton(current_view.clone())
        .cross_singleton(sequence_number.clone())
        .map(q!(move |(((index, request), view), seq_num)| PrePrepare {
            view: *view,
            sequence_number: *seq_num + index as u64 + 1,
            request: request.clone(),
            digest: format!("{:?}", request),
        }))
        .persist();

    // Update the sequence number for the next batch
    let next_sequence_number = sequence_number
        .cross_singleton(pre_prepares.clone().count())
        .map(q!(|(seq_num, count)| seq_num + count as u64))
        .defer_tick();
    sequence_number_cycle.complete(next_sequence_number.into());

    // The primary broadcasts PrePrepare messages to all replicas
    let pre_prepares_broadcast = pre_prepares.clone().broadcast_bincode(&replicas);

    // Replicas receive PrePrepare messages
    let received_pre_prepares = pre_prepares_broadcast.tick_batch();

    // Replicas verify PrePrepare messages and create Prepare messages
    let prepares = received_pre_prepares
        .cross_singleton(replica_id.clone())
        .map(q!(|pre_prepare, replica_id| Prepare {
            view: pre_prepare.view,
            sequence_number: pre_prepare.sequence_number,
            digest: pre_prepare.digest.clone(),
            replica_id,
        }))
        .broadcast_bincode(&replicas);

    // Replicas receive Prepare messages
    let received_prepares = prepares.tick_batch();

    // Collect Prepare messages
    let prepare_counts = received_prepares
        .map(q!(|prepare| ((prepare.view, prepare.sequence_number, prepare.digest.clone()), prepare.replica_id)))
        .fold_keyed(
            q!(|| HashSet::new()),
            q!(|set, replica_id| { set.insert(replica_id); }),
        );

    // Check if enough Prepare messages have been collected to move to the Commit phase
    let prepared_certificates = prepare_counts
        .filter(q!(move |(_, set)| set.len() >= 2 * f))
        .map(q!(|(key, _)| Prepared {
            view: key.0,
            sequence_number: key.1,
            digest: key.2.clone(),
        }))
        .persist();

    // Replicas create Commit messages and broadcast them
    let commits = prepared_certificates
        .cross_singleton(replica_id.clone())
        .map(q!(|cert, replica_id| Commit {
            view: cert.view,
            sequence_number: cert.sequence_number,
            digest: cert.digest.clone(),
            replica_id,
        }))
        .broadcast_bincode(&replicas);

    // Replicas receive Commit messages
    let received_commits = commits.tick_batch();

    // Collect Commit messages
    let commit_counts = received_commits
        .map(q!(|commit| ((commit.view, commit.sequence_number, commit.digest.clone()), commit.replica_id)))
        .fold_keyed(
            q!(|| HashSet::new()),
            q!(|set, replica_id| { set.insert(replica_id); }),
        );

    // Replicas execute requests after receiving enough Commit messages
    let executed_requests = commit_counts
        .filter(q!(move |(_, set)| set.len() >= 2 * f + 1))
        .map(q!(|(key, _)| {
            println!("Replica {} executed request at view {}, seq {}", replica_id, key.0, key.1);
            (key.1, key.2.clone())
        }))
        .persist();

    // Maintain the highest sequence number executed
    let highest_sequence_number_stream = executed_requests
        .map(q!(|(seq_num, _)| seq_num))
        .max()
        .unwrap_or(flow.singleton(&replicas, q!(0u64)).latest_tick());

    let highest_sequence_number = highest_sequence_number_stream.latest_tick();

    // View change mechanism

    // Define a timeout to detect primary failure
    let timeout_interval = flow
        .source_interval(&replicas, q!(Duration::from_secs(view_timeout)))
        .latest_tick();

    // Replicas send ViewChange messages upon timeout
    let view_changes = timeout_interval
        .cross_singleton(current_view.clone())
        .cross_singleton(replica_id.clone())
        .cross_singleton(highest_sequence_number.clone())
        .cross_singleton(prepared_certificates.clone().collect())
        .map(q!(move |(((((), view), replica_id), last_seq_num), prepared)| ViewChange {
            view: *view + 1,
            replica_id,
            last_sequence_number: *last_seq_num,
            P: prepared.clone(),
        }))
        .broadcast_bincode(&replicas);

    // Replicas receive ViewChange messages
    let received_view_changes = view_changes.tick_batch();

    let view_change_counts = received_view_changes
        .map(q!(|vc| (vc.view, vc)))
        .fold_keyed(
            q!(|| Vec::new()),
            q!(|vec, vc| { vec.push(vc); }),
        );

    // Check if enough ViewChange messages have been collected to form a NewView
    let new_views = view_change_counts
        .filter(q!(move |(_, vec)| vec.len() >= 2 * f + 1))
        .map(q!(|(view, vcs)| NewView {
            view,
            V: vcs.clone(),
            O: vec![], // This should be constructed based on V and P
        }))
        .broadcast_bincode(&replicas);

    // Replicas receive NewView messages and update the current view
    let received_new_views = new_views.tick_batch();

    let updated_view = received_new_views
        .map(q!(|nv| nv.view))
        .latest_tick();

    // Update the current view
    current_view_cycle.complete(updated_view.into());

    // Each replica determines if it is the new primary after view change
    let new_is_primary = updated_view
        .map(q!(move |view| (*view % (num_replicas as u32)) == replica_id))
        .latest_tick();

    // The new primary processes any pending requests from ViewChange messages and generates new PrePrepare messages
    // For simplicity, we'll assume there are no pending requests in this example
    // In practice, you would reconstruct O based on the V and P collections in the NewView message

    // Benchmark code similar to Paxos implementation

    // Track throughput

    // Define a timer for statistics output
    let stats_interval = flow.source_interval(&clients, q!(Duration::from_secs(1)));

    // Collect the number of executed requests
    let executed_requests_count = executed_requests
        .count()
        .continue_unless(stats_interval.clone().latest_tick())
        .map(q!(|count| (count, false)));

    let reset_throughput = stats_interval
        .clone()
        .latest_tick()
        .map(q!(|_| (0usize, true)))
        .defer_tick();

    let throughput = executed_requests_count
        .union(reset_throughput)
        .all_ticks()
        .fold(
            q!(|| 0usize),
            q!(|(total, (count, reset))| {
                if reset {
                    *total = 0;
                } else {
                    *total += count;
                }
            }),
        );

    // Output throughput
    stats_interval
        .cross_singleton(throughput.clone())
        .tick_samples()
        .for_each(q!(move |(_, total)| {
            println!("Throughput: {} requests/s", total);
        }));

    // Latency tracking

    // Create cycles to track request send and execution times
    let (request_times_cycle, request_times_stream) = flow.cycle(&clients);
    let request_times = request_times_stream.latest_tick();

    // When clients send requests, record the timestamp
    let client_request_times = client_requests
        .map(q!(move |_| (client_id, SystemTime::now())))
        .persist();

    request_times_cycle.complete(client_request_times.into());

    // When replicas execute requests, record the timestamp
    let execution_times = executed_requests
        .map(q!(move |(seq_num, digest)| (seq_num, digest, SystemTime::now())))
        .broadcast_bincode(&clients);

    // Clients receive execution times and calculate latency
    let latencies = execution_times
        .cross_singleton(request_times.clone())
        .map(q!(|((seq_num, digest, exec_time), (client_id, send_time))| {
            let latency = exec_time.duration_since(send_time).unwrap_or(Duration::ZERO);
            latency.as_millis()
        }))
        .collect()
        .latest_tick();

    // Calculate average latency over the window
    let average_latency = latencies
        .map(q!(move |latencies_vec| {
            if latencies_vec.is_empty() {
                0
            } else {
                let sum: u128 = latencies_vec.iter().sum();
                (sum / latencies_vec.len() as u128) as usize
            }
        }));

    // Output latency
    stats_interval
        .cross_singleton(average_latency)
        .tick_samples()
        .for_each(q!(move |(_, avg_latency)| {
            println!("Average Latency: {} ms", avg_latency);
        }));

    (
        clients,
        replicas,
    )
}
