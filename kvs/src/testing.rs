//! Client helpers for tests — requests go through the gRPC and WebSocket
//! ingresses. The gRPC path via `send_recv_grpc` is the primary test surface;
//! `send_recv_ws` exercises the WebSocket sidecar.

use std::collections::{HashMap, HashSet};

use crate::{KvsCommand, KvsResponse, REPLICATION_FACTOR};

/// Generate a fresh 16-hex-character trace id for a test-driven request.
/// Real clients would propagate an upstream trace id; tests just need
/// something unique enough to correlate server logs.
pub fn new_trace_id() -> String {
    format!("{:08x}{:08x}", rand::random::<u32>(), rand::random::<u32>())
}

pub fn response_node_ids(resp: &KvsResponse) -> &HashSet<String> {
    match resp {
        KvsResponse::PutOk { node_ids, .. } | KvsResponse::GetResult { node_ids, .. } => node_ids,
    }
}

/// Send a command and receive the response via the gRPC client.
pub async fn send_recv(addr: &str, cmd: &KvsCommand) -> KvsResponse {
    send_recv_grpc(addr, cmd).await
}

/// gRPC client — dispatches the command to the tonic-generated `Kvs`
/// client and maps the protobuf response back to the internal
/// `KvsResponse` shape.
pub async fn send_recv_grpc(addr: &str, cmd: &KvsCommand) -> KvsResponse {
    use crate::grpc_port::pb::kvs_client::KvsClient;
    use crate::grpc_port::pb::{GetRequest, PutRequest};

    let url = if addr.starts_with("http://") || addr.starts_with("https://") {
        addr.to_string()
    } else {
        format!("http://{addr}")
    };
    let mut client = KvsClient::connect(url).await.expect("gRPC connect failed");

    match cmd {
        KvsCommand::Put {
            trace_id,
            key,
            value,
        } => {
            let resp = client
                .put(PutRequest {
                    trace_id: trace_id.clone(),
                    key: key.clone(),
                    value: value.clone(),
                })
                .await
                .expect("gRPC Put failed")
                .into_inner();
            KvsResponse::PutOk {
                trace_id: resp.trace_id,
                key: resp.key,
                existing_vc: resp.existing_vc.map(|vc| vc.entries.into_iter().collect()),
                node_ids: resp.node_ids.into_iter().collect(),
            }
        }
        KvsCommand::Get { trace_id, key } => {
            let resp = client
                .get(GetRequest {
                    trace_id: trace_id.clone(),
                    key: key.clone(),
                })
                .await
                .expect("gRPC Get failed")
                .into_inner();
            KvsResponse::GetResult {
                trace_id: resp.trace_id,
                key: resp.key,
                value: resp
                    .value
                    .map(|vs| vs.values.into_iter().collect::<HashSet<_>>()),
                existing_vc: resp.existing_vc.map(|vc| vc.entries.into_iter().collect()),
                node_ids: resp.node_ids.into_iter().collect(),
            }
        }
    }
}

/// Send a KvsCommand over a WebSocket connection and wait for the
/// response that carries the same `trace_id`. Unlike [`send_recv_grpc`],
/// the WebSocket sidecar is a thin streaming pass-through — it does no
/// request-response correlation itself and broadcasts every dataflow
/// response to every connected client. Correlation is therefore done
/// here, on the client, by matching `trace_id`.
pub async fn send_recv_ws(addr: &str, cmd: &KvsCommand) -> KvsResponse {
    use futures::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::Message;

    let expected_trace_id = match cmd {
        KvsCommand::Put { trace_id, .. } | KvsCommand::Get { trace_id, .. } => trace_id.clone(),
    };

    let url = if addr.starts_with("ws://") || addr.starts_with("wss://") {
        addr.to_string()
    } else {
        format!("ws://{addr}")
    };
    let (mut ws, _) = tokio_tungstenite::connect_async(url)
        .await
        .expect("WS connect failed");

    let body = serde_json::to_string(cmd).expect("serialize KvsCommand");
    ws.send(Message::Text(body)).await.expect("WS send failed");

    loop {
        let msg = ws
            .next()
            .await
            .expect("WS closed before response")
            .expect("WS read failed");
        let bytes = match msg {
            Message::Text(t) => t.into_bytes(),
            Message::Binary(b) => b,
            Message::Ping(_) | Message::Pong(_) | Message::Frame(_) => continue,
            Message::Close(_) => panic!("WS closed before response"),
        };
        let resp: KvsResponse = serde_json::from_slice(&bytes).expect("deserialize KvsResponse");
        let resp_trace_id = match &resp {
            KvsResponse::PutOk { trace_id, .. } | KvsResponse::GetResult { trace_id, .. } => {
                trace_id.as_str()
            }
        };
        if resp_trace_id == expected_trace_id {
            let _ = ws.close(None).await;
            return resp;
        }
        // Not our response — broadcast-based sidecar may deliver replies
        // destined for other clients too. Keep reading.
    }
}

pub async fn send_and_check_put(addr: &str, key: &str, value: &str) {
    let resp = send_recv(
        addr,
        &KvsCommand::Put {
            trace_id: new_trace_id(),
            key: key.into(),
            value: value.into(),
        },
    )
    .await;
    assert!(matches!(&resp, KvsResponse::PutOk { key: k, .. } if k == key));
}

pub async fn send_and_check_get(addr: &str, key: &str) {
    let resp = send_recv(
        addr,
        &KvsCommand::Get {
            trace_id: new_trace_id(),
            key: key.into(),
        },
    )
    .await;
    assert!(matches!(&resp, KvsResponse::GetResult { key: k, value: Some(_), .. } if k == key));
}

pub async fn run_kvs_test(addr: &str, cluster_size: usize) {
    let resp = send_recv(
        addr,
        &KvsCommand::Put {
            trace_id: new_trace_id(),
            key: "hello".into(),
            value: "world".into(),
        },
    )
    .await;
    println!("response: {resp:?}");
    assert!(matches!(&resp, KvsResponse::PutOk { key, .. } if key == "hello"));
    assert_eq!(response_node_ids(&resp).len(), REPLICATION_FACTOR);

    let resp = send_recv(
        addr,
        &KvsCommand::Put {
            trace_id: new_trace_id(),
            key: "foo".into(),
            value: "bar".into(),
        },
    )
    .await;
    println!("response: {resp:?}");
    assert!(matches!(&resp, KvsResponse::PutOk { key, .. } if key == "foo"));

    let resp = send_recv(
        addr,
        &KvsCommand::Get {
            trace_id: new_trace_id(),
            key: "hello".into(),
        },
    )
    .await;
    println!("response: {resp:?}");
    match &resp {
        KvsResponse::GetResult {
            key,
            value: Some(v),
            ..
        } => {
            assert_eq!(key, "hello");
            assert_eq!(*v, HashSet::from(["world".to_string()]));
        }
        other => panic!("expected GetResult with Some({{\"world\"}}), got {other:?}"),
    }
    assert_eq!(response_node_ids(&resp).len(), REPLICATION_FACTOR);

    let resp = send_recv(
        addr,
        &KvsCommand::Get {
            trace_id: new_trace_id(),
            key: "missing".into(),
        },
    )
    .await;
    println!("response: {resp:?}");
    assert!(matches!(&resp, KvsResponse::GetResult { key, value: None, .. } if key == "missing"));

    let resp = send_recv(
        addr,
        &KvsCommand::Put {
            trace_id: new_trace_id(),
            key: "hello".into(),
            value: "updated".into(),
        },
    )
    .await;
    assert!(matches!(&resp, KvsResponse::PutOk { key, .. } if key == "hello"));

    let resp = send_recv(
        addr,
        &KvsCommand::Get {
            trace_id: new_trace_id(),
            key: "hello".into(),
        },
    )
    .await;
    println!("response: {resp:?}");
    match &resp {
        KvsResponse::GetResult {
            key,
            value: Some(v),
            ..
        } => {
            assert_eq!(key, "hello");
            assert_eq!(*v, HashSet::from(["updated".to_string()]));
        }
        other => panic!("expected GetResult with Some({{\"updated\"}}), got {other:?}"),
    }

    let resp = send_recv(
        addr,
        &KvsCommand::Put {
            trace_id: new_trace_id(),
            key: "hello".into(),
            value: "final".into(),
        },
    )
    .await;
    assert!(matches!(&resp, KvsResponse::PutOk { key, .. } if key == "hello"));

    let resp = send_recv(
        addr,
        &KvsCommand::Get {
            trace_id: new_trace_id(),
            key: "hello".into(),
        },
    )
    .await;
    println!("response: {resp:?}");
    match &resp {
        KvsResponse::GetResult {
            key,
            value: Some(v),
            ..
        } => {
            assert_eq!(key, "hello");
            assert_eq!(*v, HashSet::from(["final".to_string()]));
        }
        other => panic!("expected GetResult with Some({{\"final\"}}), got {other:?}"),
    }

    // Distribution test
    let num_keys = 100;
    let mut node_hit_counts: HashMap<String, usize> = HashMap::new();
    for i in 0..num_keys {
        let key = format!("dist_key_{i}");
        let resp = send_recv(
            addr,
            &KvsCommand::Put {
                trace_id: new_trace_id(),
                key: key.clone(),
                value: format!("val_{i}"),
            },
        )
        .await;
        assert!(matches!(&resp, KvsResponse::PutOk { .. }));
        let ids = response_node_ids(&resp);
        assert_eq!(
            ids.len(),
            REPLICATION_FACTOR,
            "key {key}: expected {REPLICATION_FACTOR} node_ids, got {ids:?}"
        );
        for nid in ids {
            *node_hit_counts.entry(nid.clone()).or_insert(0) += 1;
        }
    }

    println!("node hit counts: {node_hit_counts:?}");
    assert_eq!(
        node_hit_counts.len(),
        cluster_size,
        "expected all {cluster_size} nodes, only {} used",
        node_hit_counts.len()
    );

    #[allow(clippy::disallowed_methods)] // summing values; order irrelevant
    let total_hits: usize = node_hit_counts.values().sum();
    let fair_share = total_hits as f64 / cluster_size as f64;
    let min_acceptable = (fair_share * 0.5) as usize;
    for (node, count) in &node_hit_counts {
        assert!(
            *count >= min_acceptable,
            "node {node} got {count} hits, expected >= {min_acceptable}"
        );
    }
    println!("kvs completed successfully");
}
