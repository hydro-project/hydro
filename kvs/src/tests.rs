//! Simulation tests for the key-value store. These are compiled as part
//! of the `kvs` crate under `#[cfg(test)]` (so stageleft's `q!` codegen
//! works), but kept out of `lib.rs` to separate logic from tests.

use hydro_lang::live_collections::stream::{ExactlyOnce, TotalOrder};
use hydro_lang::prelude::*;

use crate::*;

/// A single `Put` should produce exactly one `PutAck` once a quorum of
/// storage replicas has acknowledged the write.
#[test]
fn test_put_acks() {
    let mut flow = FlowBuilder::new();
    let routers = flow.cluster::<RouterNode>();
    let storage = flow.cluster::<StorageNode>();

    let (client_send, requests) = routers.sim_input::<ClientRequest, TotalOrder, ExactlyOnce>();

    let (responses, _membership) = kvs(&routers, &storage, requests.weaken_ordering());
    // Map to a sortable form so we can `collect_sorted` from the unordered
    // response stream.
    let out = responses
        .map(q!(|r| match r {
            ClientResponse::PutAck { req_id } => (req_id, true),
            ClientResponse::GetResult { req_id, .. } => (req_id, false),
        }))
        .assume_ordering::<TotalOrder>(nondet!(
            /// Test issues a single request, so ordering is trivial.
        ))
        .sim_cluster_output();
    flow.sim()
        .with_cluster_size(&routers, 3)
        .with_cluster_size(&storage, 9)
        .fuzz(async || {
            // Routing buffers this request until the router has discovered the
            // full storage membership.
            client_send.send(
                0,
                ClientRequest {
                    req_id: 1,
                    op: Op::Put {
                        key: "k".to_owned(),
                        value: "v".to_owned(),
                    },
                },
            );

            match out.next(0).await {
                Some((1, true)) => {}
                Some(other) => panic!("unexpected response: {other:?}"),
                None => panic!("no PutAck received before the simulation ended"),
            }
        });
}

/// After a `Put` is acknowledged, a subsequent `Get` for the same key from
/// any router must observe the written value.
#[test]
fn test_put_then_get() {
    let mut flow = FlowBuilder::new();
    let routers = flow.cluster::<RouterNode>();
    let storage = flow.cluster::<StorageNode>();

    let (client_send, requests) = routers.sim_input::<ClientRequest, TotalOrder, ExactlyOnce>();

    let (responses, _membership) = kvs(&routers, &storage, requests.weaken_ordering());
    // We drive requests strictly one-at-a-time (each after the previous
    // response), so ordering the output per router is safe for this test.
    let out = responses
        .assume_ordering::<TotalOrder>(nondet!(
            /// Test issues requests sequentially, so responses arrive in order.
        ))
        .sim_cluster_output();
    flow.sim()
        .with_cluster_size(&routers, 3)
        .with_cluster_size(&storage, 9)
        // Exhaustive search over a two-step scenario across 9 storage nodes
        // is too large; fuzz explores many random interleavings instead.
        .fuzz(async || {
            // Put k=v via router 0 and wait for the ack.
            client_send.send(
                0,
                ClientRequest {
                    req_id: 1,
                    op: Op::Put {
                        key: "k".to_owned(),
                        value: "v".to_owned(),
                    },
                },
            );
            match out.next(0).await {
                Some(ClientResponse::PutAck { req_id: 1 }) => {}
                Some(other) => panic!("unexpected response before ack: {other:?}"),
                None => panic!("no PutAck received before the simulation ended"),
            }

            // Now Get k via router 1; it must see the value.
            client_send.send(
                1,
                ClientRequest {
                    req_id: 2,
                    op: Op::Get {
                        key: "k".to_owned(),
                    },
                },
            );
            match out.next(1).await {
                Some(ClientResponse::GetResult { req_id: 2, values }) => {
                    assert_eq!(values, HashSet::from(["v".to_owned()]));
                }
                Some(other) => panic!("unexpected get response: {other:?}"),
                None => panic!("no GetResult received before the simulation ended"),
            }
        });
}

/// Two `Put`s of different values under the same key should union into a
/// set, since the data model is a grow-only set per key.
#[test]
fn test_get_unions_values() {
    let mut flow = FlowBuilder::new();
    let routers = flow.cluster::<RouterNode>();
    let storage = flow.cluster::<StorageNode>();

    let (client_send, requests) = routers.sim_input::<ClientRequest, TotalOrder, ExactlyOnce>();

    let (responses, _membership) = kvs(&routers, &storage, requests.weaken_ordering());
    let out = responses
        .assume_ordering::<TotalOrder>(nondet!(
            /// Test issues requests sequentially, so responses arrive in order.
        ))
        .sim_cluster_output();
    flow.sim()
        .with_cluster_size(&routers, 3)
        .with_cluster_size(&storage, 9)
        .fuzz(async || {
            // Put two distinct values under the same key, waiting for each ack.
            for (req_id, value) in [(1u64, "a"), (2, "b")] {
                client_send.send(
                    0,
                    ClientRequest {
                        req_id,
                        op: Op::Put {
                            key: "k".to_owned(),
                            value: value.to_owned(),
                        },
                    },
                );
                match out.next(0).await {
                    Some(ClientResponse::PutAck { req_id: acked }) if acked == req_id => {}
                    Some(other) => panic!("unexpected response before ack: {other:?}"),
                    None => panic!("no PutAck received before the simulation ended"),
                }
            }

            // Get should return the union of both values.
            client_send.send(
                0,
                ClientRequest {
                    req_id: 3,
                    op: Op::Get {
                        key: "k".to_owned(),
                    },
                },
            );
            match out.next(0).await {
                Some(ClientResponse::GetResult { req_id: 3, values }) => {
                    assert_eq!(values, HashSet::from(["a".to_owned(), "b".to_owned()]));
                }
                Some(other) => panic!("unexpected get response: {other:?}"),
                None => panic!("no GetResult received before the simulation ended"),
            }
        });
}

/// A `Get` for a key that was never written should return an empty set
/// (rather than hanging), so clients can distinguish "no values" cleanly.
#[test]
fn test_get_missing_key() {
    let mut flow = FlowBuilder::new();
    let routers = flow.cluster::<RouterNode>();
    let storage = flow.cluster::<StorageNode>();

    let (client_send, requests) = routers.sim_input::<ClientRequest, TotalOrder, ExactlyOnce>();

    let (responses, _membership) = kvs(&routers, &storage, requests.weaken_ordering());
    let out = responses
        .assume_ordering::<TotalOrder>(nondet!(
            /// Test issues a single request, so ordering is trivial.
        ))
        .sim_cluster_output();
    flow.sim()
        .with_cluster_size(&routers, 3)
        .with_cluster_size(&storage, 9)
        .fuzz(async || {
            client_send.send(
                0,
                ClientRequest {
                    req_id: 1,
                    op: Op::Get {
                        key: "absent".to_owned(),
                    },
                },
            );
            match out.next(0).await {
                Some(ClientResponse::GetResult { req_id: 1, values }) => {
                    assert_eq!(values, HashSet::new());
                }
                Some(other) => panic!("unexpected response: {other:?}"),
                None => panic!("no GetResult received before the simulation ended"),
            }
        });
}
