//! Wire types for the KVS protocol.
//!
//! `KvsCommand`, `NodeCommand`, and `KvsResponse` are thin dispatch enums used
//! in the dataflow. `ClockedValue` is a simple data carrier for a vector clock
//! + value set, used by the inter-node protobuf codec.
//!
//! Inter-node serialization uses prost/protobuf via `proto_codec`.

use std::collections::{HashMap, HashSet};

use lattices::map_union::MapUnion;
use lattices::set_union::SetUnionHashSet;
use lattices::{DomPair, Max};

/// A vector clock + value set, used internally as a data carrier between
/// the router and storage nodes; the client never sees it.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ClockedValue {
    pub vector_clock: HashMap<String, i64>,
    pub values: Vec<String>,
}

pub type InternalClockedSet = DomPair<MapUnion<HashMap<String, Max<u64>>>, SetUnionHashSet<String>>;
pub type VectorClock = HashMap<String, u64>;

#[allow(clippy::disallowed_methods)] // iteration order irrelevant for serialization
pub fn clocked_set_to_cv(cs: InternalClockedSet) -> ClockedValue {
    let vc = cs.as_reveal_ref().0.as_reveal_ref();
    let vals = cs.as_reveal_ref().1.as_reveal_ref();
    ClockedValue {
        vector_clock: vc
            .iter()
            .map(|(k, v)| (k.clone(), v.into_reveal() as i64))
            .collect(),
        values: vals.iter().cloned().collect(),
    }
}

pub fn cv_to_clocked_set(cv: ClockedValue) -> InternalClockedSet {
    DomPair::new(
        MapUnion::new(
            cv.vector_clock
                .into_iter()
                .map(|(k, v)| (k, Max::new(v as u64)))
                .collect(),
        ),
        SetUnionHashSet::new(cv.values.into_iter().collect()),
    )
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum KvsCommand {
    Put {
        trace_id: String,
        key: String,
        value: String,
    },
    Get {
        trace_id: String,
        key: String,
    },
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum NodeCommand {
    ClockedPut {
        trace_id: String,
        key: String,
        clocked_value: ClockedValue,
    },
    Get {
        trace_id: String,
        key: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum KvsResponse {
    PutOk {
        trace_id: String,
        key: String,
        existing_vc: Option<HashMap<String, u64>>,
        node_ids: HashSet<String>,
    },
    GetResult {
        trace_id: String,
        key: String,
        value: Option<HashSet<String>>,
        existing_vc: Option<HashMap<String, u64>>,
        node_ids: HashSet<String>,
    },
}

/// Which ingress produced an in-flight command. Threaded through the
/// dataflow as part of the `(Ingress, u64)` envelope so the ingress
/// layer can demux responses back to the originating sidecar without
/// rewriting or reinterpreting the sidecar-owned `u64` request id.
///
/// The sidecar-side variants ([`Ingress::Grpc`], [`Ingress::Ws`])
/// correspond 1:1 to the sidecars registered in
/// `complete_distributed_kvs`. The [`Ingress::Test`] variant is used by
/// sim and fuzz tests that call `distributed_kvs` directly with no
/// sidecar in between.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Ingress {
    Grpc,
    Ws,
    Test,
}
