#![allow(clippy::type_complexity)]

#[cfg(stageleft_runtime)]
hydro_lang::setup!();

#[cfg(test)]
mod tests;

/// Wire format types for the KVS protocol.
pub mod protocol;

/// gRPC (tonic) port — demonstrates that `bidi_external_sidecar` is
/// protocol-agnostic: same primitive, same dataflow, different wire protocol.
pub mod grpc_port;

/// WebSocket port — the third ingress, demonstrating that
/// `bidi_external_sidecar` extends to message-oriented, bidirectional
/// protocols and not just classic request-response HTTP.
pub mod ws_port;

/// Prost/protobuf serialization for inter-node traffic.
pub mod proto_codec;

/// Direct AppConfigData polling — exposes boolean feature flags to the
/// dataflow as `Unbounded` `Singleton`s. See `appconfig_bool_flag` below.
pub mod appconfig;

use std::collections::{HashMap, HashSet};
use std::hash::Hash;

use hydro_lang::live_collections::stream::{ExactlyOnce, NoOrder, TotalOrder};
use hydro_lang::location::cluster::CLUSTER_SELF_ID;
use hydro_lang::location::tick::NoTick;
use hydro_lang::location::{Location, MemberId, MembershipEvent};
use hydro_lang::networking::TCP;
use hydro_lang::prelude::*;
use lattices::map_union::MapUnion;
use lattices::set_union::SetUnionHashSet;
use lattices::{DomPair, Max};

type VectorClock = MapUnion<HashMap<String, Max<u64>>>;
type Clocked<T> = DomPair<VectorClock, T>;

/// The storage type for each key: a vector-clocked set of string values.
type ClockedSet = Clocked<SetUnionHashSet<String>>;

// ── Distributed Key-Value Store ─────────────────────────────────────

pub struct KvsRouter {}
pub struct KvsNode {}

/// Number of replicas to write/read for each key.
pub const REPLICATION_FACTOR: usize = 3;

// Re-export wire format types from the protocol module.
pub use protocol::{ClockedValue, Ingress, KvsCommand, KvsResponse, NodeCommand};

/// Helper to extract the node_ids from a KvsResponse.
pub fn response_node_ids(resp: &KvsResponse) -> &HashSet<String> {
    match resp {
        KvsResponse::PutOk { node_ids, .. } | KvsResponse::GetResult { node_ids, .. } => node_ids,
    }
}

/// Helper to extract the trace_id from a KvsCommand.
pub fn command_trace_id(cmd: &KvsCommand) -> &str {
    match cmd {
        KvsCommand::Put { trace_id, .. } | KvsCommand::Get { trace_id, .. } => trace_id,
    }
}

/// Helper to extract the key from a KvsCommand.
pub fn command_key(cmd: &KvsCommand) -> &str {
    match cmd {
        KvsCommand::Put { key, .. } | KvsCommand::Get { key, .. } => key,
    }
}

// ── Pure helper functions (extracted for testability) ────────────────

/// Build a KvsResponse for a given NodeCommand against the current store.
#[allow(clippy::disallowed_methods)] // iteration order irrelevant for building response VCs
pub fn build_node_response(
    cmd: NodeCommand,
    store: &HashMap<String, ClockedSet>,
    node_id: &str,
) -> KvsResponse {
    match cmd {
        NodeCommand::ClockedPut { trace_id, key, .. } => {
            let entry = store.get(&key);
            let existing_vc = entry.map(|dp| {
                dp.as_reveal_ref()
                    .0
                    .as_reveal_ref()
                    .iter()
                    .map(|(k, v)| (k.clone(), v.into_reveal()))
                    .collect()
            });
            KvsResponse::PutOk {
                trace_id,
                key,
                existing_vc,
                node_ids: HashSet::from([node_id.to_string()]),
            }
        }
        NodeCommand::Get { trace_id, key } => {
            let entry = store.get(&key);
            let value = entry.map(|dp| dp.as_reveal_ref().1.as_reveal_ref().clone());
            let existing_vc = entry.map(|dp| {
                dp.as_reveal_ref()
                    .0
                    .as_reveal_ref()
                    .iter()
                    .map(|(k, v)| (k.clone(), v.into_reveal()))
                    .collect()
            });
            KvsResponse::GetResult {
                trace_id,
                key,
                value,
                existing_vc,
                node_ids: HashSet::from([node_id.to_string()]),
            }
        }
    }
}

/// Compute the top-N rendezvous hash targets for a key among a set of members.
#[allow(clippy::disallowed_methods)] // iteration order irrelevant; output is sorted by hash score
pub fn rendezvous_targets<T: Hash + Clone + Eq>(
    key: &str,
    members: &std::collections::HashSet<T>,
    n: usize,
) -> Vec<T> {
    let mut scored: Vec<_> = members
        .iter()
        .map(|id| {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            key.hash(&mut hasher);
            id.hash(&mut hasher);
            (id.clone(), std::hash::Hasher::finish(&hasher))
        })
        .collect();
    scored.sort_by(|a, b| b.1.cmp(&a.1));
    scored.truncate(std::cmp::min(n, scored.len()));
    scored.into_iter().map(|(id, _)| id).collect()
}

/// The envelope key that flows through [`distributed_kvs`] alongside
/// every in-flight command: which ingress produced the command plus
/// the ingress-sidecar-local request id.
///
/// The sidecar-owned `u64` is preserved verbatim end-to-end — the
/// ingress layer never rewrites or inspects it. Ingress demux in
/// `complete_distributed_kvs` is done by matching on the [`Ingress`]
/// variant.
pub type ClientKey = (Ingress, u64);

/// Classify a merged response as either a client-facing response or a
/// read-phase response that should trigger the write phase.
///
/// Returns `Ok((client_id, response))` for client responses, or
/// `Err((real_client_id, key, existing_vc))` for read-phase responses.
pub fn classify_merged_response(
    client_id: ClientKey,
    resp: KvsResponse,
) -> Result<(ClientKey, KvsResponse), (ClientKey, String, String, Option<HashMap<String, u64>>)> {
    const READ_PHASE_OFFSET: u64 = 1_000_000_000;
    let (ingress, id) = client_id;
    match &resp {
        KvsResponse::PutOk { .. } => Ok((client_id, resp)),
        KvsResponse::GetResult { .. } if id < READ_PHASE_OFFSET => Ok((client_id, resp)),
        KvsResponse::GetResult {
            trace_id,
            key,
            existing_vc,
            ..
        } => Err((
            (ingress, id - READ_PHASE_OFFSET),
            trace_id.clone(),
            key.clone(),
            existing_vc.clone(),
        )),
    }
}

/// Build a ClockedSet with a vector clock that dominates the existing VC.
/// Used in the write phase of read-before-write.
pub fn build_dominating_clocked_put(
    value: String,
    existing_vc: Option<HashMap<String, u64>>,
    router_id: &str,
    seq: u64,
) -> ClockedSet {
    let mut vc: HashMap<String, Max<u64>> = existing_vc
        .unwrap_or_default()
        .into_iter()
        .map(|(k, v)| (k, Max::new(v)))
        .collect();
    vc.entry(router_id.to_string())
        .and_modify(|e| {
            if seq > e.into_reveal() {
                *e = Max::new(seq);
            }
        })
        .or_insert(Max::new(seq));
    DomPair::new(
        MapUnion::new(vc),
        SetUnionHashSet::new(HashSet::from([value])),
    )
}

/// The client ID offset used to distinguish read-phase Gets from client Gets.
pub const READ_PHASE_OFFSET: u64 = 1_000_000_000;

/// Split a client command into the appropriate internal representation.
/// Returns `(offset_client_id, internal_command)` pairs.
///
/// - Get → single `(client_id, Get)` for direct routing
/// - Put → `(client_id + OFFSET, Get)` for read phase, plus the pending put info
pub fn split_client_command(
    client_id: ClientKey,
    cmd: KvsCommand,
) -> (
    Option<(ClientKey, KvsCommand)>,
    Option<(ClientKey, KvsCommand)>,
    Option<(ClientKey, String, String, String)>,
) {
    let (ingress, id) = client_id;
    match cmd {
        KvsCommand::Get {
            ref trace_id,
            ref key,
        } => (
            Some((
                client_id,
                KvsCommand::Get {
                    trace_id: trace_id.clone(),
                    key: key.clone(),
                },
            )),
            None,
            None,
        ),
        KvsCommand::Put {
            trace_id,
            key,
            value,
        } => (
            None,
            Some((
                (ingress, id + READ_PHASE_OFFSET),
                KvsCommand::Get {
                    trace_id: trace_id.clone(),
                    key: key.clone(),
                },
            )),
            Some((client_id, trace_id, key, value)),
        ),
    }
}

/// Type alias for the shared store — wrapped in Rc for cheap cloning during snapshots.
pub type Store = std::rc::Rc<HashMap<String, ClockedSet>>;

/// Lattice-merge storage: merges two streams of `(key, ClockedSet)` into a
/// single `HashMap` singleton using CRDT merge semantics. Idempotent — duplicate
/// puts are no-ops.
pub fn kvs_storage<'a, L: Location<'a> + NoTick>(
    primary_puts: Stream<(String, ClockedSet), L, Unbounded, NoOrder, ExactlyOnce>,
    secondary_puts: Stream<(String, ClockedSet), L, Unbounded, NoOrder, ExactlyOnce>,
) -> Singleton<Store, L, Unbounded> {
    sliced! {
        let primary = use(primary_puts, nondet!(/** batch */));
        let secondary = use(secondary_puts, nondet!(/** batch */));
        let mut store = use::state(|l| l.singleton(q!(
            std::rc::Rc::new(HashMap::<String, ClockedSet>::new())
        )));

        let new_store = store.clone()
            .zip(primary.chain(secondary)
                .assume_ordering::<TotalOrder>(nondet!(/** lattice merge is order-independent */))
                .collect_vec())
            .map(q!(move |(mut store, puts): (
                std::rc::Rc<HashMap<String, ClockedSet>>,
                Vec<(String, ClockedSet)>,
            )| {
                let map = std::rc::Rc::make_mut(&mut store);
                for (key, clocked_value) in puts {
                    if let Some(existing) = map.get_mut(&key) {
                        lattices::Merge::merge(existing, clocked_value);
                    } else {
                        map.insert(key, clocked_value);
                    }
                }
                store
            }));
        store = new_store;
        store.clone()
    }
}

/// Process commands on a storage node: look up the store, build responses,
/// and attach this node's ID. Returns `(router_id, (client_id, response))`.
///
/// The `node_id_str` parameter provides the node's identity string for responses.
/// On a real cluster this comes from `CLUSTER_SELF_ID`; in tests it can be any string.
pub fn node_respond_with_id<
    'a,
    L: Location<'a> + NoTick,
    B: hydro_lang::live_collections::boundedness::Boundedness,
>(
    commands: Stream<(String, ClientKey, NodeCommand), L, Unbounded, NoOrder, ExactlyOnce>,
    store: Singleton<Store, L, B>,
    node_id_str: Singleton<String, L, Bounded>,
    tick: &Tick<L>,
) -> Stream<(String, (ClientKey, KvsResponse)), L, Unbounded, NoOrder, ExactlyOnce> {
    commands
        .batch(tick, nondet!(/** batch */))
        .cross_singleton(store.snapshot(tick, nondet!(/** snapshot */)))
        .cross_singleton(node_id_str.snapshot(tick, nondet!(/** snapshot */)))
        .map(q!(|(((router_id, client_id, cmd), store), nid): (
            ((String, ClientKey, NodeCommand), Store),
            String,
        )| {
            let resp = self::build_node_response(cmd, &store, &nid);
            (router_id, (client_id, resp))
        }))
        .all_ticks()
        .inspect(q!(|(router_id, (client_id, resp))| {
            let trace_id = match &resp {
                KvsResponse::PutOk { trace_id, .. } | KvsResponse::GetResult { trace_id, .. } => {
                    trace_id.as_str()
                }
            };
            tracing::debug!(name: "node_resp_send", %trace_id, ?router_id, ?client_id, ?resp);
        }))
}

/// Cluster-specific wrapper that uses `CLUSTER_SELF_ID` for the node identity.
fn node_respond<'a>(
    commands: Stream<
        (MemberId<KvsRouter>, ClientKey, NodeCommand),
        Cluster<'a, KvsNode>,
        Unbounded,
        NoOrder,
        ExactlyOnce,
    >,
    store: Singleton<Store, Cluster<'a, KvsNode>, Unbounded>,
    tick: &Tick<Cluster<'a, KvsNode>>,
) -> Stream<
    (MemberId<KvsRouter>, (ClientKey, KvsResponse)),
    Cluster<'a, KvsNode>,
    Unbounded,
    NoOrder,
    ExactlyOnce,
> {
    let self_id_str = commands
        .location()
        .source_iter(q!(vec![CLUSTER_SELF_ID.clone().into_tagless().to_string()]))
        .fold(q!(|| String::new()), q!(|acc, v| *acc = v));

    commands
        .batch(tick, nondet!(/** batch */))
        .cross_singleton(store.snapshot(tick, nondet!(/** snapshot */)))
        .cross_singleton(self_id_str.snapshot(tick, nondet!(/** snapshot */)))
        .map(q!(|(((router_id, client_id, cmd), store), nid): (
            ((MemberId<KvsRouter>, ClientKey, NodeCommand), Store),
            String,
        )| {
            let resp = self::build_node_response(cmd, &store, &nid);
            (router_id, (client_id, resp))
        }))
        .all_ticks()
        .inspect(q!(|(router_id, (client_id, resp))| {
            let trace_id = match &resp {
                KvsResponse::PutOk { trace_id, .. } | KvsResponse::GetResult { trace_id, .. } => {
                    trace_id.as_str()
                }
            };
            tracing::debug!(name: "node_resp_send", %trace_id, ?router_id, ?client_id, ?resp);
        }))
}

/// Merge replicated responses from storage nodes. Accumulates partial responses
/// keyed by `(client_id, key)` and emits a completed `(client_id, KvsResponse)`
/// once `REP_FACTOR` node_ids have been collected.
pub fn merge_responses<'a, L: Location<'a> + NoTick, const REP_FACTOR: usize>(
    incoming: Stream<(ClientKey, KvsResponse), L, Unbounded, NoOrder, ExactlyOnce>,
) -> Stream<(ClientKey, KvsResponse), L, Unbounded, NoOrder, ExactlyOnce> {
    sliced! {
        let batch = use(incoming, nondet!(/** batch boundaries don't affect final merged result */));
        let mut pending = use::state(|l| l.singleton(q!(
            HashMap::<(ClientKey, String), KvsResponse>::new()
        )));

        let merge_result = pending.clone()
            .zip(batch.assume_ordering::<TotalOrder>(nondet!(/** merge is order-independent */)).collect_vec())
            .flat_map_unordered(q!(move |(mut acc, new_resps): (
                HashMap<(ClientKey, String), KvsResponse>,
                Vec<(ClientKey, KvsResponse)>,
            )| {
                let replication_factor = REP_FACTOR; // bind const generic so q!() captures it
                for (client_id, resp) in &new_resps {
                    let key_str = match resp {
                        KvsResponse::PutOk { key, .. } => key.clone(),
                        KvsResponse::GetResult { key, .. } => key.clone(),
                    };
                    let merge_key = (*client_id, key_str);
                    let entry = acc.entry(merge_key);
                    match entry {
                        std::collections::hash_map::Entry::Vacant(e) => {
                            e.insert(resp.clone());
                        }
                        std::collections::hash_map::Entry::Occupied(mut e) => {
                            match (e.get_mut(), resp) {
                                (
                                    KvsResponse::PutOk { existing_vc: existing_evc, node_ids: existing_ids, .. },
                                    KvsResponse::PutOk { existing_vc: new_evc, node_ids: new_ids, .. },
                                ) => {
                                    for id in new_ids {
                                        existing_ids.insert(id.clone());
                                    }
                                    // Merge existing VCs: take max per entry
                                    match (existing_evc.as_mut(), new_evc.as_ref()) {
                                        (Some(a), Some(b)) => {
                                            for (k, v) in b {
                                                a.entry(k.clone())
                                                    .and_modify(|e| { if *v > *e { *e = *v; } })
                                                    .or_insert(*v);
                                            }
                                        },
                                        (None, Some(b)) => {
                                            *existing_evc = Some(b.clone());
                                        },
                                        _ => {}
                                    }
                                }
                                (
                                    KvsResponse::GetResult { value: existing, existing_vc: existing_evc, node_ids: existing_ids, .. },
                                    KvsResponse::GetResult { value: new_val, existing_vc: new_evc, node_ids: new_ids, .. },
                                ) => {
                                    match (existing.as_mut(), new_val.as_ref()) {
                                        (Some(a), Some(b)) => {
                                            for v in b {
                                                a.insert(v.clone());
                                            }
                                        },
                                        (None, Some(b)) => {
                                            *existing = Some(b.clone());
                                        },
                                        _ => {}
                                    }
                                    // Merge existing VCs: take max per entry
                                    match (existing_evc.as_mut(), new_evc.as_ref()) {
                                        (Some(a), Some(b)) => {
                                            for (k, v) in b {
                                                a.entry(k.clone())
                                                    .and_modify(|e| { if *v > *e { *e = *v; } })
                                                    .or_insert(*v);
                                            }
                                        },
                                        (None, Some(b)) => {
                                            *existing_evc = Some(b.clone());
                                        },
                                        _ => {}
                                    }
                                    for id in new_ids {
                                        existing_ids.insert(id.clone());
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }

                let mut completed: Vec<(ClientKey, KvsResponse)> = Vec::new();
                acc.retain(|(client_id, _key), resp| {
                    let count = match resp {
                        KvsResponse::PutOk { ref node_ids, .. } => node_ids.len(),
                        KvsResponse::GetResult { ref node_ids, .. } => node_ids.len(),
                    };
                    let trace_id = match resp {
                        KvsResponse::PutOk { trace_id, .. } | KvsResponse::GetResult { trace_id, .. } => trace_id.as_str(),
                    };
                    tracing::debug!(name: "merge_check", %trace_id, ?client_id, %count, needed = replication_factor);
                    if count >= replication_factor {
                        completed.push((*client_id, resp.clone()));
                        false
                    } else {
                        true
                    }
                });

                let mut out: Vec<Result<HashMap<(ClientKey, String), KvsResponse>, (ClientKey, KvsResponse)>> = Vec::new();
                out.push(Ok(acc));
                for c in completed {
                    out.push(Err(c));
                }
                out
            }));

        let new_pending = merge_result.clone()
            .filter_map(q!(|r: Result<HashMap<(ClientKey, String), KvsResponse>, (ClientKey, KvsResponse)>| r.ok()))
            .assume_ordering::<TotalOrder>(nondet!(/** exactly one Ok element per slice */))
            .first()
            .unwrap_or(pending.clone());
        pending = new_pending;

        merge_result
            .filter_map(q!(|r: Result<HashMap<(ClientKey, String), KvsResponse>, (ClientKey, KvsResponse)>| r.err()))
    }
}

/// Core KVS logic: takes typed commands on routers, returns typed responses.
/// This is the sim-testable core, separated from the external TCP/JSON layer.
/// Route commands to storage nodes using rendezvous hashing with replication.
pub fn route_commands<'a, const REP_FACTOR: usize>(
    router_commands: Stream<
        (ClientKey, KvsCommand),
        Cluster<'a, KvsRouter>,
        Unbounded,
        NoOrder,
        ExactlyOnce,
    >,
    node_members: Singleton<
        std::collections::HashSet<MemberId<KvsNode>>,
        Cluster<'a, KvsRouter>,
        Unbounded,
    >,
    router_self_id_str: Singleton<String, Cluster<'a, KvsRouter>, Bounded>,
) -> Stream<
    (
        MemberId<KvsNode>,
        (MemberId<KvsRouter>, ClientKey, NodeCommand),
    ),
    Cluster<'a, KvsRouter>,
    Unbounded,
    NoOrder,
    ExactlyOnce,
> {
    sliced! {
        let batch = use(router_commands, nondet!(/** batch */));
        let members = use(node_members, nondet!(/** snapshot membership */));
        let router_id = use(router_self_id_str, nondet!(/** snapshot router id */));
        let mut seq = use::state(|l| l.singleton(q!(0u64)));

        let result = seq.clone()
            .zip(batch.assume_ordering::<TotalOrder>(nondet!(/** process commands in order */)).collect_vec())
            .zip(members)
            .zip(router_id)
            .flat_map_unordered(q!(move |(((mut seq, cmds), member_ids), router_id_str): (
                ((u64, Vec<(ClientKey, KvsCommand)>), std::collections::HashSet<MemberId<KvsNode>>),
                String,
            )| {
                let my_router_id = CLUSTER_SELF_ID.clone();
                let replication_factor = REP_FACTOR;
                let mut transfers: Vec<Result<u64, (MemberId<KvsNode>, (MemberId<KvsRouter>, ClientKey, NodeCommand))>> = Vec::new();
                for (client_id, cmd) in cmds {
                    if member_ids.is_empty() {
                        let trace_id = self::command_trace_id(&cmd);
                        tracing::warn!(name: "route_no_members", %trace_id, ?client_id);
                        continue;
                    }
                    let key = match &cmd {
                        KvsCommand::Put { key, .. } | KvsCommand::Get { key, .. } => key.clone(),
                    };

                    let node_cmd = match cmd {
                        KvsCommand::Put { trace_id, key: k, value } => {
                            seq += 1;
                            let mut vc = HashMap::new();
                            vc.insert(router_id_str.clone(), Max::new(seq));
                            let clocked_value = DomPair::new(
                                MapUnion::new(vc),
                                SetUnionHashSet::new(HashSet::from([value])),
                            );
                            NodeCommand::ClockedPut {
                                trace_id,
                                key: k,
                                clocked_value: protocol::clocked_set_to_cv(clocked_value),
                            }
                        }
                        KvsCommand::Get { trace_id, key: k } => NodeCommand::Get { trace_id, key: k },
                    };

                    let targets = self::rendezvous_targets(&key, &member_ids, replication_factor);
                    for target in targets {
                        let trace_id = match &node_cmd { NodeCommand::ClockedPut { trace_id, .. } | NodeCommand::Get { trace_id, .. } => trace_id.as_str() };
                        tracing::debug!(name: "route_to_node", %trace_id, ?client_id, ?target, ?my_router_id);
                        transfers.push(Err((target, (my_router_id.clone(), client_id, node_cmd.clone()))));
                    }
                }
                transfers.push(Ok(seq));
                transfers
            }));

        seq = result.clone()
            .filter_map(q!(|r: Result<u64, _>| r.ok()))
            .assume_ordering::<TotalOrder>(nondet!(/** exactly one Ok per slice */))
            .first()
            .unwrap_or(seq.clone());

        result
            .filter_map(q!(|r: Result<u64, (MemberId<KvsNode>, (MemberId<KvsRouter>, ClientKey, NodeCommand))>| r.err()))
    }
}

/// Core distributed KVS dataflow. Takes a stream of `(client_id, KvsCommand)`
/// on the router cluster, returns a stream of `(client_id, KvsResponse)` on
/// the same cluster. Contains routing, replication, quorum merging, and
/// rebalancing — but no ingress/egress (HTTP, TCP). This shape lets
/// the full pipeline be exercised by the sim harness (see `fuzz_distributed_kvs`)
/// without spinning up any network I/O.
///
/// For the production setup that wires this up to gRPC and WebSocket servers via
/// [`Location::bidi_external_sidecar`], see [`complete_distributed_kvs`].
pub fn distributed_kvs<'a, const REP_FACTOR: usize>(
    commands: KeyedStream<
        ClientKey,
        KvsCommand,
        Cluster<'a, KvsRouter>,
        Unbounded,
        TotalOrder,
        ExactlyOnce,
    >,
    routers: &Cluster<'a, KvsRouter>,
    nodes: &Cluster<'a, KvsNode>,
) -> Stream<(ClientKey, KvsResponse), Cluster<'a, KvsRouter>, Unbounded, NoOrder, ExactlyOnce> {
    let commands = commands.inspect_with_key(q!(|(client_id, cmd)| {
        let trace_id = self::command_trace_id(cmd);
        tracing::debug!(name: "command_parsed", %trace_id, ?client_id, ?cmd);
    }));

    let router_commands = commands.entries();

    // ── Core KVS logic with read-before-write ───────────────────────
    // Membership: track active storage node IDs on each router
    let node_members = routers
        .source_cluster_members(nodes)
        .entries()
        .inspect(q!(|(id, event)| {
            let trace_id = format!("mem-{:08x}", rand::random::<u32>());
            tracing::info!(name: "membership_event", %trace_id, ?id, ?event);
        }))
        .assume_ordering::<TotalOrder>(nondet!(/** membership events are processed in order */))
        .fold(
            q!(|| std::collections::HashSet::<MemberId<KvsNode>>::new()),
            q!(|set, (id, event)| {
                match event {
                    MembershipEvent::Joined => {
                        set.insert(id);
                    }
                    MembershipEvent::Left => {
                        set.remove(&id);
                    }
                }
            }),
        );

    let router_self_id_str = routers
        .source_iter(q!(vec![CLUSTER_SELF_ID.clone().into_tagless().to_string()]))
        .fold(q!(|| String::new()), q!(|acc, v| *acc = v));

    // ── Split commands: Gets route directly, Puts become two-phase ──
    // For Puts, we use internal client_ids (negative offset) for the read phase
    // to distinguish read-phase Get responses from client Get responses.

    // Client Gets: route directly as NodeCommand::Get
    let client_gets = router_commands.clone().filter_map(q!(|(client_id, cmd)| {
        let (get, _, _) = self::split_client_command(client_id, cmd);
        get
    }));

    // Client Puts: send a Get for the same key (read phase)
    let put_read_phase = router_commands.clone().filter_map(q!(|(client_id, cmd)| {
        let (_, read, _) = self::split_client_command(client_id, cmd);
        read
    }));

    // Track pending puts keyed by client_id so we can match them with read responses
    let pending_puts = router_commands.filter_map(q!(|(client_id, cmd)| {
        let (_, _, pending) = self::split_client_command(client_id, cmd);
        pending
    }));

    // Route client Gets and read-phase Gets to nodes separately
    let routed_client_gets = route_commands::<REP_FACTOR>(
        client_gets,
        node_members.clone(),
        router_self_id_str.clone(),
    );
    let routed_read_phase = route_commands::<REP_FACTOR>(
        put_read_phase,
        node_members.clone(),
        router_self_id_str.clone(),
    );

    // ── Write phase: forward-declare the put channel ────────────────
    let (put_cycle, put_commands) = routers.forward_ref::<Stream<
        (
            MemberId<KvsNode>,
            (MemberId<KvsRouter>, ClientKey, NodeCommand),
        ),
        _,
        Unbounded,
        NoOrder,
    >>();

    // Merge all routed commands into a single stream via sliced on nodes

    // Demux each source to nodes separately, then complete the cycle
    let gets_on_nodes = routed_client_gets
        .map(q!(|(target, (rid, cid, cmd)): (
            MemberId<KvsNode>,
            (MemberId<KvsRouter>, ClientKey, NodeCommand)
        )| {
            let bytes = self::proto_codec::encode_router_to_node(cid, &cmd);
            (target, (rid, bytes))
        }))
        .demux(
            nodes,
            TCP.fail_stop().bincode().name("router_to_nodes_gets"),
        )
        .entries()
        .map(q!(|(_from, (rid, bytes)): (
            MemberId<KvsRouter>,
            (MemberId<KvsRouter>, Vec<u8>)
        )| {
            let (cid, cmd) = self::proto_codec::decode_router_to_node(&bytes);
            (rid, cid, cmd)
        }));

    let reads_on_nodes = routed_read_phase
        .map(q!(|(target, (rid, cid, cmd)): (
            MemberId<KvsNode>,
            (MemberId<KvsRouter>, ClientKey, NodeCommand)
        )| {
            let bytes = self::proto_codec::encode_router_to_node(cid, &cmd);
            (target, (rid, bytes))
        }))
        .demux(
            nodes,
            TCP.fail_stop().bincode().name("router_to_nodes_reads"),
        )
        .entries()
        .map(q!(|(_from, (rid, bytes)): (
            MemberId<KvsRouter>,
            (MemberId<KvsRouter>, Vec<u8>)
        )| {
            let (cid, cmd) = self::proto_codec::decode_router_to_node(&bytes);
            (rid, cid, cmd)
        }));

    let puts_on_nodes = put_commands
        .map(q!(|(target, (rid, cid, cmd)): (
            MemberId<KvsNode>,
            (MemberId<KvsRouter>, ClientKey, NodeCommand)
        )| {
            let bytes = self::proto_codec::encode_router_to_node(cid, &cmd);
            (target, (rid, bytes))
        }))
        .demux(
            nodes,
            TCP.fail_stop().bincode().name("router_to_nodes_puts"),
        )
        .entries()
        .map(q!(|(_from, (rid, bytes)): (
            MemberId<KvsRouter>,
            (MemberId<KvsRouter>, Vec<u8>)
        )| {
            let (cid, cmd) = self::proto_codec::decode_router_to_node(&bytes);
            (rid, cid, cmd)
        }));

    // Use sliced to merge the three unbounded streams
    let on_node_all = sliced! {
        let g = use(gets_on_nodes, nondet!(/** batch */));
        let r = use(reads_on_nodes, nondet!(/** batch */));
        let p = use(puts_on_nodes, nondet!(/** batch */));
        g.chain(r).chain(p)
    }
    .inspect(q!(|(router_id, client_id, cmd)| {
        let trace_id = match &cmd {
            NodeCommand::ClockedPut { trace_id, .. } | NodeCommand::Get { trace_id, .. } => {
                trace_id.as_str()
            }
        };
        tracing::debug!(name: "node_cmd_recv", %trace_id, ?router_id, ?client_id, ?cmd);
    }));

    let on_node_commands = on_node_all
        .clone()
        .map(q!(|(_, client_id, cmd)| (client_id, cmd)));

    // ── Rebalancing: track node membership on storage nodes ─────────
    let node_node_members = nodes
        .source_cluster_members(nodes)
        .entries()
        .inspect(q!(|(id, event)| {
            let trace_id = format!("mem-{:08x}", rand::random::<u32>());
            tracing::info!(name: "node_membership_event", %trace_id, ?id, ?event);
        }))
        .assume_ordering::<TotalOrder>(nondet!(/** membership events are processed in order */))
        .fold(
            q!(|| std::collections::HashSet::<MemberId<KvsNode>>::new()),
            q!(|set, (id, event)| {
                match event {
                    MembershipEvent::Joined => {
                        set.insert(id);
                    }
                    MembershipEvent::Left => {
                        set.remove(&id);
                    }
                }
            }),
        );

    // Storage: nodes receive pre-clocked puts from routers and lattice-merge them
    let node_puts_from_routers =
        on_node_commands
            .clone()
            .filter_map(q!(|(_client_id, cmd)| match cmd {
                NodeCommand::ClockedPut {
                    key, clocked_value, ..
                } => Some((key, protocol::cv_to_clocked_set(clocked_value))),
                _ => None,
            }));

    // Forward-declare the rebalance channel so we can merge it into storage
    let (rebalance_cycle, rebalance_incoming) =
        nodes.forward_ref::<Stream<(String, ClockedSet), _, Unbounded, NoOrder>>();

    // Build unified storage from router puts and rebalance transfers using sliced merge
    let node_storage = kvs_storage(node_puts_from_routers, rebalance_incoming);

    let node_tick = nodes.tick();

    // ── Rebalancing: when membership changes, send keys to their current owners ──
    // Track previous membership set to detect changes and avoid continuous rebalancing.
    // Each node sends every key it holds to all other top-N owners for that key.
    // The lattice merge at the receiver makes duplicate sends idempotent.
    let rebalance_transfers = sliced! {
        let members = use(node_node_members, nondet!(/** snapshot membership */));
        let storage = use(node_storage.clone(), nondet!(/** snapshot storage */));
        let mut prev_members = use::state(|l| l.singleton(q!(
            std::collections::HashSet::<MemberId<KvsNode>>::new()
        )));

        let transfers = prev_members.clone()
            .zip(members)
            .zip(storage)
            .flat_map_unordered(q!(move |((prev, member_ids), store): (
                (std::collections::HashSet<MemberId<KvsNode>>, std::collections::HashSet<MemberId<KvsNode>>),
                Store,
            )| {
                // Only rebalance when membership set has actually changed
                if member_ids == prev || member_ids.len() <= 1 {
                    return vec![Ok(member_ids)];
                }
                let self_id = CLUSTER_SELF_ID.clone();
                let trace_id = format!("rebal-{:08x}", rand::random::<u32>());
                tracing::info!(name: "rebalance_triggered", %trace_id, prev = prev.len(), cur = member_ids.len(), keys = store.len());
                let mut transfers: Vec<Result<std::collections::HashSet<MemberId<KvsNode>>, (MemberId<KvsNode>, (String, ClockedSet))>> = Vec::new();
                for (key, value) in &*store {
                    let owners = self::rendezvous_targets(key, &member_ids, REP_FACTOR);
                    for target in &owners {
                        if *target != self_id {
                            transfers.push(Err((target.clone(), (key.clone(), value.clone()))));
                        }
                    }
                }
                tracing::info!(name: "rebalance_batch", %trace_id, count = transfers.len());
                transfers.push(Ok(member_ids));
                transfers
            }));

        prev_members = transfers.clone()
            .filter_map(q!(move |r: Result<std::collections::HashSet<MemberId<KvsNode>>, _>| r.ok()))
            .assume_ordering::<TotalOrder>(nondet!(/** exactly one Ok per slice */))
            .first()
            .unwrap_or(prev_members.clone());

        transfers
            .filter_map(q!(|r: Result<std::collections::HashSet<MemberId<KvsNode>>, (MemberId<KvsNode>, (String, ClockedSet))>| r.err()))
    };

    // Send rebalance transfers to target nodes
    let rebalance_at_target = rebalance_transfers
        .map(q!(|(target, (key, cs)): (
            MemberId<KvsNode>,
            (String, ClockedSet)
        )| {
            let cv = protocol::clocked_set_to_cv(cs);
            let bytes = self::proto_codec::encode_rebalance(&key, &cv);
            (target, bytes)
        }))
        .demux(nodes, TCP.fail_stop().bincode().name("node_rebalance"));

    // Extract the (key, value) from incoming rebalance messages
    let rebalance_puts = rebalance_at_target
        .entries()
        .map(q!(|(from_node, bytes): (MemberId<KvsNode>, Vec<u8>)| {
            let (key, cv) = self::proto_codec::decode_rebalance(&bytes);
            let cs = protocol::cv_to_clocked_set(cv);
            (from_node, (key, cs))
        }))
        .inspect(q!(|(from_node, (key, _value)): &(
            MemberId<KvsNode>,
            (String, ClockedSet)
        )| {
            let trace_id = format!("rebal-{:08x}", rand::random::<u32>());
            tracing::info!(name: "rebalance_recv", %trace_id, ?from_node, %key);
        }))
        .map(q!(|(_from_node, kv): (
            MemberId<KvsNode>,
            (String, ClockedSet)
        )| kv));

    rebalance_cycle.complete(rebalance_puts);

    let node_responses = node_respond(on_node_all, node_storage.clone(), &node_tick);

    // ── Storage nodes → Routers ─────────────────────────────────────
    // Demux responses back to the originating router using router_id
    let responses_at_router = node_responses
        .map(q!(|(rid, (cid, resp)): (
            MemberId<KvsRouter>,
            (ClientKey, KvsResponse)
        )| {
            let bytes = self::proto_codec::encode_node_to_router(cid, &resp);
            (rid, bytes)
        }))
        .into_keyed()
        .demux(routers, TCP.fail_stop().bincode().name("nodes_to_router"));

    // ── Merge replicated responses at each router ───────────────────
    let incoming = responses_at_router
        .entries()
        .map(q!(|(from_node, bytes): (MemberId<KvsNode>, Vec<u8>)| {
            let (client_id, resp) = self::proto_codec::decode_node_to_router(&bytes);
            (from_node, (client_id, resp))
        }))
        .inspect(q!(|(from_node, (client_id, resp))| {
            let trace_id = match &resp {
                KvsResponse::PutOk { trace_id, .. } | KvsResponse::GetResult { trace_id, .. } => {
                    trace_id.as_str()
                }
            };
            tracing::debug!(name: "router_resp_recv", %trace_id, ?from_node, ?client_id, ?resp);
        }))
        .map(q!(|(_from_node, (client_id, resp)): (
            MemberId<KvsNode>,
            (ClientKey, KvsResponse)
        )| (client_id, resp)));

    let merged_responses = merge_responses::<_, REP_FACTOR>(incoming);

    // ── Split merged responses: client responses vs read-phase responses ──
    // Read-phase Get responses (client_id >= 1B) trigger the write phase.
    // Client Get responses and PutOk responses go to the client.

    // Client responses: Gets (client_id < 1B) and PutOks (any client_id)
    let client_responses = merged_responses.clone().filter_map(q!(|(client_id, resp)| {
        self::classify_merged_response(client_id, resp).ok()
    }));

    // Read-phase responses: GetResults with offset client_id
    let read_phase_responses = merged_responses.filter_map(q!(|(client_id, resp)| {
        self::classify_merged_response(client_id, resp).err()
    }));

    // ── Write phase: build ClockedPuts from read-phase responses + pending puts ──
    // Match read-phase responses with pending puts and build dominating VCs.
    let write_phase_commands = sliced! {
        let reads = use(read_phase_responses, nondet!(/** batch */));
        let puts = use(pending_puts, nondet!(/** batch */));
        let members = use(node_members, nondet!(/** snapshot */));
        let router_id = use(router_self_id_str, nondet!(/** snapshot */));
        let mut seq = use::state(|l| l.singleton(q!(0u64)));
        let mut pending = use::state(|l| l.singleton(q!(
            HashMap::<ClientKey, (String, String, String)>::new()
        )));

        // Accumulate new pending puts
        let new_pending = pending.clone()
            .zip(puts.assume_ordering::<TotalOrder>(nondet!(/** order */)).collect_vec())
            .map(q!(move |(mut pending, new_puts): (
                HashMap<ClientKey, (String, String, String)>,
                Vec<(ClientKey, String, String, String)>,
            )| {
                for (client_id, trace_id, key, value) in new_puts {
                    pending.insert(client_id, (trace_id, key, value));
                }
                pending
            }));

        // Process read-phase responses: match with pending, build ClockedPuts
        let result = new_pending
            .zip(reads.assume_ordering::<TotalOrder>(nondet!(/** order */)).collect_vec())
            .zip(seq.clone())
            .zip(members)
            .zip(router_id)
            .flat_map_unordered(q!(move |((((mut pending, read_resps), mut seq), member_ids), router_id_str): (
                (((HashMap<ClientKey, (String, String, String)>, Vec<(ClientKey, String, String, Option<HashMap<String, u64>>)>), u64), std::collections::HashSet<MemberId<KvsNode>>),
                String,
            )| {
                let my_router_id = CLUSTER_SELF_ID.clone();
                let replication_factor = REP_FACTOR;
                let mut out: Vec<Result<(HashMap<ClientKey, (String, String, String)>, u64), (MemberId<KvsNode>, (MemberId<KvsRouter>, ClientKey, NodeCommand))>> = Vec::new();

                for (client_id, _trace_id, key, existing_vc) in read_resps {
                    if let Some((put_trace_id, put_key, value)) = pending.remove(&client_id) {
                        if put_key != key {
                            tracing::warn!(name: "read_phase_key_mismatch", %put_trace_id, ?client_id, %put_key, %key);
                            continue;
                        }

                        seq += 1;
                        let clocked_value = self::build_dominating_clocked_put(
                            value, existing_vc, &router_id_str, seq,
                        );
                        let node_cmd = NodeCommand::ClockedPut {
                            trace_id: put_trace_id.clone(),
                            key: key.clone(),
                            clocked_value: protocol::clocked_set_to_cv(clocked_value),
                        };

                        // Route to same nodes as the read phase
                        let targets = self::rendezvous_targets(&key, &member_ids, replication_factor);
                        for target in targets {
                            tracing::debug!(name: "write_phase_to_node", %put_trace_id, ?client_id, ?target, ?my_router_id);
                            out.push(Err((target, (my_router_id.clone(), client_id, node_cmd.clone()))));
                        }
                    }
                }

                out.push(Ok((pending, seq)));
                out
            }));

        pending = result.clone()
            .filter_map(q!(|r: Result<(HashMap<ClientKey, (String, String, String)>, u64), _>| r.ok().map(|(p, _)| p)))
            .assume_ordering::<TotalOrder>(nondet!(/** one Ok */))
            .first()
            .unwrap_or(pending.clone());

        seq = result.clone()
            .filter_map(q!(|r: Result<(HashMap<ClientKey, (String, String, String)>, u64), _>| r.ok().map(|(_, s)| s)))
            .assume_ordering::<TotalOrder>(nondet!(/** one Ok */))
            .first()
            .unwrap_or(seq.clone());

        result
            .filter_map(q!(|r: Result<_, (MemberId<KvsNode>, (MemberId<KvsRouter>, ClientKey, NodeCommand))>| r.err()))
    };

    put_cycle.complete(write_phase_commands);

    client_responses.inspect(q!(|(client_id, resp)| {
        let trace_id = match &resp {
            KvsResponse::PutOk { trace_id, .. } | KvsResponse::GetResult { trace_id, .. } => {
                trace_id
            }
        };
        tracing::info!(name: "request_end", %trace_id, ?client_id, ?resp);
    }))
}

/// Expose an AppConfig boolean flag to the dataflow as an `Unbounded`
/// `Singleton`. Each router member polls AppConfig directly (see
/// `appconfig::appconfig_bool_stream`); the latest observed value is
/// folded into a `Singleton<bool>` that dataflow stages can read (via
/// `cross_singleton` etc.) to branch on.
///
/// The `Singleton` starts at `false` and updates on each successful
/// poll. On poll errors (transient network issue, missing profile in
/// local dev, etc.) the last-known-good value is retained.
///
/// Resource identifiers are resolved from environment variables at
/// runtime: the application and environment ids come from
/// `APPCONFIG_APPLICATION_ID` and `APPCONFIG_ENVIRONMENT_ID`, and the
/// per-flag profile id comes from the env var named by
/// `profile_env_var`. The CDK stack is responsible for setting these
/// on each router task. If any of the three is missing, polling is
/// skipped and the flag stays at `false`.
///
/// Intended use: fork the dataflow between old and new behavior on a
/// deploy cut-over by keying branch selection off this flag.
pub fn appconfig_bool_flag<'a>(
    routers: &Cluster<'a, KvsRouter>,
    profile_env_var: &'static str,
) -> Singleton<bool, Cluster<'a, KvsRouter>, Unbounded> {
    routers
        .source_stream(q!(self::appconfig::appconfig_bool_stream(profile_env_var)))
        .fold(q!(|| false), q!(|acc: &mut bool, v: bool| *acc = v))
}

/// Production wrapper: registers **two** sidecars on every router
/// member — tonic/gRPC and a tokio-tungstenite WebSocket server — and
/// merges both ingresses into a single [`distributed_kvs`] pipeline.
/// Responses are demuxed back to the originating sidecar by matching on
/// the [`Ingress`] variant carried as part of every [`ClientKey`]; each
/// sidecar sees exactly the request ids it issued, untouched.
///
/// Each sidecar runs on a TCP listener that Hydro Deploy has bound and
/// exposed through Docker/ECS port mapping. The two returned
/// [`ExternalBytesPort`]s are what the caller passes to
/// `DeployResult::get_all_tcp_endpoints` to discover each router
/// member's host-visible address per protocol.
pub fn complete_distributed_kvs<'a, const REP_FACTOR: usize, Ext>(
    external: &hydro_lang::prelude::External<'a, Ext>,
    routers: &Cluster<'a, KvsRouter>,
    nodes: &Cluster<'a, KvsNode>,
) -> (
    hydro_lang::location::external_process::ExternalBytesPort<
        hydro_lang::location::external_process::Many,
    >,
    hydro_lang::location::external_process::ExternalBytesPort<
        hydro_lang::location::external_process::Many,
    >,
) {
    // gRPC/HTTP-2 ingress. Envelope: `(u64, KvsCommand)` /
    // `(u64, KvsResponse)` — the `u64` is a sidecar-owned per-request
    // id that doubles as both routing key (keyed into the
    // `HashMap<u64, oneshot::Sender>` in the sidecar) and uniqueness
    // source for `merge_responses`.
    let (grpc_port, grpc_commands, grpc_response_sink) = routers
        .bidi_external_sidecar::<Ext, (u64, KvsCommand), (u64, KvsResponse), _>(
            external,
            q!(|listener, cmds_tx, resp_rx| {
                self::grpc_port::kvs_grpc_sidecar(listener, cmds_tx, resp_rx)
            }),
        );

    // WebSocket ingress. Envelope: `(SeqId, KvsCommand)` /
    // `(SeqId, KvsResponse)` — same shape as gRPC but with different
    // semantics: the `SeqId` is a sidecar-assigned per-command
    // uniquifier, and the sidecar maintains an internal
    // `HashMap<SeqId, ConnId>` side-table to route responses back to
    // the originating connection. See `ws_port` module docs.
    let (ws_port, ws_commands, ws_response_sink) = routers
        .bidi_external_sidecar::<
            Ext,
            (self::ws_port::SeqId, KvsCommand),
            (self::ws_port::SeqId, KvsResponse),
            _,
        >(
            external,
            q!(|listener, cmds_tx, resp_rx| {
                self::ws_port::kvs_ws_sidecar(listener, cmds_tx, resp_rx)
            }),
        );

    // Example AppConfig feature flag — sampled on every router member and
    // observable in structured logs. This is wired here purely as a
    // demonstration of how to expose a runtime config flag to the
    // dataflow (intended use-case: cut-over between old and new code
    // paths after a deploy). The value is exposed via the
    // `APPCONFIG_USE_NEW_VERSION_PROFILE_ID` env var that the CDK stack
    // sets on each router task. Real uses would snapshot the singleton
    // into a tick and `cross_singleton` with the command stream to
    // branch execution.
    let use_new_version: Singleton<bool, Cluster<'a, KvsRouter>, Unbounded> =
        appconfig_bool_flag(routers, "APPCONFIG_USE_NEW_VERSION_PROFILE_ID");

    // Proof-of-life: log the flag's current snapshot once per router
    // tick. The `nondet!` acknowledgement records that the observed value
    // can be at any point in time relative to commands flowing through
    // the dataflow — fine for observation, intentional for cut-overs.
    let router_tick = routers.tick();
    use_new_version
        .snapshot(
            &router_tick,
            nondet!(/** observational sampling of a config flag */),
        )
        .all_ticks()
        .for_each(q!(|v: bool| {
            tracing::info!(name: "appconfig_use_new_version", use_new_version = %v);
        }));

    // Pack each sidecar's incoming stream into the dataflow's
    // envelope key. The key type is `ClientKey = (Ingress, u64)`;
    // `Ingress` tags the ingress for demux, and the `u64` is the
    // per-ingress uniquifier consumed by `merge_responses`'
    // quorum-merge HashMap.
    //
    // Both sidecars happen to use a `(u64, T)` envelope where the
    // `u64` is sidecar-assigned, globally-unique per command. Their
    // *semantics* differ — gRPC treats it as a per-request correlation
    // id matched against an internal oneshot map, WS treats it as a
    // sidecar-internal uniquifier looked up in a `seq_id → conn_id`
    // side-table — but the envelope shape is the same, so
    // `complete_distributed_kvs` is uniform across both.
    //
    // This is not imposed by the framework: the primitive is
    // `(InT, OutT)`-parametric, and each sidecar chose `(u64, T)` for
    // its own reasons. A sidecar whose reply target is truly streaming
    // (e.g. append-only log) would pick a different envelope.
    let tagged_grpc = grpc_commands
        .map(q!(|(id, cmd): (u64, KvsCommand)| (
            (self::Ingress::Grpc, id),
            cmd,
        )))
        .into_keyed();
    let tagged_ws = ws_commands
        .map(q!(|(id, cmd): (u64, KvsCommand)| (
            (self::Ingress::Ws, id),
            cmd,
        )))
        .into_keyed();
    // merge_unordered weakens to NoOrder; coerce back to TotalOrder —
    // distributed_kvs treats commands as independently routed, so no
    // inter-command ordering dependency exists between the ingresses.
    let merged_commands = tagged_grpc
        .merge_unordered(tagged_ws)
        .assume_ordering::<TotalOrder>(nondet!(
            /** commands from different ingresses are independent */
        ));

    let responses = distributed_kvs::<REP_FACTOR>(merged_commands, routers, nodes);

    // Demux: filter by Ingress variant, strip the tag, hand the
    // sidecar-owned `u64` back to its originating sidecar untouched.
    let responses_to_grpc = responses.clone().filter_map(q!(|((ingress, id), resp): (
        self::ClientKey,
        KvsResponse
    )| {
        if matches!(ingress, self::Ingress::Grpc) {
            Some((id, resp))
        } else {
            None
        }
    }));
    let responses_to_ws = responses.filter_map(q!(|((ingress, id), resp): (
        self::ClientKey,
        KvsResponse
    )| {
        if matches!(ingress, self::Ingress::Ws) {
            Some((id, resp))
        } else {
            None
        }
    }));

    grpc_response_sink.complete(responses_to_grpc);
    ws_response_sink.complete(responses_to_ws);

    (grpc_port, ws_port)
}

/// Shared KVS test logic that can be used by both local examples and
/// remote integration tests. Works over raw TCP with length-delimited framing.
pub mod testing;

/// A pass-through codec that delivers raw bytes without framing.
///
/// Functionally equivalent to [`tokio_util::codec::BytesCodec`], but defined
/// here in the library crate so stageleft can resolve its type path.
///
/// ## Why not just use `tokio_util::codec::BytesCodec`?
///
/// Stageleft-generated trybuild code (the `examples/hy-kvs*` binaries produced
/// during `brazil-build release`) expands type arguments to their
/// **defining-module path**, not their re-exported path. `BytesCodec` lives in
/// the private module `tokio_util::codec::bytes_codec` and is only publicly
/// visible via a `pub use` at `tokio_util::codec::BytesCodec`. The generated
/// code therefore emits `tokio_util::codec::bytes_codec::BytesCodec`, which
/// fails to compile with:
///
/// ```text
/// error[E0603]: module `bytes_codec` is private
///    --> examples/hy-kvsrouter-loc2v1_XXXXXXXX.rs:23:75
///     |
///  23 |         amzn_hydro_project_demo_app::__staged::__deps::tokio_util::codec::bytes_codec::BytesCodec,
///     |                                                                           ^^^^^^^^^^^ private module
/// ```
///
/// ## Why must `RawCodec` live in `lib.rs`, not `examples/kvs.rs`?
///
/// Even though `RawCodec` is only *used* from `examples/kvs.rs`, it cannot be
/// defined there. The generated trybuild crate
/// (`amzn-hydro-project-demo-app-hydro-trybuild`, built separately at
/// `build/private/cargo-target/hydro_trybuild/`) depends only on this library
/// crate — it has no visibility into example binaries. Defining `RawCodec`
/// inside `examples/kvs.rs` causes stageleft to emit a reference like
/// `kvs::RawCodec`, and the trybuild crate fails with:
///
/// ```text
/// error[E0433]: failed to resolve: use of unresolved module or unlinked crate `kvs`
///   --> examples/hy-kvsrouter-loc2v1_XXXXXXXX.rs:23:9
///    |
/// 23 |         kvs::RawCodec,
///    |         ^^^ use of unresolved module or unlinked crate `kvs`
/// ```
///
/// So the type path used in a `bidi_external_many_bytes::<_, _, T>` call must
/// be reachable from both the example build (compiled against this library)
/// *and* the stageleft-generated trybuild crate (which depends on this
/// library). The only place that satisfies both is this library crate.
///
/// Do not move this type unless stageleft changes to respect re-exports or
/// `tokio_util` makes the `bytes_codec` module public.
#[derive(Default)]
pub struct RawCodec;

impl tokio_util::codec::Decoder for RawCodec {
    type Item = bytes::BytesMut;
    type Error = std::io::Error;
    fn decode(&mut self, buf: &mut bytes::BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if buf.is_empty() {
            Ok(None)
        } else {
            Ok(Some(buf.split()))
        }
    }
}

impl tokio_util::codec::Encoder<bytes::Bytes> for RawCodec {
    type Error = std::io::Error;
    fn encode(&mut self, data: bytes::Bytes, buf: &mut bytes::BytesMut) -> Result<(), Self::Error> {
        buf.extend_from_slice(&data);
        Ok(())
    }
}
