//! Protobuf codec for inter-node traffic (router<->node, node<->node rebalance).

use std::collections::{HashMap, HashSet};

use prost::Message;

use crate::protocol::{self, ClockedValue};

pub mod pb {
    include!(concat!(env!("OUT_DIR"), "/kvs_internal.rs"));
}

// ── ClientKey (Ingress tag + request id) ────────────────────────────
//
// The app's envelope key that rides alongside every command/response.
// Encoded as a protobuf message for forward/backward compatibility.

fn ingress_to_pb(ingress: protocol::Ingress) -> i32 {
    match ingress {
        protocol::Ingress::Grpc => pb::IngressTag::Grpc as i32,
        protocol::Ingress::Ws => pb::IngressTag::Ws as i32,
        protocol::Ingress::Test => pb::IngressTag::Test as i32,
    }
}

fn ingress_from_pb(tag: i32) -> protocol::Ingress {
    match pb::IngressTag::try_from(tag).unwrap_or(pb::IngressTag::Unknown) {
        pb::IngressTag::Grpc => protocol::Ingress::Grpc,
        pb::IngressTag::Ws => protocol::Ingress::Ws,
        pb::IngressTag::Test => protocol::Ingress::Test,
        // Unknown — arrived from a newer sender we don't recognize.
        // Use Test as a sentinel; it'll route to any Test sidecar (which
        // there shouldn't be in production, so the response is dropped,
        // which is correct failure behaviour).
        pb::IngressTag::Unknown => protocol::Ingress::Test,
    }
}

fn client_key_to_pb(ck: crate::ClientKey) -> pb::ClientKey {
    pb::ClientKey {
        ingress: ingress_to_pb(ck.0),
        request_id: ck.1,
    }
}

fn client_key_from_pb(pb: pb::ClientKey) -> crate::ClientKey {
    (ingress_from_pb(pb.ingress), pb.request_id)
}

// ── RouterToNodeEnvelope (router → node wire) ───────────────────────

pub fn encode_router_to_node(client_id: crate::ClientKey, cmd: &protocol::NodeCommand) -> Vec<u8> {
    let inner = build_node_command_msg(cmd);
    let env = pb::RouterToNodeEnvelope {
        client_id: Some(client_key_to_pb(client_id)),
        inner: Some(inner),
    };
    env.encode_to_vec()
}

pub fn decode_router_to_node(bytes: &[u8]) -> (crate::ClientKey, protocol::NodeCommand) {
    let env = pb::RouterToNodeEnvelope::decode(bytes).expect("decode RouterToNodeEnvelope");
    let client_id = client_key_from_pb(env.client_id.expect("RouterToNodeEnvelope.client_id"));
    let cmd = node_command_from_msg(env.inner.expect("RouterToNodeEnvelope.inner"));
    (client_id, cmd)
}

// ── NodeToRouterEnvelope (node → router wire) ───────────────────────

pub fn encode_node_to_router(client_id: crate::ClientKey, resp: &protocol::KvsResponse) -> Vec<u8> {
    let inner = build_node_response_msg(resp);
    let env = pb::NodeToRouterEnvelope {
        client_id: Some(client_key_to_pb(client_id)),
        inner: Some(inner),
    };
    env.encode_to_vec()
}

pub fn decode_node_to_router(bytes: &[u8]) -> (crate::ClientKey, protocol::KvsResponse) {
    let env = pb::NodeToRouterEnvelope::decode(bytes).expect("decode NodeToRouterEnvelope");
    let client_id = client_key_from_pb(env.client_id.expect("NodeToRouterEnvelope.client_id"));
    let resp = node_response_from_msg(env.inner.expect("NodeToRouterEnvelope.inner"));
    (client_id, resp)
}

// ── NodeCommand (router → node) ─────────────────────────────────────

#[allow(clippy::disallowed_methods)] // iteration order irrelevant for protobuf serialization
fn build_node_command_msg(cmd: &protocol::NodeCommand) -> pb::NodeCommand {
    match cmd {
        protocol::NodeCommand::ClockedPut {
            trace_id,
            key,
            clocked_value,
        } => pb::NodeCommand {
            trace_id: trace_id.clone(),
            key: key.clone(),
            kind: Some(pb::node_command::Kind::ClockedPut(pb::ClockedPutPayload {
                vector_clock: clocked_value
                    .vector_clock
                    .iter()
                    .map(|(k, v)| (k.clone(), *v as u64))
                    .collect(),
                values: clocked_value.values.clone(),
            })),
        },
        protocol::NodeCommand::Get { trace_id, key } => pb::NodeCommand {
            trace_id: trace_id.clone(),
            key: key.clone(),
            kind: Some(pb::node_command::Kind::Get(pb::GetPayload {})),
        },
    }
}

fn node_command_from_msg(msg: pb::NodeCommand) -> protocol::NodeCommand {
    match msg.kind.expect("NodeCommand.kind") {
        pb::node_command::Kind::ClockedPut(p) => {
            let cv = ClockedValue {
                vector_clock: p
                    .vector_clock
                    .into_iter()
                    .map(|(k, v)| (k, v as i64))
                    .collect(),
                values: p.values,
            };
            protocol::NodeCommand::ClockedPut {
                trace_id: msg.trace_id,
                key: msg.key,
                clocked_value: cv,
            }
        }
        pb::node_command::Kind::Get(_) => protocol::NodeCommand::Get {
            trace_id: msg.trace_id,
            key: msg.key,
        },
    }
}

// ── KvsResponse (node → router) ─────────────────────────────────────

fn vc_to_pb(vc: &HashMap<String, u64>) -> pb::VectorClockMsg {
    pb::VectorClockMsg {
        entries: vc.clone(),
    }
}

fn pb_to_vc(m: pb::VectorClockMsg) -> HashMap<String, u64> {
    m.entries
}

pub fn encode_kvs_response(resp: &protocol::KvsResponse) -> Vec<u8> {
    build_node_response_msg(resp).encode_to_vec()
}

#[allow(clippy::disallowed_methods)] // iteration order irrelevant for protobuf serialization
fn build_node_response_msg(resp: &protocol::KvsResponse) -> pb::NodeResponse {
    match resp {
        protocol::KvsResponse::PutOk {
            trace_id,
            key,
            existing_vc,
            node_ids,
        } => pb::NodeResponse {
            trace_id: trace_id.clone(),
            key: key.clone(),
            existing_vc: existing_vc.as_ref().map(vc_to_pb),
            node_ids: node_ids.iter().cloned().collect(),
            kind: Some(pb::node_response::Kind::PutOk(pb::PutOkPayload {})),
        },
        protocol::KvsResponse::GetResult {
            trace_id,
            key,
            value,
            existing_vc,
            node_ids,
        } => pb::NodeResponse {
            trace_id: trace_id.clone(),
            key: key.clone(),
            existing_vc: existing_vc.as_ref().map(vc_to_pb),
            node_ids: node_ids.iter().cloned().collect(),
            kind: Some(pb::node_response::Kind::GetResult(pb::GetResultPayload {
                value: value.as_ref().map(|vs| pb::ValueSetMsg {
                    values: vs.iter().cloned().collect(),
                }),
            })),
        },
    }
}

pub fn decode_kvs_response(bytes: &[u8]) -> protocol::KvsResponse {
    let msg = pb::NodeResponse::decode(bytes).expect("decode NodeResponse");
    node_response_from_msg(msg)
}

fn node_response_from_msg(msg: pb::NodeResponse) -> protocol::KvsResponse {
    let trace_id = msg.trace_id;
    let key = msg.key;
    let existing_vc = msg.existing_vc.map(pb_to_vc);
    let node_ids: HashSet<String> = msg.node_ids.into_iter().collect();
    match msg.kind.expect("NodeResponse.kind") {
        pb::node_response::Kind::PutOk(_) => protocol::KvsResponse::PutOk {
            trace_id,
            key,
            existing_vc,
            node_ids,
        },
        pb::node_response::Kind::GetResult(p) => protocol::KvsResponse::GetResult {
            trace_id,
            key,
            value: p.value.map(|v| v.values.into_iter().collect()),
            existing_vc,
            node_ids,
        },
    }
}

// ── Rebalance transfer (node → node) ────────────────────────────────

#[allow(clippy::disallowed_methods)] // iteration order irrelevant for protobuf serialization
pub fn encode_rebalance(key: &str, cv: &ClockedValue) -> Vec<u8> {
    let msg = pb::RebalanceTransfer {
        key: key.to_string(),
        vector_clock: cv
            .vector_clock
            .iter()
            .map(|(k, v)| (k.clone(), *v as u64))
            .collect(),
        values: cv.values.clone(),
    };
    msg.encode_to_vec()
}

pub fn decode_rebalance(bytes: &[u8]) -> (String, ClockedValue) {
    let msg = pb::RebalanceTransfer::decode(bytes).expect("decode RebalanceTransfer");
    let cv = ClockedValue {
        vector_clock: msg
            .vector_clock
            .into_iter()
            .map(|(k, v)| (k, v as i64))
            .collect(),
        values: msg.values,
    };
    (msg.key, cv)
}
