//! Lean JSON emitter for coordination analysis.
//!
//! Produces a minimal graph representation of the Hydro IR suitable for
//! consumption by a standalone coordination analysis tool. Unlike the viz
//! JSON (which includes backtraces, hierarchy, styling), this format contains
//! only the information needed for backward goal-seeking analysis:
//!
//! - Operator identity (HydroNode variant name)
//! - Collection kind (boundedness, ordering, collection type)
//! - Location identity and type
//! - Algebraic properties (commutativity, idempotency, associativity)
//! - Networking info (fault model, durability)
//! - Cycle IDs
//! - Sink tags and observability

use std::collections::HashMap;

use serde::Serialize;
use slotmap::SecondaryMap;

use crate::compile::ir::{HydroIrMetadata, HydroNode, HydroRoot};
use crate::location::{LocationKey, LocationType};

// ---------------------------------------------------------------------------
// Serializable types
// ---------------------------------------------------------------------------

/// The top-level analysis graph.
#[derive(Serialize)]
pub struct AnalysisGraph {
    pub nodes: Vec<AnalysisNode>,
    pub edges: Vec<AnalysisEdge>,
    pub roots: Vec<AnalysisRoot>,
    pub locations: Vec<AnalysisLocation>,
}

/// A node in the analysis graph.
#[derive(Serialize)]
pub struct AnalysisNode {
    pub id: usize,
    /// HydroNode variant name (e.g. "map", "fold_keyed", "scan", "network").
    pub operator: &'static str,
    /// Location where this node executes.
    #[serde(rename = "locationId")]
    pub location_id: String,
    /// Collection kind at this node's output.
    #[serde(rename = "collectionKind")]
    pub collection_kind: CollectionKindInfo,
    /// Algebraic properties (fold/reduce nodes only).
    #[serde(rename = "algebraicProperties", skip_serializing_if = "Option::is_none")]
    pub algebraic_properties: Option<AlgebraicProperties>,
    /// Networking info (Network nodes only).
    #[serde(rename = "networkingInfo", skip_serializing_if = "Option::is_none")]
    pub networking_info: Option<NetworkingInfoSer>,
    /// Cycle ID (CycleSource nodes only).
    #[serde(rename = "cycleId", skip_serializing_if = "Option::is_none")]
    pub cycle_id: Option<usize>,
    /// User-assigned tag/name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    /// Closure expressions as strings (for display in reports).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub closures: Vec<String>,
}

/// Directed edge in the analysis graph.
#[derive(Serialize)]
pub struct AnalysisEdge {
    pub source: usize,
    pub target: usize,
    /// Role label for multi-input operators (e.g. "left", "right", "pos", "neg").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<&'static str>,
}

/// A root (sink) in the IR.
#[derive(Serialize)]
pub struct AnalysisRoot {
    pub kind: &'static str,
    /// ID of the input node.
    #[serde(rename = "inputNodeId")]
    pub input_node_id: usize,
    /// Cycle ID (CycleSink only).
    #[serde(rename = "cycleId", skip_serializing_if = "Option::is_none")]
    pub cycle_id: Option<usize>,
    /// User-assigned tag/name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
}

/// Location metadata.
#[derive(Serialize)]
pub struct AnalysisLocation {
    pub key: String,
    #[serde(rename = "type")]
    pub location_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Serializable collection kind.
#[derive(Serialize)]
pub struct CollectionKindInfo {
    /// "Stream", "Singleton", "Optional", "KeyedStream", "KeyedSingleton"
    pub variant: &'static str,
    pub bounded: bool,
    /// "TotalOrder" or "NoOrder" (streams only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<&'static str>,
}

/// Algebraic properties for fold/reduce operators.
#[derive(Serialize)]
pub struct AlgebraicProperties {
    pub commutative: bool,
    pub idempotent: bool,
    pub associative: bool,
}

/// Serializable networking info.
#[derive(Serialize)]
pub struct NetworkingInfoSer {
    pub fault: &'static str,
    pub durable: bool,
    #[serde(rename = "gapFree")]
    pub gap_free: bool,
}

// ---------------------------------------------------------------------------
// Builder
// ---------------------------------------------------------------------------

struct AnalysisBuilder {
    nodes: Vec<AnalysisNode>,
    edges: Vec<AnalysisEdge>,
    roots: Vec<AnalysisRoot>,
    locations: HashMap<String, (LocationType, Option<String>)>,
    seen_tees: HashMap<*const std::cell::RefCell<HydroNode>, usize>,
    next_id: usize,
}

impl AnalysisBuilder {
    fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            roots: Vec::new(),
            locations: HashMap::new(),
            seen_tees: HashMap::new(),
            next_id: 0,
        }
    }

    fn alloc_id(&mut self) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn record_location(&mut self, metadata: &HydroIrMetadata, location_names: &SecondaryMap<LocationKey, String>) {
        let loc_id = metadata.location_id.root();
        let key_str = format!("{:?}", loc_id);
        if !self.locations.contains_key(&key_str) {
            let loc_type = loc_id.location_type().unwrap_or(LocationType::Process);
            let name = location_names.get(loc_id.key()).cloned();
            self.locations.insert(key_str, (loc_type, name));
        }
    }

    fn visit_root(&mut self, root: &HydroRoot, location_names: &SecondaryMap<LocationKey, String>) {
        let (kind, input, cycle_id, tag) = match root {
            HydroRoot::ForEach { input, .. } => {
                ("for_each", input.as_ref(), None, input.metadata().tag.clone())
            }
            HydroRoot::SendExternal { input, .. } => {
                ("send_external", input.as_ref(), None, input.metadata().tag.clone())
            }
            HydroRoot::DestSink { input, .. } => {
                ("dest_sink", input.as_ref(), None, input.metadata().tag.clone())
            }
            HydroRoot::CycleSink { cycle_id, input, .. } => {
                ("cycle_sink", input.as_ref(), Some(*cycle_id), input.metadata().tag.clone())
            }
            HydroRoot::EmbeddedOutput { input, .. } => {
                ("embedded_output", input.as_ref(), None, input.metadata().tag.clone())
            }
            HydroRoot::Null { input, .. } => {
                ("null", input.as_ref(), None, input.metadata().tag.clone())
            }
        };
        let input_node_id = self.visit_node(input, location_names);
        self.roots.push(AnalysisRoot {
            kind,
            input_node_id,
            cycle_id: cycle_id.map(|c| c.into_inner()),
            tag,
        });
    }

    fn visit_node(&mut self, node: &HydroNode, location_names: &SecondaryMap<LocationKey, String>) -> usize {
        // Placeholder should not appear in finalized IR
        if matches!(node, HydroNode::Placeholder) {
            let id = self.alloc_id();
            return id;
        }

        // Handle Tee/Partition deduplication
        match node {
            HydroNode::Tee { inner, metadata } | HydroNode::Partition { inner, metadata, .. } => {
                let ptr = inner.as_ptr();
                if let Some(&existing_id) = self.seen_tees.get(&ptr) {
                    return existing_id;
                }
                let inner_id = self.visit_node(&inner.0.borrow(), location_names);
                let id = self.alloc_id();
                self.seen_tees.insert(ptr, id);
                self.record_location(metadata, location_names);
                let analysis_node = make_analysis_node(id, node, metadata);
                self.nodes.push(analysis_node);
                self.edges.push(AnalysisEdge { source: inner_id, target: id, label: None });
                return id;
            }
            _ => {}
        }

        let metadata = node.metadata();
        self.record_location(metadata, location_names);
        let id = self.alloc_id();

        // Visit children first, then add edges
        match node {
            // Sources (no inputs)
            HydroNode::Source { .. }
            | HydroNode::SingletonSource { .. }
            | HydroNode::ExternalInput { .. }
            | HydroNode::CycleSource { .. } => {}

            // Placeholder should not appear (early return above)
            HydroNode::Placeholder => unreachable!(),

            // Already handled above
            HydroNode::Tee { .. } | HydroNode::Partition { .. } => unreachable!(),

            // Single input (inner)
            HydroNode::Cast { inner, .. }
            | HydroNode::ObserveNonDet { inner, .. }
            | HydroNode::BeginAtomic { inner, .. }
            | HydroNode::EndAtomic { inner, .. }
            | HydroNode::Batch { inner, .. }
            | HydroNode::YieldConcat { inner, .. } => {
                let child_id = self.visit_node(inner, location_names);
                self.edges.push(AnalysisEdge { source: child_id, target: id, label: None });
            }

            // Single input (input)
            HydroNode::Map { input, .. }
            | HydroNode::FlatMap { input, .. }
            | HydroNode::FlatMapStreamBlocking { input, .. }
            | HydroNode::Filter { input, .. }
            | HydroNode::FilterMap { input, .. }
            | HydroNode::Inspect { input, .. }
            | HydroNode::Sort { input, .. }
            | HydroNode::DeferTick { input, .. }
            | HydroNode::Enumerate { input, .. }
            | HydroNode::Unique { input, .. }
            | HydroNode::Network { input, .. }
            | HydroNode::Counter { input, .. }
            | HydroNode::ResolveFutures { input, .. }
            | HydroNode::ResolveFuturesBlocking { input, .. }
            | HydroNode::ResolveFuturesOrdered { input, .. }
            | HydroNode::Fold { input, .. }
            | HydroNode::FoldKeyed { input, .. }
            | HydroNode::Reduce { input, .. }
            | HydroNode::ReduceKeyed { input, .. }
            | HydroNode::Scan { input, .. }
            | HydroNode::ScanAsyncBlocking { input, .. } => {
                let child_id = self.visit_node(input, location_names);
                self.edges.push(AnalysisEdge { source: child_id, target: id, label: None });
            }

            // Two inputs: left/right
            HydroNode::Join { left, right, .. }
            | HydroNode::JoinBounded { left, right, .. }
            | HydroNode::CrossProduct { left, right, .. }
            | HydroNode::CrossSingleton { left, right, .. } => {
                let left_id = self.visit_node(left, location_names);
                let right_id = self.visit_node(right, location_names);
                self.edges.push(AnalysisEdge { source: left_id, target: id, label: Some("left") });
                self.edges.push(AnalysisEdge { source: right_id, target: id, label: Some("right") });
            }

            // Two inputs: pos/neg
            HydroNode::Difference { pos, neg, .. }
            | HydroNode::AntiJoin { pos, neg, .. } => {
                let pos_id = self.visit_node(pos, location_names);
                let neg_id = self.visit_node(neg, location_names);
                self.edges.push(AnalysisEdge { source: pos_id, target: id, label: Some("pos") });
                self.edges.push(AnalysisEdge { source: neg_id, target: id, label: Some("neg") });
            }

            // Two inputs: first/second
            HydroNode::Chain { first, second, .. }
            | HydroNode::ChainFirst { first, second, .. } => {
                let first_id = self.visit_node(first, location_names);
                let second_id = self.visit_node(second, location_names);
                self.edges.push(AnalysisEdge { source: first_id, target: id, label: Some("first") });
                self.edges.push(AnalysisEdge { source: second_id, target: id, label: Some("second") });
            }

            // Two inputs: input + watermark
            HydroNode::ReduceKeyedWatermark { input, watermark, .. } => {
                let input_id = self.visit_node(input, location_names);
                let watermark_id = self.visit_node(watermark, location_names);
                self.edges.push(AnalysisEdge { source: input_id, target: id, label: Some("input") });
                self.edges.push(AnalysisEdge { source: watermark_id, target: id, label: Some("watermark") });
            }
        }

        let analysis_node = make_analysis_node(id, node, metadata);
        self.nodes.push(analysis_node);
        id
    }

    fn finish(self) -> AnalysisGraph {
        let locations = self.locations.into_iter().map(|(key, (loc_type, name))| {
            AnalysisLocation {
                key,
                location_type: format!("{:?}", loc_type),
                name,
            }
        }).collect();

        AnalysisGraph {
            nodes: self.nodes,
            edges: self.edges,
            roots: self.roots,
            locations,
        }
    }
}

fn make_analysis_node(
    id: usize,
    node: &HydroNode,
    metadata: &HydroIrMetadata,
) -> AnalysisNode {
    let loc_id = metadata.location_id.root();
    let location_id = format!("{:?}", loc_id);

    let collection_kind = collection_kind_info(&metadata.collection_kind);

    let algebraic_properties = match node {
        HydroNode::Fold { is_commutative, is_idempotent, is_associative, .. }
        | HydroNode::FoldKeyed { is_commutative, is_idempotent, is_associative, .. }
        | HydroNode::Reduce { is_commutative, is_idempotent, is_associative, .. }
        | HydroNode::ReduceKeyed { is_commutative, is_idempotent, is_associative, .. }
        | HydroNode::ReduceKeyedWatermark { is_commutative, is_idempotent, is_associative, .. } => {
            Some(AlgebraicProperties {
                commutative: *is_commutative,
                idempotent: *is_idempotent,
                associative: *is_associative,
            })
        }
        _ => None,
    };

    let networking_info = match node {
        HydroNode::Network { networking_info, .. } => {
            Some(networking_info_ser(networking_info))
        }
        _ => None,
    };

    let cycle_id = match node {
        HydroNode::CycleSource { cycle_id, .. } => Some(cycle_id.into_inner()),
        _ => None,
    };

    let closures = extract_closures(node);

    AnalysisNode {
        id,
        operator: operator_name(node),
        location_id,
        collection_kind,
        algebraic_properties,
        networking_info,
        cycle_id,
        tag: metadata.tag.clone(),
        closures,
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn operator_name(node: &HydroNode) -> &'static str {
    match node {
        HydroNode::Placeholder => "placeholder",
        HydroNode::Source { .. } => "source",
        HydroNode::SingletonSource { .. } => "singleton_source",
        HydroNode::CycleSource { .. } => "cycle_source",
        HydroNode::ExternalInput { .. } => "external_input",
        HydroNode::Tee { .. } => "tee",
        HydroNode::Partition { .. } => "partition",
        HydroNode::Cast { .. } => "cast",
        HydroNode::ObserveNonDet { .. } => "observe_nondet",
        HydroNode::BeginAtomic { .. } => "begin_atomic",
        HydroNode::EndAtomic { .. } => "end_atomic",
        HydroNode::YieldConcat { .. } => "yield_concat",
        HydroNode::Batch { .. } => "batch",
        HydroNode::Map { .. } => "map",
        HydroNode::FlatMap { .. } => "flat_map",
        HydroNode::FlatMapStreamBlocking { .. } => "flat_map_stream_blocking",
        HydroNode::Filter { .. } => "filter",
        HydroNode::FilterMap { .. } => "filter_map",
        HydroNode::Inspect { .. } => "inspect",
        HydroNode::Enumerate { .. } => "enumerate",
        HydroNode::Unique { .. } => "unique",
        HydroNode::Sort { .. } => "sort",
        HydroNode::DeferTick { .. } => "defer_tick",
        HydroNode::Network { .. } => "network",
        HydroNode::Counter { .. } => "counter",
        HydroNode::ResolveFutures { .. } => "resolve_futures",
        HydroNode::ResolveFuturesBlocking { .. } => "resolve_futures_blocking",
        HydroNode::ResolveFuturesOrdered { .. } => "resolve_futures_ordered",
        HydroNode::Chain { .. } => "chain",
        HydroNode::ChainFirst { .. } => "chain_first",
        HydroNode::Join { .. } => "join",
        HydroNode::JoinBounded { .. } => "join_bounded",
        HydroNode::CrossProduct { .. } => "cross_product",
        HydroNode::CrossSingleton { .. } => "cross_singleton",
        HydroNode::Difference { .. } => "difference",
        HydroNode::AntiJoin { .. } => "anti_join",
        HydroNode::Fold { .. } => "fold",
        HydroNode::FoldKeyed { .. } => "fold_keyed",
        HydroNode::Reduce { .. } => "reduce",
        HydroNode::ReduceKeyed { .. } => "reduce_keyed",
        HydroNode::ReduceKeyedWatermark { .. } => "reduce_keyed_watermark",
        HydroNode::Scan { .. } => "scan",
        HydroNode::ScanAsyncBlocking { .. } => "scan_async_blocking",
    }
}

fn collection_kind_info(kind: &crate::compile::ir::CollectionKind) -> CollectionKindInfo {
    use crate::compile::ir::{BoundKind, CollectionKind, KeyedSingletonBoundKind, SingletonBoundKind, StreamOrder};
    match kind {
        CollectionKind::Stream { bound, order, .. } => CollectionKindInfo {
            variant: "Stream",
            bounded: matches!(bound, BoundKind::Bounded),
            order: Some(match order {
                StreamOrder::TotalOrder => "TotalOrder",
                StreamOrder::NoOrder => "NoOrder",
            }),
        },
        CollectionKind::Singleton { bound, .. } => CollectionKindInfo {
            variant: "Singleton",
            bounded: matches!(bound, SingletonBoundKind::Bounded),
            order: None,
        },
        CollectionKind::Optional { bound, .. } => CollectionKindInfo {
            variant: "Optional",
            bounded: matches!(bound, BoundKind::Bounded),
            order: None,
        },
        CollectionKind::KeyedStream { bound, value_order, .. } => CollectionKindInfo {
            variant: "KeyedStream",
            bounded: matches!(bound, BoundKind::Bounded),
            order: Some(match value_order {
                StreamOrder::TotalOrder => "TotalOrder",
                StreamOrder::NoOrder => "NoOrder",
            }),
        },
        CollectionKind::KeyedSingleton { bound, .. } => CollectionKindInfo {
            variant: "KeyedSingleton",
            bounded: matches!(bound, KeyedSingletonBoundKind::Bounded),
            order: None,
        },
    }
}

fn networking_info_ser(info: &crate::networking::NetworkingInfo) -> NetworkingInfoSer {
    use crate::networking::TcpFault;
    NetworkingInfoSer {
        fault: match info.fault() {
            TcpFault::FailStop => "FailStop",
            TcpFault::Lossy => "Lossy",
            TcpFault::LossyDelayedForever => "LossyDelayedForever",
        },
        durable: info.is_durable(),
        gap_free: info.is_gap_free(),
    }
}

fn extract_closures(node: &HydroNode) -> Vec<String> {
    match node {
        HydroNode::Map { f, .. }
        | HydroNode::FlatMap { f, .. }
        | HydroNode::FlatMapStreamBlocking { f, .. }
        | HydroNode::Filter { f, .. }
        | HydroNode::FilterMap { f, .. }
        | HydroNode::Inspect { f, .. }
        | HydroNode::Reduce { f, .. }
        | HydroNode::ReduceKeyed { f, .. }
        | HydroNode::ReduceKeyedWatermark { f, .. } => vec![f.to_string()],
        HydroNode::Fold { init, acc, .. }
        | HydroNode::FoldKeyed { init, acc, .. }
        | HydroNode::Scan { init, acc, .. }
        | HydroNode::ScanAsyncBlocking { init, acc, .. } => vec![init.to_string(), acc.to_string()],
        _ => vec![],
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Build the analysis graph from Hydro IR roots.
pub fn build_analysis_graph(
    ir: &[HydroRoot],
    location_names: &SecondaryMap<LocationKey, String>,
) -> AnalysisGraph {
    let mut builder = AnalysisBuilder::new();
    for root in ir {
        builder.visit_root(root, location_names);
    }
    builder.finish()
}

/// Emit the analysis graph as a JSON string.
pub fn emit_analysis_json(
    ir: &[HydroRoot],
    location_names: &SecondaryMap<LocationKey, String>,
) -> Result<String, serde_json::Error> {
    let graph = build_analysis_graph(ir, location_names);
    serde_json::to_string_pretty(&graph)
}
