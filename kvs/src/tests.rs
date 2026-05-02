use std::collections::{HashMap, HashSet};

use hydro_lang::live_collections::stream::{NoOrder, TotalOrder};
use hydro_lang::prelude::*;
use lattices::map_union::MapUnion;
use lattices::set_union::SetUnionHashSet;
use lattices::{DomPair, Max};

use super::*;

#[test]
fn hydro_distributed_kvs_compiles() {
    let _ = super::distributed_kvs::<REPLICATION_FACTOR> as fn(_, &_, &_) -> _;
    let _ = super::complete_distributed_kvs::<REPLICATION_FACTOR, ()> as fn(&_, &_, &_) -> (_, _);
}

// ── Helper to build a ClockedSet for testing ────────────────────────

fn clocked(writer: &str, seq: u64, value: &str) -> ClockedSet {
    let mut vc = HashMap::new();
    vc.insert(writer.to_string(), Max::new(seq));
    DomPair::new(
        MapUnion::new(vc),
        SetUnionHashSet::new(HashSet::from([value.to_string()])),
    )
}

// ── kvs_storage tests ───────────────────────────────────────────────

#[test]
fn sim_storage_single_put() {
    let mut flow = FlowBuilder::new();
    let p = flow.process::<()>();

    let (put_port, put_stream) = p.sim_input::<(String, ClockedSet), TotalOrder, _>();
    // No secondary puts — use an empty stream via a second sim_input we never send to
    let (_unused, empty_stream) = p.sim_input::<(String, ClockedSet), TotalOrder, _>();

    let store = kvs_storage(
        put_stream.weaken_ordering::<NoOrder>(),
        empty_stream.weaken_ordering::<NoOrder>(),
    );
    let tick = p.tick();
    let out = store
        .snapshot(&tick, nondet!(/** snapshot */))
        .into_stream()
        .all_ticks()
        .sim_output();

    flow.sim().exhaustive(async || {
        put_port.send(("key1".to_string(), clocked("w", 1, "val1")));
        let snapshot = out.next().await.unwrap();
        assert_eq!(snapshot.len(), 1);
        assert!(snapshot.contains_key("key1"));
    });
}

#[test]
fn sim_storage_merge_concurrent_writes() {
    let mut flow = FlowBuilder::new();
    let p = flow.process::<()>();

    let (primary_port, primary) = p.sim_input::<(String, ClockedSet), TotalOrder, _>();
    let (secondary_port, secondary) = p.sim_input::<(String, ClockedSet), TotalOrder, _>();

    let store = kvs_storage(
        primary.weaken_ordering::<NoOrder>(),
        secondary.weaken_ordering::<NoOrder>(),
    );
    let tick = p.tick();
    let out = store
        .snapshot(&tick, nondet!(/** snapshot */))
        .into_stream()
        .all_ticks()
        .sim_output();

    flow.sim().exhaustive(async || {
        // Two concurrent writes to the same key from different writers
        primary_port.send(("k".to_string(), clocked("writer_a", 1, "a_val")));
        secondary_port.send(("k".to_string(), clocked("writer_b", 1, "b_val")));

        // May take multiple snapshots for both writes to merge (batching is non-deterministic)
        let mut snapshot = out.next().await.unwrap();
        // If only one write merged, consume the next snapshot
        if let Some(entry) = snapshot.get("k") {
            let values = entry.as_reveal_ref().1.as_reveal_ref();
            if values.len() < 2 {
                snapshot = out.next().await.unwrap();
            }
        }
        let entry = snapshot.get("k").unwrap();
        let values = entry.as_reveal_ref().1.as_reveal_ref();
        assert!(values.contains("a_val"), "missing a_val in {values:?}");
        assert!(values.contains("b_val"), "missing b_val in {values:?}");
    });
}

#[test]
fn sim_storage_sequential_overwrite() {
    let mut flow = FlowBuilder::new();
    let p = flow.process::<()>();

    let (put_port, put_stream) = p.sim_input::<(String, ClockedSet), TotalOrder, _>();
    let (_unused, empty) = p.sim_input::<(String, ClockedSet), TotalOrder, _>();

    let store = kvs_storage(
        put_stream.weaken_ordering::<NoOrder>(),
        empty.weaken_ordering::<NoOrder>(),
    );
    let tick = p.tick();
    let out = store
        .snapshot(&tick, nondet!(/** snapshot */))
        .into_stream()
        .all_ticks()
        .sim_output();

    flow.sim().exhaustive(async || {
        // Sequential writes: second dominates first via higher VC
        put_port.send(("k".to_string(), clocked("w", 1, "old")));
        let _ = out.next().await; // consume first snapshot

        put_port.send(("k".to_string(), clocked("w", 2, "new")));
        let snapshot = out.next().await.unwrap();
        let values = snapshot.get("k").unwrap().as_reveal_ref().1.as_reveal_ref();
        // DomPair with dominating VC replaces the value set
        assert!(values.contains("new"), "expected 'new' in {values:?}");
        assert!(
            !values.contains("old"),
            "should not contain 'old' in {values:?}"
        );
    });
}

// ── merge_responses tests ───────────────────────────────────────────

#[test]
fn sim_merge_completes_at_replication_factor() {
    let mut flow = FlowBuilder::new();
    let p = flow.process::<()>();

    let (in_port, in_stream) = p.sim_input::<(ClientKey, KvsResponse), TotalOrder, _>();
    let merged = merge_responses::<_, REPLICATION_FACTOR>(in_stream.weaken_ordering::<NoOrder>());
    let out = merged.sim_output();

    flow.sim().exhaustive(async || {
        // Send REPLICATION_FACTOR (3) responses for the same client_id + key
        in_port.send((
            (Ingress::Test, 42),
            KvsResponse::PutOk {
                trace_id: String::new(),
                existing_vc: None,
                key: "k".into(),
                node_ids: HashSet::from(["n1".into()]),
            },
        ));
        in_port.send((
            (Ingress::Test, 42),
            KvsResponse::PutOk {
                trace_id: String::new(),
                existing_vc: None,
                key: "k".into(),
                node_ids: HashSet::from(["n2".into()]),
            },
        ));
        in_port.send((
            (Ingress::Test, 42),
            KvsResponse::PutOk {
                trace_id: String::new(),
                existing_vc: None,
                key: "k".into(),
                node_ids: HashSet::from(["n3".into()]),
            },
        ));

        out.assert_yields_unordered([(
            (Ingress::Test, 42u64),
            KvsResponse::PutOk {
                trace_id: String::new(),
                existing_vc: None,
                key: "k".into(),
                node_ids: HashSet::from(["n1".into(), "n2".into(), "n3".into()]),
            },
        )])
        .await;
    });
}

#[test]
fn sim_merge_does_not_emit_before_quorum() {
    let mut flow = FlowBuilder::new();
    let p = flow.process::<()>();

    let (in_port, in_stream) = p.sim_input::<(ClientKey, KvsResponse), TotalOrder, _>();
    let merged = merge_responses::<_, REPLICATION_FACTOR>(in_stream.weaken_ordering::<NoOrder>());
    let out = merged.sim_output();

    flow.sim().exhaustive(async || {
        // Send only 2 of 3 needed responses — should not emit
        in_port.send((
            (Ingress::Test, 1),
            KvsResponse::PutOk {
                trace_id: String::new(),
                existing_vc: None,
                key: "k".into(),
                node_ids: HashSet::from(["n1".into()]),
            },
        ));
        in_port.send((
            (Ingress::Test, 1),
            KvsResponse::PutOk {
                trace_id: String::new(),
                existing_vc: None,
                key: "k".into(),
                node_ids: HashSet::from(["n2".into()]),
            },
        ));
        out.assert_no_more().await;
    });
}

#[test]
fn sim_merge_get_unions_values() {
    let mut flow = FlowBuilder::new();
    let p = flow.process::<()>();

    let (in_port, in_stream) = p.sim_input::<(ClientKey, KvsResponse), TotalOrder, _>();
    let merged = merge_responses::<_, REPLICATION_FACTOR>(in_stream.weaken_ordering::<NoOrder>());
    let out = merged.sim_output();

    flow.sim().exhaustive(async || {
        // Node 1 has the value, node 2 doesn't, node 3 has it
        in_port.send((
            (Ingress::Test, 1),
            KvsResponse::GetResult {
                trace_id: String::new(),
                key: "k".into(),
                value: Some(HashSet::from(["v".into()])),
                existing_vc: None,
                node_ids: HashSet::from(["n1".into()]),
            },
        ));
        in_port.send((
            (Ingress::Test, 1),
            KvsResponse::GetResult {
                trace_id: String::new(),
                key: "k".into(),
                value: None,
                existing_vc: None,
                node_ids: HashSet::from(["n2".into()]),
            },
        ));
        in_port.send((
            (Ingress::Test, 1),
            KvsResponse::GetResult {
                trace_id: String::new(),
                key: "k".into(),
                value: Some(HashSet::from(["v".into()])),
                existing_vc: None,
                node_ids: HashSet::from(["n3".into()]),
            },
        ));

        out.assert_yields_unordered([(
            (Ingress::Test, 1u64),
            KvsResponse::GetResult {
                trace_id: String::new(),
                key: "k".into(),
                value: Some(HashSet::from(["v".into()])),
                existing_vc: None,
                node_ids: HashSet::from(["n1".into(), "n2".into(), "n3".into()]),
            },
        )])
        .await;
    });
}

// ── route_commands tests ────────────────────────────────────────────

// ── route_commands tests ────────────────────────────────────────────

#[test]
fn sim_route_sequential_puts_have_increasing_seq() {
    let mut flow = FlowBuilder::new();
    let p = flow.process::<()>();

    let (cmd_port, cmd_stream) = p.sim_input::<(ClientKey, KvsCommand), TotalOrder, _>();

    // Provide membership as a static singleton (one fake node)
    let node_members = p.source_iter(q!(vec![0u32])).fold(
        q!(|| std::collections::HashSet::<u32>::new()),
        q!(|set, id| {
            set.insert(id);
        }),
    );

    let router_id = p
        .source_iter(q!(vec!["router_0".to_string()]))
        .fold(q!(|| String::new()), q!(|acc, v| *acc = v));

    // Inline the routing logic on a Process to test VC stamping directly
    let routed = sliced! {
        let batch = use(cmd_stream.weaken_ordering::<NoOrder>(), nondet!(/** batch */));
        let members = use(node_members, nondet!(/** snapshot */));
        let rid = use(router_id, nondet!(/** snapshot */));
        let mut seq = use::state(|l| l.singleton(q!(0u64)));

        let result = seq.clone()
            .zip(batch.assume_ordering::<TotalOrder>(nondet!(/** order */)).collect_vec())
            .zip(members)
            .zip(rid)
            .flat_map_unordered(q!(move |(((mut seq, cmds), _members), router_id_str): (
                ((u64, Vec<(ClientKey, KvsCommand)>), std::collections::HashSet<u32>),
                String,
            )| {
                let mut out: Vec<Result<u64, NodeCommand>> = Vec::new();
                for (_client_id, cmd) in cmds {
                    let node_cmd = match cmd {
                        KvsCommand::Put { trace_id: _, key, value } => {
                            seq += 1;
                            let mut vc = HashMap::new();
                            vc.insert(router_id_str.clone(), Max::new(seq));
                            let clocked_value = DomPair::new(
                                MapUnion::new(vc),
                                SetUnionHashSet::new(HashSet::from([value])),
                            );
                            NodeCommand::ClockedPut { trace_id: String::new(), key, clocked_value: protocol::clocked_set_to_cv(clocked_value) }
                        }
                        KvsCommand::Get { trace_id, key } => NodeCommand::Get { trace_id, key },
                    };
                    out.push(Err(node_cmd));
                }
                out.push(Ok(seq));
                out
            }));

        seq = result.clone()
            .filter_map(q!(|r: Result<u64, NodeCommand>| r.ok()))
            .assume_ordering::<TotalOrder>(nondet!(/** one Ok */))
            .first()
            .unwrap_or(seq.clone());

        result.filter_map(q!(|r: Result<u64, NodeCommand>| r.err()))
    };

    let out = routed
        .assume_ordering::<TotalOrder>(nondet!(/** single process */))
        .sim_output();

    flow.sim().exhaustive(async || {
        cmd_port.send((
            (Ingress::Test, 0u64),
            KvsCommand::Put {
                trace_id: String::new(),
                key: "k".into(),
                value: "v1".into(),
            },
        ));
        cmd_port.send((
            (Ingress::Test, 1u64),
            KvsCommand::Put {
                trace_id: String::new(),
                key: "k".into(),
                value: "v2".into(),
            },
        ));

        let cmd1 = out.next().await.unwrap();
        let cmd2 = out.next().await.unwrap();

        // Collect seq numbers keyed by value
        let mut seqs = HashMap::new();
        for cmd in [cmd1, cmd2] {
            match cmd {
                NodeCommand::ClockedPut { clocked_value, .. } => {
                    let cs = protocol::cv_to_clocked_set(clocked_value);
                    let vc = cs.as_reveal_ref().0.as_reveal_ref();
                    #[allow(clippy::disallowed_methods)] // single-entry VC; order irrelevant
                    let seq = vc.values().next().unwrap().into_reveal();
                    let vals = cs.as_reveal_ref().1.as_reveal_ref();
                    for v in vals {
                        seqs.insert(v.clone(), seq);
                    }
                }
                other => panic!("expected ClockedPut, got {other:?}"),
            }
        }

        let seq_v1 = seqs["v1"];
        let seq_v2 = seqs["v2"];
        assert_ne!(
            seq_v1, seq_v2,
            "sequential puts must have different seq numbers: v1={seq_v1}, v2={seq_v2}"
        );
    });
}

// ── Fuzz: distributed_kvs (typed KvsCommand in, typed KvsResponse out) ───

#[test]
fn fuzz_distributed_kvs() {
    let mut flow = FlowBuilder::new();
    let routers = flow.cluster::<KvsRouter>();
    let nodes = flow.cluster::<KvsNode>();

    let (cmd_port, input) = routers.sim_input::<(ClientKey, KvsCommand)>();
    let responses = distributed_kvs::<REPLICATION_FACTOR>(input.into_keyed(), &routers, &nodes);
    let resp_port = responses
        .assume_ordering::<TotalOrder>(nondet!(/** test */))
        .sim_cluster_output();

    flow.sim()
        .with_cluster_size(&routers, 2)
        .with_cluster_size(&nodes, 3)
        .unit_test_fuzz_iterations(10000)
        .fuzz(async || {
            // Send concurrent PUTs on router 0 and router 1
            cmd_port.send(
                0,
                (
                    (Ingress::Test, 0u64),
                    KvsCommand::Put {
                        trace_id: "t1".into(),
                        key: "k".into(),
                        value: "v1".into(),
                    },
                ),
            );
            cmd_port.send(
                1,
                (
                    (Ingress::Test, 1u64),
                    KvsCommand::Put {
                        trace_id: "t2".into(),
                        key: "k".into(),
                        value: "v2".into(),
                    },
                ),
            );

            let mut put_count = 0u32;
            for router_id in 0..2u32 {
                if let Some((cid, resp)) = resp_port.next(router_id).await {
                    assert_eq!(cid, (Ingress::Test, router_id as u64));
                    match resp {
                        KvsResponse::PutOk { key, .. } => {
                            assert_eq!(key, "k");
                            put_count += 1;
                        }
                        other => panic!("expected PutOk, got {other:?}"),
                    }
                }
            }

            if put_count == 2 {
                cmd_port.send(
                    0,
                    (
                        (Ingress::Test, 2u64),
                        KvsCommand::Get {
                            trace_id: "t3".into(),
                            key: "k".into(),
                        },
                    ),
                );
                if let Some((cid, resp)) = resp_port.next(0).await {
                    assert_eq!(cid, (Ingress::Test, 2));
                    match resp {
                        KvsResponse::GetResult { key, value, .. } => {
                            assert_eq!(key, "k");
                            if let Some(v) = value {
                                assert!(
                                    v.contains("v1") || v.contains("v2"),
                                    "expected v1 or v2 in {v:?}"
                                );
                            }
                        }
                        other => panic!("expected GetResult, got {other:?}"),
                    }
                }
            }
        });
}

// ── response_node_ids tests ─────────────────────────────────────────

#[test]
fn response_node_ids_put_ok() {
    let resp = KvsResponse::PutOk {
        trace_id: String::new(),
        key: "k".into(),
        existing_vc: None,
        node_ids: HashSet::from(["n1".into(), "n2".into()]),
    };
    assert_eq!(
        response_node_ids(&resp),
        &HashSet::from(["n1".into(), "n2".into()])
    );
}

#[test]
fn response_node_ids_get_result() {
    let resp = KvsResponse::GetResult {
        trace_id: String::new(),
        key: "k".into(),
        value: None,
        existing_vc: None,
        node_ids: HashSet::from(["n3".into()]),
    };
    assert_eq!(response_node_ids(&resp), &HashSet::from(["n3".into()]));
}

// ── kvs_storage additional tests ────────────────────────────────────

#[test]
fn sim_storage_multiple_keys() {
    let mut flow = FlowBuilder::new();
    let p = flow.process::<()>();

    let (put_port, put_stream) = p.sim_input::<(String, ClockedSet), TotalOrder, _>();
    let (_unused, empty) = p.sim_input::<(String, ClockedSet), TotalOrder, _>();

    let store = kvs_storage(
        put_stream.weaken_ordering::<NoOrder>(),
        empty.weaken_ordering::<NoOrder>(),
    );
    let tick = p.tick();
    let out = store
        .snapshot(&tick, nondet!(/** snapshot */))
        .into_stream()
        .all_ticks()
        .sim_output();

    flow.sim().exhaustive(async || {
        put_port.send(("a".to_string(), clocked("w", 1, "va")));
        put_port.send(("b".to_string(), clocked("w", 1, "vb")));
        put_port.send(("c".to_string(), clocked("w", 1, "vc")));

        // Consume snapshots until we see all 3 keys
        let mut snapshot;
        loop {
            snapshot = out.next().await.unwrap();
            if snapshot.len() >= 3 {
                break;
            }
        }
        assert!(snapshot.contains_key("a"));
        assert!(snapshot.contains_key("b"));
        assert!(snapshot.contains_key("c"));
        let vals_a = snapshot.get("a").unwrap().as_reveal_ref().1.as_reveal_ref();
        assert!(vals_a.contains("va"));
    });
}

#[test]
fn sim_storage_secondary_puts_merge() {
    let mut flow = FlowBuilder::new();
    let p = flow.process::<()>();

    let (primary_port, primary) = p.sim_input::<(String, ClockedSet), TotalOrder, _>();
    let (secondary_port, secondary) = p.sim_input::<(String, ClockedSet), TotalOrder, _>();

    let store = kvs_storage(
        primary.weaken_ordering::<NoOrder>(),
        secondary.weaken_ordering::<NoOrder>(),
    );
    let tick = p.tick();
    let out = store
        .snapshot(&tick, nondet!(/** snapshot */))
        .into_stream()
        .all_ticks()
        .sim_output();

    flow.sim().exhaustive(async || {
        // Primary writes key "k"
        primary_port.send(("k".to_string(), clocked("w1", 1, "primary_val")));
        let _ = out.next().await;

        // Secondary writes same key with concurrent VC
        secondary_port.send(("k".to_string(), clocked("w2", 1, "secondary_val")));
        let mut snapshot = out.next().await.unwrap();
        if let Some(entry) = snapshot.get("k") {
            if entry.as_reveal_ref().1.as_reveal_ref().len() < 2 {
                snapshot = out.next().await.unwrap();
            }
        }
        let values = snapshot.get("k").unwrap().as_reveal_ref().1.as_reveal_ref();
        assert!(
            values.contains("primary_val"),
            "missing primary_val: {values:?}"
        );
        assert!(
            values.contains("secondary_val"),
            "missing secondary_val: {values:?}"
        );
    });
}

#[test]
fn sim_storage_idempotent_duplicate_put() {
    let mut flow = FlowBuilder::new();
    let p = flow.process::<()>();

    let (put_port, put_stream) = p.sim_input::<(String, ClockedSet), TotalOrder, _>();
    let (_unused, empty) = p.sim_input::<(String, ClockedSet), TotalOrder, _>();

    let store = kvs_storage(
        put_stream.weaken_ordering::<NoOrder>(),
        empty.weaken_ordering::<NoOrder>(),
    );
    let tick = p.tick();
    let out = store
        .snapshot(&tick, nondet!(/** snapshot */))
        .into_stream()
        .all_ticks()
        .sim_output();

    flow.sim().exhaustive(async || {
        // Send the exact same put twice — lattice merge should be idempotent
        put_port.send(("k".to_string(), clocked("w", 1, "v")));
        let _ = out.next().await;

        put_port.send(("k".to_string(), clocked("w", 1, "v")));
        let snapshot = out.next().await.unwrap();
        let values = snapshot.get("k").unwrap().as_reveal_ref().1.as_reveal_ref();
        assert_eq!(
            values.len(),
            1,
            "duplicate put should be idempotent: {values:?}"
        );
        assert!(values.contains("v"));
    });
}

// ── merge_responses additional tests ────────────────────────────────

#[test]
fn sim_merge_rep_factor_1_emits_immediately() {
    let mut flow = FlowBuilder::new();
    let p = flow.process::<()>();

    let (in_port, in_stream) = p.sim_input::<(ClientKey, KvsResponse), TotalOrder, _>();
    let merged = merge_responses::<_, 1>(in_stream.weaken_ordering::<NoOrder>());
    let out = merged.sim_output();

    flow.sim().exhaustive(async || {
        in_port.send((
            (Ingress::Test, 1),
            KvsResponse::PutOk {
                trace_id: String::new(),
                existing_vc: None,
                key: "k".into(),
                node_ids: HashSet::from(["n1".into()]),
            },
        ));
        out.assert_yields_unordered([(
            (Ingress::Test, 1u64),
            KvsResponse::PutOk {
                trace_id: String::new(),
                existing_vc: None,
                key: "k".into(),
                node_ids: HashSet::from(["n1".into()]),
            },
        )])
        .await;
    });
}

#[test]
fn sim_merge_independent_keys_tracked_separately() {
    let mut flow = FlowBuilder::new();
    let p = flow.process::<()>();

    let (in_port, in_stream) = p.sim_input::<(ClientKey, KvsResponse), TotalOrder, _>();
    let merged = merge_responses::<_, 2>(in_stream.weaken_ordering::<NoOrder>());
    let out = merged.sim_output();

    flow.sim().exhaustive(async || {
        // Two responses for key "a", two for key "b", same client_id
        in_port.send((
            (Ingress::Test, 1),
            KvsResponse::PutOk {
                trace_id: String::new(),
                existing_vc: None,
                key: "a".into(),
                node_ids: HashSet::from(["n1".into()]),
            },
        ));
        in_port.send((
            (Ingress::Test, 1),
            KvsResponse::PutOk {
                trace_id: String::new(),
                existing_vc: None,
                key: "b".into(),
                node_ids: HashSet::from(["n1".into()]),
            },
        ));
        in_port.send((
            (Ingress::Test, 1),
            KvsResponse::PutOk {
                trace_id: String::new(),
                existing_vc: None,
                key: "a".into(),
                node_ids: HashSet::from(["n2".into()]),
            },
        ));
        in_port.send((
            (Ingress::Test, 1),
            KvsResponse::PutOk {
                trace_id: String::new(),
                existing_vc: None,
                key: "b".into(),
                node_ids: HashSet::from(["n2".into()]),
            },
        ));

        // Both keys should complete independently
        out.assert_yields_unordered([
            (
                (Ingress::Test, 1u64),
                KvsResponse::PutOk {
                    trace_id: String::new(),
                    existing_vc: None,
                    key: "a".into(),
                    node_ids: HashSet::from(["n1".into(), "n2".into()]),
                },
            ),
            (
                (Ingress::Test, 1u64),
                KvsResponse::PutOk {
                    trace_id: String::new(),
                    existing_vc: None,
                    key: "b".into(),
                    node_ids: HashSet::from(["n1".into(), "n2".into()]),
                },
            ),
        ])
        .await;
    });
}

#[test]
fn sim_merge_different_client_ids_independent() {
    let mut flow = FlowBuilder::new();
    let p = flow.process::<()>();

    let (in_port, in_stream) = p.sim_input::<(ClientKey, KvsResponse), TotalOrder, _>();
    let merged = merge_responses::<_, 2>(in_stream.weaken_ordering::<NoOrder>());
    let out = merged.sim_output();

    flow.sim().exhaustive(async || {
        // client 1 and client 2 both put to key "k"
        in_port.send((
            (Ingress::Test, 1),
            KvsResponse::PutOk {
                trace_id: String::new(),
                existing_vc: None,
                key: "k".into(),
                node_ids: HashSet::from(["n1".into()]),
            },
        ));
        in_port.send((
            (Ingress::Test, 2),
            KvsResponse::PutOk {
                trace_id: String::new(),
                existing_vc: None,
                key: "k".into(),
                node_ids: HashSet::from(["n1".into()]),
            },
        ));
        // Complete client 1
        in_port.send((
            (Ingress::Test, 1),
            KvsResponse::PutOk {
                trace_id: String::new(),
                existing_vc: None,
                key: "k".into(),
                node_ids: HashSet::from(["n2".into()]),
            },
        ));
        // Complete client 2
        in_port.send((
            (Ingress::Test, 2),
            KvsResponse::PutOk {
                trace_id: String::new(),
                existing_vc: None,
                key: "k".into(),
                node_ids: HashSet::from(["n2".into()]),
            },
        ));

        out.assert_yields_unordered([
            (
                (Ingress::Test, 1u64),
                KvsResponse::PutOk {
                    trace_id: String::new(),
                    existing_vc: None,
                    key: "k".into(),
                    node_ids: HashSet::from(["n1".into(), "n2".into()]),
                },
            ),
            (
                (Ingress::Test, 2u64),
                KvsResponse::PutOk {
                    trace_id: String::new(),
                    existing_vc: None,
                    key: "k".into(),
                    node_ids: HashSet::from(["n1".into(), "n2".into()]),
                },
            ),
        ])
        .await;
    });
}

#[test]
fn sim_merge_get_vc_merging() {
    let mut flow = FlowBuilder::new();
    let p = flow.process::<()>();

    let (in_port, in_stream) = p.sim_input::<(ClientKey, KvsResponse), TotalOrder, _>();
    let merged = merge_responses::<_, 2>(in_stream.weaken_ordering::<NoOrder>());
    let out = merged.sim_output();

    flow.sim().exhaustive(async || {
        // Node 1 has VC {r1: 3}, node 2 has VC {r1: 2, r2: 1}
        let mut vc1 = HashMap::new();
        vc1.insert("r1".to_string(), 3u64);
        let mut vc2 = HashMap::new();
        vc2.insert("r1".to_string(), 2u64);
        vc2.insert("r2".to_string(), 1u64);

        in_port.send((
            (Ingress::Test, 1),
            KvsResponse::GetResult {
                trace_id: String::new(),
                key: "k".into(),
                value: Some(HashSet::from(["v".into()])),
                existing_vc: Some(vc1),
                node_ids: HashSet::from(["n1".into()]),
            },
        ));
        in_port.send((
            (Ingress::Test, 1),
            KvsResponse::GetResult {
                trace_id: String::new(),
                key: "k".into(),
                value: Some(HashSet::from(["v".into()])),
                existing_vc: Some(vc2),
                node_ids: HashSet::from(["n2".into()]),
            },
        ));

        // Merged VC should be max per entry: {r1: 3, r2: 1}
        let mut expected_vc = HashMap::new();
        expected_vc.insert("r1".to_string(), 3u64);
        expected_vc.insert("r2".to_string(), 1u64);
        out.assert_yields_unordered([(
            (Ingress::Test, 1u64),
            KvsResponse::GetResult {
                trace_id: String::new(),
                key: "k".into(),
                value: Some(HashSet::from(["v".into()])),
                existing_vc: Some(expected_vc),
                node_ids: HashSet::from(["n1".into(), "n2".into()]),
            },
        )])
        .await;
    });
}

#[test]
fn sim_merge_get_both_none_yields_none() {
    let mut flow = FlowBuilder::new();
    let p = flow.process::<()>();

    let (in_port, in_stream) = p.sim_input::<(ClientKey, KvsResponse), TotalOrder, _>();
    let merged = merge_responses::<_, 2>(in_stream.weaken_ordering::<NoOrder>());
    let out = merged.sim_output();

    flow.sim().exhaustive(async || {
        in_port.send((
            (Ingress::Test, 1),
            KvsResponse::GetResult {
                trace_id: String::new(),
                key: "missing".into(),
                value: None,
                existing_vc: None,
                node_ids: HashSet::from(["n1".into()]),
            },
        ));
        in_port.send((
            (Ingress::Test, 1),
            KvsResponse::GetResult {
                trace_id: String::new(),
                key: "missing".into(),
                value: None,
                existing_vc: None,
                node_ids: HashSet::from(["n2".into()]),
            },
        ));

        out.assert_yields_unordered([(
            (Ingress::Test, 1u64),
            KvsResponse::GetResult {
                trace_id: String::new(),
                key: "missing".into(),
                value: None,
                existing_vc: None,
                node_ids: HashSet::from(["n1".into(), "n2".into()]),
            },
        )])
        .await;
    });
}

// ── Vector clock domination edge cases ──────────────────────────────

#[test]
fn sim_storage_three_way_concurrent_merge() {
    let mut flow = FlowBuilder::new();
    let p = flow.process::<()>();

    let (put_port, put_stream) = p.sim_input::<(String, ClockedSet), TotalOrder, _>();
    let (_unused, empty) = p.sim_input::<(String, ClockedSet), TotalOrder, _>();

    let store = kvs_storage(
        put_stream.weaken_ordering::<NoOrder>(),
        empty.weaken_ordering::<NoOrder>(),
    );
    let tick = p.tick();
    let out = store
        .snapshot(&tick, nondet!(/** snapshot */))
        .into_stream()
        .all_ticks()
        .sim_output();

    flow.sim().exhaustive(async || {
        // Three concurrent writers, all with independent VCs
        put_port.send(("k".to_string(), clocked("w1", 1, "v1")));
        put_port.send(("k".to_string(), clocked("w2", 1, "v2")));
        put_port.send(("k".to_string(), clocked("w3", 1, "v3")));

        // Wait until all 3 values are merged
        let mut snapshot;
        loop {
            snapshot = out.next().await.unwrap();
            if let Some(entry) = snapshot.get("k") {
                if entry.as_reveal_ref().1.as_reveal_ref().len() >= 3 {
                    break;
                }
            }
        }
        let values = snapshot.get("k").unwrap().as_reveal_ref().1.as_reveal_ref();
        assert!(values.contains("v1"), "missing v1: {values:?}");
        assert!(values.contains("v2"), "missing v2: {values:?}");
        assert!(values.contains("v3"), "missing v3: {values:?}");
    });
}

#[test]
fn sim_storage_dominating_vc_clears_all_concurrent() {
    let mut flow = FlowBuilder::new();
    let p = flow.process::<()>();

    let (put_port, put_stream) = p.sim_input::<(String, ClockedSet), TotalOrder, _>();
    let (_unused, empty) = p.sim_input::<(String, ClockedSet), TotalOrder, _>();

    let store = kvs_storage(
        put_stream.weaken_ordering::<NoOrder>(),
        empty.weaken_ordering::<NoOrder>(),
    );
    let tick = p.tick();
    let out = store
        .snapshot(&tick, nondet!(/** snapshot */))
        .into_stream()
        .all_ticks()
        .sim_output();

    flow.sim().exhaustive(async || {
        // Two concurrent writes
        put_port.send(("k".to_string(), clocked("w1", 1, "v1")));
        put_port.send(("k".to_string(), clocked("w2", 1, "v2")));

        // Wait for both to merge
        let mut snapshot;
        loop {
            snapshot = out.next().await.unwrap();
            if let Some(entry) = snapshot.get("k") {
                if entry.as_reveal_ref().1.as_reveal_ref().len() >= 2 {
                    break;
                }
            }
        }

        // Now send a write that dominates both: VC = {w1: 2, w2: 2}
        let mut vc = HashMap::new();
        vc.insert("w1".to_string(), Max::new(2));
        vc.insert("w2".to_string(), Max::new(2));
        let dominating = DomPair::new(
            MapUnion::new(vc),
            SetUnionHashSet::new(HashSet::from(["final".to_string()])),
        );
        put_port.send(("k".to_string(), dominating));
        let snapshot = out.next().await.unwrap();
        let values = snapshot.get("k").unwrap().as_reveal_ref().1.as_reveal_ref();
        assert_eq!(
            values.len(),
            1,
            "dominating VC should clear concurrent values: {values:?}"
        );
        assert!(values.contains("final"));
    });
}

// ── merge_responses: accumulation across batches ────────────────────

#[test]
fn sim_merge_get_unions_multiple_values_from_different_nodes() {
    let mut flow = FlowBuilder::new();
    let p = flow.process::<()>();

    let (in_port, in_stream) = p.sim_input::<(ClientKey, KvsResponse), TotalOrder, _>();
    let merged = merge_responses::<_, 2>(in_stream.weaken_ordering::<NoOrder>());
    let out = merged.sim_output();

    flow.sim().exhaustive(async || {
        // Node 1 sees {"a", "b"}, node 2 sees {"b", "c"} — union should be {"a","b","c"}
        in_port.send((
            (Ingress::Test, 1),
            KvsResponse::GetResult {
                trace_id: String::new(),
                key: "k".into(),
                value: Some(HashSet::from(["a".into(), "b".into()])),
                existing_vc: None,
                node_ids: HashSet::from(["n1".into()]),
            },
        ));
        in_port.send((
            (Ingress::Test, 1),
            KvsResponse::GetResult {
                trace_id: String::new(),
                key: "k".into(),
                value: Some(HashSet::from(["b".into(), "c".into()])),
                existing_vc: None,
                node_ids: HashSet::from(["n2".into()]),
            },
        ));

        out.assert_yields_unordered([(
            (Ingress::Test, 1u64),
            KvsResponse::GetResult {
                trace_id: String::new(),
                key: "k".into(),
                value: Some(HashSet::from(["a".into(), "b".into(), "c".into()])),
                existing_vc: None,
                node_ids: HashSet::from(["n1".into(), "n2".into()]),
            },
        )])
        .await;
    });
}

// ── Pure function unit tests ────────────────────────────────────────

#[test]
fn build_node_response_clocked_put_returns_put_ok() {
    let cmd = NodeCommand::ClockedPut {
        trace_id: String::new(),
        key: "k".into(),
        clocked_value: protocol::clocked_set_to_cv(clocked("w", 1, "v")),
    };
    let store = HashMap::new();
    let resp = build_node_response(cmd, &store, "node_0");
    assert_eq!(
        resp,
        KvsResponse::PutOk {
            trace_id: String::new(),
            key: "k".into(),
            existing_vc: None,
            node_ids: HashSet::from(["node_0".into()]),
        }
    );
}

#[test]
fn build_node_response_get_existing_key() {
    let mut store = HashMap::new();
    store.insert("k".to_string(), clocked("w", 5, "val"));
    let cmd = NodeCommand::Get {
        trace_id: String::new(),
        key: "k".into(),
    };
    let resp = build_node_response(cmd, &store, "node_0");
    match resp {
        KvsResponse::GetResult {
            key,
            value: Some(v),
            existing_vc: Some(vc),
            node_ids,
            ..
        } => {
            assert_eq!(key, "k");
            assert!(v.contains("val"));
            assert_eq!(vc.get("w"), Some(&5u64));
            assert_eq!(node_ids, HashSet::from(["node_0".into()]));
        }
        other => panic!("expected GetResult with value, got {other:?}"),
    }
}

#[test]
fn build_node_response_get_missing_key() {
    let store = HashMap::new();
    let cmd = NodeCommand::Get {
        trace_id: String::new(),
        key: "missing".into(),
    };
    let resp = build_node_response(cmd, &store, "node_0");
    assert_eq!(
        resp,
        KvsResponse::GetResult {
            trace_id: String::new(),
            key: "missing".into(),
            value: None,
            existing_vc: None,
            node_ids: HashSet::from(["node_0".into()]),
        }
    );
}

#[test]
fn rendezvous_targets_returns_n_targets() {
    let members: HashSet<u32> = (0..10).collect();
    let targets = rendezvous_targets("some_key", &members, 3);
    assert_eq!(targets.len(), 3);
    // All targets should be unique
    let unique: HashSet<_> = targets.iter().collect();
    assert_eq!(unique.len(), 3);
}

#[test]
fn rendezvous_targets_deterministic() {
    let members: HashSet<u32> = (0..10).collect();
    let t1 = rendezvous_targets("key", &members, 3);
    let t2 = rendezvous_targets("key", &members, 3);
    assert_eq!(t1, t2);
}

#[test]
fn rendezvous_targets_different_keys_differ() {
    let members: HashSet<u32> = (0..10).collect();
    let t1 = rendezvous_targets("key_a", &members, 3);
    let t2 = rendezvous_targets("key_b", &members, 3);
    // Very unlikely to be the same for different keys with 10 members
    assert_ne!(t1, t2);
}

#[test]
fn rendezvous_targets_capped_at_member_count() {
    let members: HashSet<u32> = HashSet::from([1, 2]);
    let targets = rendezvous_targets("key", &members, 5);
    assert_eq!(targets.len(), 2);
}

#[test]
fn rendezvous_targets_empty_members() {
    let members: HashSet<u32> = HashSet::new();
    let targets = rendezvous_targets("key", &members, 3);
    assert!(targets.is_empty());
}

#[test]
fn rendezvous_targets_distributes_across_members() {
    let members: HashSet<u32> = (0..5).collect();
    let mut hit_counts: HashMap<u32, usize> = HashMap::new();
    for i in 0..100 {
        let targets = rendezvous_targets(&format!("key_{i}"), &members, 1);
        for t in targets {
            *hit_counts.entry(t).or_default() += 1;
        }
    }
    // All 5 members should be hit at least once with 100 keys
    assert_eq!(hit_counts.len(), 5, "not all members used: {hit_counts:?}");
}

#[test]
fn classify_merged_response_put_ok_is_client() {
    let resp = KvsResponse::PutOk {
        trace_id: String::new(),
        existing_vc: None,
        key: "k".into(),
        node_ids: HashSet::from(["n1".into()]),
    };
    let result = classify_merged_response((Ingress::Test, 42), resp.clone());
    assert_eq!(result, Ok(((Ingress::Test, 42), resp)));
}

#[test]
fn classify_merged_response_put_ok_with_offset_is_still_client() {
    // PutOk always goes to client regardless of client_id
    let resp = KvsResponse::PutOk {
        trace_id: String::new(),
        existing_vc: None,
        key: "k".into(),
        node_ids: HashSet::from(["n1".into()]),
    };
    let result = classify_merged_response((Ingress::Test, 1_000_000_042), resp.clone());
    assert_eq!(result, Ok(((Ingress::Test, 1_000_000_042), resp)));
}

#[test]
fn classify_merged_response_get_below_offset_is_client() {
    let resp = KvsResponse::GetResult {
        trace_id: String::new(),
        key: "k".into(),
        value: Some(HashSet::from(["v".into()])),
        existing_vc: None,
        node_ids: HashSet::from(["n1".into()]),
    };
    let result = classify_merged_response((Ingress::Test, 42), resp.clone());
    assert_eq!(result, Ok(((Ingress::Test, 42), resp)));
}

#[test]
fn classify_merged_response_get_at_offset_is_read_phase() {
    let mut vc = HashMap::new();
    vc.insert("r1".to_string(), 3u64);
    let resp = KvsResponse::GetResult {
        trace_id: String::new(),
        key: "k".into(),
        value: Some(HashSet::from(["v".into()])),
        existing_vc: Some(vc.clone()),
        node_ids: HashSet::from(["n1".into()]),
    };
    let result = classify_merged_response((Ingress::Test, 1_000_000_042), resp);
    assert_eq!(
        result,
        Err(((Ingress::Test, 42), String::new(), "k".into(), Some(vc)))
    );
}

#[test]
fn split_client_command_get() {
    let (get, read, pending) = split_client_command(
        (Ingress::Test, 5),
        KvsCommand::Get {
            trace_id: String::new(),
            key: "k".into(),
        },
    );
    assert_eq!(
        get,
        Some((
            (Ingress::Test, 5),
            KvsCommand::Get {
                trace_id: String::new(),
                key: "k".into()
            }
        ))
    );
    assert!(read.is_none());
    assert!(pending.is_none());
}

#[test]
fn split_client_command_put() {
    let (get, read, pending) = split_client_command(
        (Ingress::Test, 5),
        KvsCommand::Put {
            trace_id: String::new(),
            key: "k".into(),
            value: "v".into(),
        },
    );
    assert!(get.is_none());
    assert_eq!(
        read,
        Some((
            (Ingress::Test, 5 + READ_PHASE_OFFSET),
            KvsCommand::Get {
                trace_id: String::new(),
                key: "k".into()
            }
        ))
    );
    assert_eq!(
        pending,
        Some(((Ingress::Test, 5), String::new(), "k".into(), "v".into()))
    );
}

#[test]
fn build_dominating_clocked_put_no_existing_vc() {
    let cs = build_dominating_clocked_put("val".into(), None, "router_0", 1);
    let vc = cs.as_reveal_ref().0.as_reveal_ref();
    assert_eq!(vc.len(), 1);
    assert_eq!(vc.get("router_0").unwrap().into_reveal(), 1);
    let vals = cs.as_reveal_ref().1.as_reveal_ref();
    assert!(vals.contains("val"));
}

#[test]
fn build_dominating_clocked_put_with_existing_vc() {
    let mut existing = HashMap::new();
    existing.insert("old_router".to_string(), 3u64);
    existing.insert("router_0".to_string(), 1u64);

    let cs = build_dominating_clocked_put("val".into(), Some(existing), "router_0", 5);
    let vc = cs.as_reveal_ref().0.as_reveal_ref();
    // Should have old_router: 3 (preserved) and router_0: 5 (bumped)
    assert_eq!(vc.get("old_router").unwrap().into_reveal(), 3);
    assert_eq!(vc.get("router_0").unwrap().into_reveal(), 5);
}

#[test]
fn build_dominating_clocked_put_does_not_lower_existing_seq() {
    let mut existing = HashMap::new();
    existing.insert("router_0".to_string(), 10u64);

    // seq=5 is lower than existing 10 — should keep 10
    let cs = build_dominating_clocked_put("val".into(), Some(existing), "router_0", 5);
    let vc = cs.as_reveal_ref().0.as_reveal_ref();
    assert_eq!(vc.get("router_0").unwrap().into_reveal(), 10);
}
