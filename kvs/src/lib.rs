#[cfg(stageleft_runtime)]
hydro_lang::setup!();

use std::collections::{HashMap, HashSet};

use hydro_lang::live_collections::stream::{NoOrder, TotalOrder};
use hydro_lang::prelude::*;
use serde::{Deserialize, Serialize};

pub mod combinators;

pub mod sidecar;

#[cfg(test)]
mod tests;

use combinators::{atomic_store, collect_quorum_responses, hrw_scatter};

/// The cluster of nodes that clients talk to. A router receives a request,
/// hashes the key to pick storage replicas, forwards the request, and waits
/// for a quorum of acknowledgements before replying to the client.
pub struct RouterNode;

/// The cluster of nodes that actually store data. Each storage node keeps a
/// grow-only set of values per key.
pub struct StorageNode;

/// How many storage replicas each key is written to (the top-N of the HRW
/// ranking).
pub const REPLICATION_FACTOR: usize = 3;
/// How many replica acknowledgements constitute a majority.
pub const QUORUM: usize = 2;
/// How many storage members must be discovered before requests are routed.
pub const STORAGE_MEMBERS: usize = 9;

/// The two operations the store supports.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum Op {
    /// Store `value` under `key`.
    Put { key: String, value: String },
    /// Look up all values stored under `key`.
    Get { key: String },
}

/// A request as it arrives at a router from a client. The `req_id` is supplied
/// by the client and is used to correlate the eventual response.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ClientRequest {
    pub req_id: u64,
    pub op: Op,
}

/// The response a router sends back to the client once a quorum is reached.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum ClientResponse {
    /// The `Put` with this id was applied in memory by a quorum of replicas.
    PutAck { req_id: u64 },
    /// The `Get` with this id resolved to the union of the quorum's values.
    GetResult {
        req_id: u64,
        values: HashSet<String>,
    },
}

/// Wires up the key-value store.
///
/// `requests` is a keyed stream of client requests arriving at each router,
/// keyed by the client's raw id (used only to route the response back).
/// Returns `(responses, storage_membership)`:
/// - `responses`: the stream of responses each router should deliver to its client.
/// - `storage_membership`: how many storage members each router currently sees.
///   Requests are buffered internally until [`STORAGE_MEMBERS`] have been
///   discovered. The stream remains useful for observability and tests.
pub fn kvs<'a>(
    routers: &Cluster<'a, RouterNode>,
    storage: &Cluster<'a, StorageNode>,
    requests: Stream<ClientRequest, Cluster<'a, RouterNode>, Unbounded, NoOrder>,
) -> (
    Stream<ClientResponse, Cluster<'a, RouterNode>, Unbounded, NoOrder>,
    Stream<usize, Cluster<'a, RouterNode>, Unbounded, NoOrder>,
) {
    // Log every client request as it arrives at a router. Under a real
    // deployment this shows up in that router's container logs (the dataflow's
    // `tracing` output is captured per container).
    let requests = requests.inspect(q!(|req| {
        tracing::info!(
            "kvs router received request: req_id={} op={:?}",
            req.req_id,
            req.op
        );
    }));

    // Route each request to the key's HRW-selected replicas. hrw_scatter
    // buffers requests until the expected storage membership is visible.
    let routed_requests = requests.map(q!(|req| {
        let key = match &req.op {
            Op::Put { key, .. } | Op::Get { key } => key.clone(),
        };
        (key, req)
    }));
    let (routed_requests, storage_membership) = hrw_scatter(
        storage,
        routed_requests,
        REPLICATION_FACTOR,
        STORAGE_MEMBERS,
    );
    let storage_requests = routed_requests
        .demux(
            storage,
            TCP.fail_stop().bincode().name("routers_to_storage"),
        )
        .entries();

    // Split and execute the KVS protocol directly at each storage member.
    let writes = storage_requests
        .clone()
        .filter_map(q!(|(router, req)| match req.op {
            Op::Put { key, value } => Some(((router, req.req_id), (key, value))),
            Op::Get { .. } => None,
        }))
        .inspect(q!(|((router, req_id), (key, value))| {
            tracing::info!(
                "kvs storage applying put: from_router={} req_id={} key={} value={}",
                router,
                req_id,
                key,
                value
            );
        }));
    let reads = storage_requests
        .filter_map(q!(|(router, req)| match req.op {
            Op::Get { key } => Some((router, req.req_id, key)),
            Op::Put { .. } => None,
        }))
        .inspect(q!(|(router, req_id, key)| {
            tracing::info!(
                "kvs storage serving get: from_router={} req_id={} key={}",
                router,
                req_id,
                key
            );
        }));

    let (write_acks, read_responses) = atomic_store(
        writes,
        reads,
        |writes| {
            writes.fold(
                q!(|| HashMap::<String, HashSet<String>>::new()),
                q!(
                    |map, (key, value)| {
                        map.entry(key).or_default().insert(value);
                    },
                    commutative = manual_proof!(/** per-key set insert is commutative */)
                ),
            )
        },
        |requests, snapshot| {
            let store_ref = snapshot.by_ref();
            requests.map(q!(move |(router, req_id, key)| {
                let values = store_ref.get(&key).cloned().unwrap_or_default();
                (router, req_id, values)
            }))
        },
    );

    // Route replica responses back to their originating router while retaining
    // the storage member ID supplied by demux.
    let write_acks = write_acks
        .map(q!(|(router, req_id)| (router, (req_id, ()))))
        .demux(
            routers,
            TCP.fail_stop().bincode().name("storage_write_acks"),
        );
    let read_responses = read_responses
        .map(q!(|(router, req_id, values)| (router, (req_id, values))))
        .demux(
            routers,
            TCP.fail_stop().bincode().name("storage_read_responses"),
        );

    // Deduplicate by (request, replica) before counting quorum responses.
    let unique_write_acks = write_acks
        .entries()
        .map(q!(|(replica, (req_id, value))| ((req_id, replica), value)))
        .into_keyed()
        .assume_ordering::<TotalOrder>(nondet!(
            /// Each replica emits at most one write acknowledgement per request.
        ))
        .first();
    let unique_read_responses = read_responses
        .entries()
        .map(q!(|(replica, (req_id, values))| (
            (req_id, replica),
            values
        )))
        .into_keyed()
        .assume_ordering::<TotalOrder>(nondet!(
            /// Each replica emits at most one read response per request.
        ))
        .first();

    let put_quorums = collect_quorum_responses(
        unique_write_acks,
        QUORUM,
        REPLICATION_FACTOR,
        nondet!(
            /// The output retains only the request ID, so which replicas
            /// acknowledged first is not observable.
        ),
    )
    .map(q!(|(req_id, _responses)| req_id));
    let read_quorums = collect_quorum_responses(
        unique_read_responses,
        QUORUM,
        REPLICATION_FACTOR,
        nondet!(
            /// A Get may observe values from writes concurrent with it depending
            /// on which replicas respond first. Every previously acknowledged
            /// value is present in every valid read quorum.
        ),
    );

    let put_responses = put_quorums
        .inspect(q!(|req_id| {
            tracing::info!("kvs router put quorum reached: req_id={}", req_id);
        }))
        .map(q!(|req_id| ClientResponse::PutAck { req_id }));

    let get_responses = read_quorums
        .map(q!(|(req_id, replica_values)| {
            let values = replica_values
                .into_values()
                .flatten()
                .collect::<HashSet<String>>();
            ClientResponse::GetResult { req_id, values }
        }))
        .inspect(q!(|resp| {
            if let ClientResponse::GetResult { req_id, values } = resp {
                tracing::info!(
                    "kvs router get quorum reached: req_id={} values={:?}",
                    req_id,
                    values
                );
            }
        }));

    (
        put_responses.merge_unordered(get_responses),
        storage_membership,
    )
}

/// Wires the key-value store up for a real (non-simulated) deployment.
///
/// Each router member runs a [`sidecar`] that listens on `client_port` for a
/// single external client, bridging that TCP connection to the dataflow: the
/// client's [`ClientRequest`]s become the router's request stream, and the
/// [`ClientResponse`]s the router produces are written back over the same
/// connection. Because the sidecar runs per member, a client connects to exactly
/// one router — matching the design where a request goes from a client to one
/// router node — with no separate gateway process and no external↔cluster
/// networking support required from the deploy backend.
///
/// `client_port` is the TCP port each router container listens on; the caller
/// exposes it and connects a client to a chosen router's endpoint.
pub fn kvs_deploy<'a>(
    routers: &Cluster<'a, RouterNode>,
    storage: &Cluster<'a, StorageNode>,
    client_port: u16,
) {
    // Each router member owns a sidecar TCP listener bridging a client to the
    // dataflow. `inbound` is this router's client-request stream.
    let (inbound, response_handle) =
        routers.sidecar_bidi::<ClientRequest, ClientResponse, _>(q!(move || {
            crate::sidecar::create(client_port)
        }));

    // Run the store; responses for a router's requests are produced at that same
    // router, so we hand them straight back to its sidecar.
    let (responses, _storage_membership) =
        kvs(routers, storage, inbound.weaken_ordering::<NoOrder>());

    response_handle.complete(responses);
}
