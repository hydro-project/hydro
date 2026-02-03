//! This implements the Maelstrom broadcast workload.

use hydro_lang::live_collections::stream::NoOrder;
use hydro_lang::prelude::*;

/// Creates an broadcast server flow for Maelstrom.
///
/// Takes a keyed input stream of (client_id, message_body) and returns
/// a keyed output stream of (client_id, response_body).
pub fn broadcast_server<'a, C: 'a>(
    input: KeyedStream<String, serde_json::Value, Cluster<'a, C>>,
) -> KeyedStream<String, serde_json::Value, Cluster<'a, C>, Unbounded, NoOrder> {
    let broadcast_requests = input.clone().filter_map(q!(|body| {
        if body.get("type").and_then(|v| v.as_str()) == Some("broadcast") {
            Some((
                body.get("msg_id").unwrap().as_u64().unwrap(),
                body.get("message").unwrap().as_u64().unwrap(),
            ))
        } else {
            None
        }
    }));

    let broadcast_response = broadcast_requests.clone().map(q!(|(msg_id, _)| {
        serde_json::json!({
            "type": "broadcast_ok",
            "in_reply_to": msg_id
        })
    }));

    let written_data = broadcast_requests.values().map(q!(|t| t.1));
    let broadcasted = written_data
        .broadcast(input.location(), TCP.bincode(), nondet!(/** TODO */))
        .values();

    let read_requests = input.clone().filter_map(q!(|body| {
        if body.get("type").and_then(|v| v.as_str()) == Some("read") {
            Some(body.get("msg_id").unwrap().as_u64().unwrap())
        } else {
            None
        }
    }));

    let read_response = sliced! {
        let req = use(read_requests, nondet!(/** TODO */));
        let data = use(broadcasted.fold(q!(|| vec![]), q!(|v, d| v.push(d), commutative = ManualProof(/* TODO */))), nondet!(/** TODO */));

        req.cross_singleton(data).map(q!(|(msg_id, data)| {
            serde_json::json!({
                "type": "read_ok",
                "messages": data,
                "in_reply_to": msg_id
            })
        }))
    };

    let topology_requests = input.filter_map(q!(|body| {
        if body.get("type").and_then(|v| v.as_str()) == Some("topology") {
            Some(body.get("msg_id").unwrap().as_u64().unwrap())
        } else {
            None
        }
    }));

    let topology_response = topology_requests.map(q!(|msg_id| {
        eprintln!("sending topology response");
        serde_json::json!({
            "type": "topology_ok",
            "in_reply_to": msg_id
        })
    }));

    broadcast_response
        .interleave(read_response)
        .interleave(topology_response)
}
