use std::collections::HashMap;
use std::error::Error;
use std::fmt::Write;

use auto_impl::auto_impl;

pub use super::graphviz::{HydroDot, escape_dot};
// Re-export specific implementations
pub use super::mermaid::{HydroMermaid, escape_mermaid};
pub use super::reactflow::HydroReactFlow;
use crate::ir::{DebugExpr, HydroLeaf, HydroNode, HydroSource};

/// Label for a graph node - can be either a static string or contain expressions.
#[derive(Debug, Clone)]
pub enum NodeLabel {
    /// A static string label
    Static(String),
    /// A label with an operation name and expression arguments
    WithExprs {
        op_name: String,
        exprs: Vec<DebugExpr>,
    },
}

impl NodeLabel {
    /// Create a static label
    pub fn static_label<S: Into<String>>(s: S) -> Self {
        Self::Static(s.into())
    }

    /// Create a label for an operation with multiple expression
    pub fn with_exprs(op_name: String, exprs: Vec<DebugExpr>) -> Self {
        Self::WithExprs { op_name, exprs }
    }
}

impl std::fmt::Display for NodeLabel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Static(s) => write!(f, "{}", s),
            Self::WithExprs { op_name, exprs } => {
                if exprs.is_empty() {
                    write!(f, "{}()", op_name)
                } else {
                    let expr_strs: Vec<_> = exprs.iter().map(|e| e.to_string()).collect();
                    write!(f, "{}({})", op_name, expr_strs.join(", "))
                }
            }
        }
    }
}

/// Base struct for text-based graph writers that use indentation.
/// Contains common fields shared by DOT and Mermaid writers.
pub struct IndentedGraphWriter<W> {
    pub write: W,
    pub indent: usize,
    pub config: HydroWriteConfig,
}

impl<W> IndentedGraphWriter<W> {
    /// Create a new writer with default configuration.
    pub fn new(write: W) -> Self {
        Self {
            write,
            indent: 0,
            config: HydroWriteConfig::default(),
        }
    }

    /// Create a new writer with the given configuration.
    pub fn new_with_config(write: W, config: &HydroWriteConfig) -> Self {
        Self {
            write,
            indent: 0,
            config: config.clone(),
        }
    }
}

impl<W: Write> IndentedGraphWriter<W> {
    /// Write an indented line using the current indentation level.
    pub fn writeln_indented(&mut self, content: &str) -> Result<(), std::fmt::Error> {
        writeln!(self.write, "{b:i$}{content}", b = "", i = self.indent)
    }
}

/// Common error type used by all graph writers.
pub type GraphWriteError = std::fmt::Error;

/// Trait for writing textual representations of Hydro IR graphs, i.e. mermaid or dot graphs.
#[auto_impl(&mut, Box)]
pub trait HydroGraphWrite {
    /// Error type emitted by writing.
    type Err: Error;

    /// Begin the graph. First method called.
    fn write_prologue(&mut self) -> Result<(), Self::Err>;

    /// Write a node definition with styling.
    fn write_node_definition(
        &mut self,
        node_id: usize,
        node_label: &NodeLabel,
        node_type: HydroNodeType,
        location_id: Option<usize>,
        location_type: Option<&str>,
    ) -> Result<(), Self::Err>;

    /// Write an edge between nodes with optional labeling.
    fn write_edge(
        &mut self,
        src_id: usize,
        dst_id: usize,
        edge_type: HydroEdgeType,
        label: Option<&str>,
    ) -> Result<(), Self::Err>;

    /// Begin writing a location grouping (process/cluster).
    fn write_location_start(
        &mut self,
        location_id: usize,
        location_type: &str,
    ) -> Result<(), Self::Err>;

    /// Write a node within a location.
    fn write_node(&mut self, node_id: usize) -> Result<(), Self::Err>;

    /// End writing a location grouping.
    fn write_location_end(&mut self) -> Result<(), Self::Err>;

    /// End the graph. Last method called.
    fn write_epilogue(&mut self) -> Result<(), Self::Err>;
}

/// Types of nodes in Hydro IR for styling purposes.
#[derive(Debug, Clone, Copy)]
pub enum HydroNodeType {
    Source,
    Transform,
    Join,
    Aggregation,
    Network,
    Sink,
    Tee,
}

/// Types of edges in Hydro IR.
#[derive(Debug, Clone, Copy)]
pub enum HydroEdgeType {
    Stream,
    Persistent,
    Network,
    Cycle,
}

/// Configuration for graph writing.
#[derive(Debug, Clone)]
pub struct HydroWriteConfig {
    pub show_metadata: bool,
    pub show_location_groups: bool,
    pub use_short_labels: bool,
    pub process_id_name: Vec<(usize, String)>,
    pub cluster_id_name: Vec<(usize, String)>,
    pub external_id_name: Vec<(usize, String)>,
}

impl Default for HydroWriteConfig {
    fn default() -> Self {
        Self {
            show_metadata: false,
            show_location_groups: true,
            use_short_labels: true, // Default to short labels for all renderers
            process_id_name: vec![],
            cluster_id_name: vec![],
            external_id_name: vec![],
        }
    }
}

// No fallback defaults: edge labels rely on recorded metadata and upstream inference only.

/// Graph structure tracker for Hydro IR rendering.
#[derive(Debug, Default)]
pub struct HydroGraphStructure {
    pub nodes: HashMap<usize, (NodeLabel, HydroNodeType, Option<usize>)>, /* node_id -> (label, type, location) */
    pub edges: Vec<(usize, usize, HydroEdgeType, Option<String>)>, // (src, dst, edge_type, label)
    pub locations: HashMap<usize, String>,                         // location_id -> location_type
    pub next_node_id: usize,
}

impl HydroGraphStructure {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(
        &mut self,
        label: NodeLabel,
        node_type: HydroNodeType,
        location: Option<usize>,
    ) -> usize {
        let node_id = self.next_node_id;
        self.next_node_id += 1;
        self.nodes.insert(node_id, (label, node_type, location));
        node_id
    }

    pub fn add_edge(
        &mut self,
        src: usize,
        dst: usize,
        edge_type: HydroEdgeType,
        label: Option<String>,
    ) {
        self.edges.push((src, dst, edge_type, label));
    }

    pub fn add_location(&mut self, location_id: usize, location_type: String) {
        self.locations.insert(location_id, location_type);
    }
}

/// Function to extract an op_name from a print_root() result for use in labels.
pub fn extract_op_name(full_label: String) -> String {
    full_label
        .split('(')
        .next()
        .unwrap_or("unknown")
        .to_string()
        .to_lowercase()
}

/// Extract a short, readable label from the full token stream label using print_root() style naming
pub fn extract_short_label(full_label: &str) -> String {
    // Use the same logic as extract_op_name but handle the specific cases we need for UI display
    if let Some(op_name) = full_label.split('(').next() {
        let base_name = op_name.to_lowercase();
        match base_name.as_str() {
            // Handle special cases for UI display
            "source" => {
                if full_label.contains("Iter") {
                    "source_iter".to_string()
                } else if full_label.contains("Stream") {
                    "source_stream".to_string()
                } else if full_label.contains("ExternalNetwork") {
                    "external_network".to_string()
                } else if full_label.contains("Spin") {
                    "spin".to_string()
                } else {
                    "source".to_string()
                }
            }
            "network" => {
                if full_label.contains("deser") {
                    "network(recv)".to_string()
                } else if full_label.contains("ser") {
                    "network(send)".to_string()
                } else {
                    "network".to_string()
                }
            }
            // For all other cases, just use the lowercase base name (same as extract_op_name)
            _ => base_name,
        }
    } else {
        // Fallback for labels that don't follow the pattern
        if full_label.len() > 20 {
            format!("{}...", &full_label[..17])
        } else {
            full_label.to_string()
        }
    }
}

// Removed unused pretty-printer helper to satisfy strict clippy settings.

/// Build a semantic label (collection kind + boundedness + order) from a type token string.
fn semantic_label_from_tokens(tokens: &str) -> Option<String> {
    // Lightweight, robust scanning: look for known collection kinds and markers anywhere in the token string.
    // Normalize whitespace without chained replace to keep clippy quiet under strict settings.
    let s: String = tokens
        .chars()
        .map(|c| match c {
            '\n' | '\t' => ' ',
            _ => c,
        })
        .collect();
    let kind = if s.contains("KeyedStream") {
        "KeyedStream"
    } else if s.contains("Stream") {
        "Stream"
    } else if s.contains("Optional") {
        "Optional"
    } else if s.contains("Singleton") {
        "Singleton"
    } else {
        return None;
    };

    let mut parts = vec![kind.to_string()];
    // Prefer more specific/stronger markers when both appear by accident
    if s.contains("Unbounded") {
        parts.push("Unbounded".to_string());
    } else if s.contains("Bounded") {
        parts.push("Bounded".to_string());
    }
    if s.contains("TotalOrder") {
        parts.push("TotalOrder".to_string());
    } else if s.contains("NoOrder") {
        parts.push("NoOrder".to_string());
    }
    Some(parts.join(" "))
}

/// Build a semantic type label from node metadata using collection_type only.
fn type_label_from_metadata(meta: &crate::ir::HydroIrMetadata) -> Option<String> {
    meta.collection_type
        .as_ref()
        .and_then(|col| semantic_label_from_tokens(&format!("{:?}", col)))
}

/// Try to find a semantic label for a node by looking at its own metadata,
/// falling back to walking upstream through single-input wrappers.
fn find_semantic_label_upstream(node: &HydroNode) -> Option<String> {
    use HydroNode::*;
    // Prevent excessive walking in degenerate cases
    let mut hops = 0usize;
    let mut cur = node;
    loop {
        if let Some(lbl) = type_label_from_metadata(cur.metadata()) {
            return Some(lbl);
        }
        if hops > 64 {
            return None;
        }
        hops += 1;
        // Use a temporary variable for tee borrow to avoid returning reference to a temporary.
        cur = match cur {
                // Treat persist boundaries as semantic breakpoints; don't walk past them
                Persist { .. }
                | Unpersist { .. } => return None,
                Delta { inner, .. }
            | Enumerate { input: inner, .. }
            | Unique { input: inner, .. }
            | Sort { input: inner, .. }
            | Map { input: inner, .. }
            | Filter { input: inner, .. }
            | FlatMap { input: inner, .. }
            | FilterMap { input: inner, .. }
            | Inspect { input: inner, .. }
            | Reduce { input: inner, .. }
            | ReduceKeyed { input: inner, .. }
            | Fold { input: inner, .. }
            | FoldKeyed { input: inner, .. }
            | Scan { input: inner, .. }
            | Counter { input: inner, .. }
            | DeferTick { input: inner, .. }
            | ResolveFutures { input: inner, .. }
            | ResolveFuturesOrdered { input: inner, .. } => inner,
            Tee { inner, .. } => {
                // SAFETY: We immediately break out of this arm by cloning the needed info; but since we need a &HydroNode,
                // we can't hold the Ref across loop iterations. Instead, fall back to using metadata on the Tee itself
                // and stop walking further if Tee doesn't have it.
                // Try metadata on the tee one last time, then stop.
                if let Some(lbl) = type_label_from_metadata(inner.0.borrow().metadata()) {
                    return Some(lbl);
                }
                return None;
            }
            // Stop for multi-input and source-like nodes
            Source { .. }
            | ExternalInput { .. }
            | CycleSource { .. }
            | Join { .. }
            | CrossProduct { .. }
            | CrossSingleton { .. }
            | Difference { .. }
            | AntiJoin { .. }
            | Network { .. }
            | Placeholder
            | ReduceKeyedWatermark { .. }
            | Chain { .. } => return None,
        };
    }
}

/// Helper function to extract location ID and type from metadata.
fn extract_location_id(metadata: &crate::ir::HydroIrMetadata) -> (Option<usize>, Option<String>) {
    use crate::location::LocationId;
    match &metadata.location_kind {
        LocationId::Process(id) => (Some(*id), Some("Process".to_string())),
        LocationId::Cluster(id) => (Some(*id), Some("Cluster".to_string())),
        LocationId::Tick(_, inner) => match inner.as_ref() {
            LocationId::Process(id) => (Some(*id), Some("Process".to_string())),
            LocationId::Cluster(id) => (Some(*id), Some("Cluster".to_string())),
            _ => (None, None),
        },
    }
}

/// Helper function to set up location in structure from metadata.
fn setup_location(
    structure: &mut HydroGraphStructure,
    metadata: &crate::ir::HydroIrMetadata,
) -> Option<usize> {
    let (location_id, location_type) = extract_location_id(metadata);
    if let (Some(loc_id), Some(loc_type)) = (location_id, location_type) {
        structure.add_location(loc_id, loc_type);
    }
    location_id
}

impl HydroLeaf {
    /// Core graph writing logic that works with any GraphWrite implementation.
    pub fn write_graph<W>(
        &self,
        mut graph_write: W,
        config: &HydroWriteConfig,
    ) -> Result<(), W::Err>
    where
        W: HydroGraphWrite,
    {
        let mut structure = HydroGraphStructure::new();
        let mut seen_tees = HashMap::new();

        // Build the graph structure by traversing the IR
        let _sink_id = self.build_graph_structure(&mut structure, &mut seen_tees, config);

        // Write the graph
        graph_write.write_prologue()?;

        // Write node definitions
        for (&node_id, (label, node_type, location)) in &structure.nodes {
            let (location_id, location_type) = if let Some(loc_id) = location {
                (
                    Some(*loc_id),
                    structure.locations.get(loc_id).map(|s| s.as_str()),
                )
            } else {
                (None, None)
            };

            // Check if this is a label that came from an expression-containing operation
            // We can detect this by looking for the pattern "op_name(...)" and checking if we have the original expressions
            graph_write.write_node_definition(
                node_id,
                label,
                *node_type,
                location_id,
                location_type,
            )?;
        }

        // Group nodes by location if requested
        if config.show_location_groups {
            let mut nodes_by_location: HashMap<usize, Vec<usize>> = HashMap::new();
            for (&node_id, (_, _, location)) in &structure.nodes {
                if let Some(location_id) = location {
                    nodes_by_location
                        .entry(*location_id)
                        .or_default()
                        .push(node_id);
                }
            }

            for (&location_id, node_ids) in &nodes_by_location {
                if let Some(location_type) = structure.locations.get(&location_id) {
                    graph_write.write_location_start(location_id, location_type)?;
                    for &node_id in node_ids {
                        graph_write.write_node(node_id)?;
                    }
                    graph_write.write_location_end()?;
                }
            }
        }

        // Write edges
        for (src_id, dst_id, edge_type, label) in &structure.edges {
            graph_write.write_edge(*src_id, *dst_id, *edge_type, label.as_deref())?;
        }

        graph_write.write_epilogue()?;
        Ok(())
    }

    /// Build the graph structure by traversing the IR tree.
    pub fn build_graph_structure(
        &self,
        structure: &mut HydroGraphStructure,
        seen_tees: &mut HashMap<*const std::cell::RefCell<HydroNode>, usize>,
        config: &HydroWriteConfig,
    ) -> usize {
        // Helper function for sink nodes to reduce duplication
        fn build_sink_node(
            structure: &mut HydroGraphStructure,
            seen_tees: &mut HashMap<*const std::cell::RefCell<HydroNode>, usize>,
            config: &HydroWriteConfig,
            input: &HydroNode,
            metadata: Option<&crate::ir::HydroIrMetadata>,
            label: NodeLabel,
            edge_type: HydroEdgeType,
        ) -> usize {
            let input_id = input.build_graph_structure(structure, seen_tees, config);
            let location_id = metadata.and_then(|m| setup_location(structure, m));
            let sink_id = structure.add_node(label, HydroNodeType::Sink, location_id);
            let edge_label = if config.show_metadata {
                type_label_from_metadata(input.metadata())
            } else {
                None
            };
            structure.add_edge(input_id, sink_id, edge_type, edge_label);
            sink_id
        }

        match self {
            // Sink operations with Stream edges - grouped by edge type
            HydroLeaf::ForEach { f, input, metadata } => build_sink_node(
                structure,
                seen_tees,
                config,
                input,
                Some(metadata),
                NodeLabel::with_exprs("for_each".to_string(), vec![f.clone()]),
                HydroEdgeType::Stream,
            ),

            HydroLeaf::SendExternal {
                to_external_id,
                to_key,
                input,
                ..
            } => build_sink_node(
                structure,
                seen_tees,
                config,
                input,
                None,
                NodeLabel::with_exprs(
                    format!("send_external({}:{})", to_external_id, to_key),
                    vec![],
                ),
                HydroEdgeType::Stream,
            ),

            HydroLeaf::DestSink {
                sink,
                input,
                metadata,
            } => build_sink_node(
                structure,
                seen_tees,
                config,
                input,
                Some(metadata),
                NodeLabel::with_exprs("dest_sink".to_string(), vec![sink.clone()]),
                HydroEdgeType::Stream,
            ),

            // Sink operation with Cycle edge - grouped by edge type
            HydroLeaf::CycleSink {
                ident,
                input,
                metadata,
                ..
            } => build_sink_node(
                structure,
                seen_tees,
                config,
                input,
                Some(metadata),
                NodeLabel::static_label(format!("cycle_sink({})", ident)),
                HydroEdgeType::Cycle,
            ),
        }
    }
}

impl HydroNode {
    /// Build the graph structure recursively for this node.
    pub fn build_graph_structure(
        &self,
        structure: &mut HydroGraphStructure,
        seen_tees: &mut HashMap<*const std::cell::RefCell<HydroNode>, usize>,
        config: &HydroWriteConfig,
    ) -> usize {
        use crate::location::LocationId;

        // Helper functions to reduce duplication, categorized by input/expression patterns

        // Build an edge label for a single input with validation against the source node's output.
        // If `base` is provided (e.g., "left"/"right"), it will be prefixed on the first line.
        // Behavior when config.show_metadata is true:
        // - Prefer the downstream node's recorded input type; if missing, fall back to the source's output type.
        // - When both exist and differ, annotate with both to surface the mismatch.
        // When config.show_metadata is false:
        // - Return only the base label if provided; otherwise, no label.
        fn build_edge_label_single_input(
            base: Option<&str>,
            src_node: &HydroNode,
            dst_metadata: &crate::ir::HydroIrMetadata,
            input_index: usize,
            config: &HydroWriteConfig,
        ) -> Option<String> {
            // Always include base when present, even if metadata is hidden (for role labeling)
            if !config.show_metadata {
                return base.map(|b| b.to_string());
            }

            // Prefer direct metadata on the source node; if absent, walk upstream through
            // simple single-input wrappers to find a semantic label.
            let src_lbl = type_label_from_metadata(src_node.metadata())
                .or_else(|| find_semantic_label_upstream(src_node));
            let dst_lbl = dst_metadata
                .input_collection_types
                .get(input_index)
                .and_then(|ty| semantic_label_from_tokens(&format!("{:?}", ty)));

            // Decide primary label text (prefer destination input type when available)
            let primary = dst_lbl.as_ref().or(src_lbl.as_ref());

            // Nothing known on either side
            let Some(primary) = primary else {
                return base.map(|b| b.to_string());
            };

            let mut label = match base {
                Some(b) => format!("{}\n{}", b, primary),
                None => primary.to_string(),
            };

            // If both exist and differ, annotate concisely with both for validation visibility
            if let (Some(d), Some(s)) = (dst_lbl.as_ref(), src_lbl.as_ref()) {
                if d != s {
                    // Add a short mismatch note; keep it compact to avoid clutter
                    // Format: "<base>\n<dst> | out=<src>"
                    if base.is_some() {
                        label = format!("{} | out={}", label, s);
                    } else {
                        label = format!("{} | out={}", d, s);
                    }
                }
            }

            Some(label)
        }

        /// Common parameters for transform builder functions to reduce argument count
        struct TransformParams<'a> {
            structure: &'a mut HydroGraphStructure,
            seen_tees: &'a mut HashMap<*const std::cell::RefCell<HydroNode>, usize>,
            config: &'a HydroWriteConfig,
            input: &'a HydroNode,
            metadata: &'a crate::ir::HydroIrMetadata,
            op_name: String,
            node_type: HydroNodeType,
            edge_type: HydroEdgeType,
        }

        // Single-input transform with no expressions
        fn build_simple_transform(params: TransformParams) -> usize {
            let input_id = params.input.build_graph_structure(
                params.structure,
                params.seen_tees,
                params.config,
            );
            let location_id = setup_location(params.structure, params.metadata);
            let node_id = params.structure.add_node(
                NodeLabel::Static(params.op_name.to_string()),
                params.node_type,
                location_id,
            );
            let edge_label = build_edge_label_single_input(
                None,
                params.input,
                params.metadata,
                0,
                params.config,
            );
            params
                .structure
                .add_edge(input_id, node_id, params.edge_type, edge_label);
            node_id
        }

        // Single-input transform with one expression
        fn build_single_expr_transform(params: TransformParams, expr: &DebugExpr) -> usize {
            let input_id = params.input.build_graph_structure(
                params.structure,
                params.seen_tees,
                params.config,
            );
            let location_id = setup_location(params.structure, params.metadata);
            let node_id = params.structure.add_node(
                NodeLabel::with_exprs(params.op_name.to_string(), vec![expr.clone()]),
                params.node_type,
                location_id,
            );
            let edge_label = build_edge_label_single_input(
                None,
                params.input,
                params.metadata,
                0,
                params.config,
            );
            params
                .structure
                .add_edge(input_id, node_id, params.edge_type, edge_label);
            node_id
        }

        // Single-input transform with two expressions
        fn build_dual_expr_transform(
            params: TransformParams,
            expr1: &DebugExpr,
            expr2: &DebugExpr,
        ) -> usize {
            let input_id = params.input.build_graph_structure(
                params.structure,
                params.seen_tees,
                params.config,
            );
            let location_id = setup_location(params.structure, params.metadata);
            let node_id = params.structure.add_node(
                NodeLabel::with_exprs(
                    params.op_name.to_string(),
                    vec![expr1.clone(), expr2.clone()],
                ),
                params.node_type,
                location_id,
            );
            let edge_label = build_edge_label_single_input(
                None,
                params.input,
                params.metadata,
                0,
                params.config,
            );
            params
                .structure
                .add_edge(input_id, node_id, params.edge_type, edge_label);
            node_id
        }

        // Helper function for source nodes
        fn build_source_node(
            structure: &mut HydroGraphStructure,
            metadata: &crate::ir::HydroIrMetadata,
            label: String,
        ) -> usize {
            let location_id = setup_location(structure, metadata);
            structure.add_node(NodeLabel::Static(label), HydroNodeType::Source, location_id)
        }

        match self {
            HydroNode::Placeholder => structure.add_node(
                NodeLabel::Static("PLACEHOLDER".to_string()),
                HydroNodeType::Transform,
                None,
            ),

            HydroNode::Source {
                source, metadata, ..
            } => {
                let label = match source {
                    HydroSource::Stream(expr) => format!("source_stream({})", expr),
                    HydroSource::ExternalNetwork() => "external_network()".to_string(),
                    HydroSource::Iter(expr) => format!("source_iter({})", expr),
                    HydroSource::Spin() => "spin()".to_string(),
                };
                build_source_node(structure, metadata, label)
            }

            HydroNode::ExternalInput {
                from_external_id,
                from_key,
                metadata,
                ..
            } => build_source_node(
                structure,
                metadata,
                format!("external_input({}:{})", from_external_id, from_key),
            ),

            HydroNode::CycleSource {
                ident, metadata, ..
            } => build_source_node(structure, metadata, format!("cycle_source({})", ident)),

            HydroNode::Tee { inner, metadata } => {
                let ptr = inner.as_ptr();
                if let Some(&existing_id) = seen_tees.get(&ptr) {
                    return existing_id;
                }

                let input_id = inner
                    .0
                    .borrow()
                    .build_graph_structure(structure, seen_tees, config);
                let location_id = setup_location(structure, metadata);

                let tee_id = structure.add_node(
                    NodeLabel::Static(extract_op_name(self.print_root())),
                    HydroNodeType::Tee,
                    location_id,
                );

                seen_tees.insert(ptr, tee_id);

                let edge_label = if config.show_metadata {
                    find_semantic_label_upstream(&inner.0.borrow())
                } else {
                    None
                };
                structure.add_edge(input_id, tee_id, HydroEdgeType::Stream, edge_label);

                tee_id
            }

            // Transform operations with Stream edges - grouped by node/edge type
            HydroNode::Delta { inner, metadata }
            | HydroNode::DeferTick {
                input: inner,
                metadata,
            }
            | HydroNode::Enumerate {
                input: inner,
                metadata,
                ..
            }
            | HydroNode::Unique {
                input: inner,
                metadata,
            }
            | HydroNode::ResolveFutures {
                input: inner,
                metadata,
            }
            | HydroNode::ResolveFuturesOrdered {
                input: inner,
                metadata,
            } => build_simple_transform(TransformParams {
                structure,
                seen_tees,
                config,
                input: inner,
                metadata,
                op_name: extract_op_name(self.print_root()),
                node_type: HydroNodeType::Transform,
                edge_type: HydroEdgeType::Stream,
            }),

            // Transform operation with Persistent edge - grouped by node/edge type
            HydroNode::Persist { inner, metadata } => build_simple_transform(TransformParams {
                structure,
                seen_tees,
                config,
                input: inner,
                metadata,
                op_name: extract_op_name(self.print_root()),
                node_type: HydroNodeType::Transform,
                edge_type: HydroEdgeType::Persistent,
            }),

            // Aggregation operation with Stream edge - grouped by node/edge type
            HydroNode::Sort {
                input: inner,
                metadata,
            } => build_simple_transform(TransformParams {
                structure,
                seen_tees,
                config,
                input: inner,
                metadata,
                op_name: extract_op_name(self.print_root()),
                node_type: HydroNodeType::Aggregation,
                edge_type: HydroEdgeType::Stream,
            }),

            // Single-expression Transform operations - grouped by node type
            HydroNode::Map { f, input, metadata }
            | HydroNode::Filter { f, input, metadata }
            | HydroNode::FlatMap { f, input, metadata }
            | HydroNode::FilterMap { f, input, metadata }
            | HydroNode::Inspect { f, input, metadata } => build_single_expr_transform(
                TransformParams {
                    structure,
                    seen_tees,
                    config,
                    input,
                    metadata,
                    op_name: extract_op_name(self.print_root()),
                    node_type: HydroNodeType::Transform,
                    edge_type: HydroEdgeType::Stream,
                },
                f,
            ),

            // Single-expression Aggregation operations - grouped by node type
            HydroNode::Reduce { f, input, metadata }
            | HydroNode::ReduceKeyed { f, input, metadata } => build_single_expr_transform(
                TransformParams {
                    structure,
                    seen_tees,
                    config,
                    input,
                    metadata,
                    op_name: extract_op_name(self.print_root()),
                    node_type: HydroNodeType::Aggregation,
                    edge_type: HydroEdgeType::Stream,
                },
                f,
            ),

            // Join-like operations with left/right edge labels - grouped by edge labeling
            HydroNode::Join {
                left,
                right,
                metadata,
            }
            | HydroNode::CrossProduct {
                left,
                right,
                metadata,
            }
            | HydroNode::CrossSingleton {
                left,
                right,
                metadata,
            } => {
                let left_id = left.build_graph_structure(structure, seen_tees, config);
                let right_id = right.build_graph_structure(structure, seen_tees, config);
                let location_id = setup_location(structure, metadata);
                let node_id = structure.add_node(
                    NodeLabel::Static(extract_op_name(self.print_root())),
                    HydroNodeType::Join,
                    location_id,
                );
                structure.add_edge(
                    left_id,
                    node_id,
                    HydroEdgeType::Stream,
                    build_edge_label_single_input(Some("left"), left, metadata, 0, config),
                );
                structure.add_edge(
                    right_id,
                    node_id,
                    HydroEdgeType::Stream,
                    build_edge_label_single_input(Some("right"), right, metadata, 1, config),
                );
                node_id
            }

            // Join-like operations with pos/neg edge labels - grouped by edge labeling
            HydroNode::Difference {
                pos: left,
                neg: right,
                metadata,
            }
            | HydroNode::AntiJoin {
                pos: left,
                neg: right,
                metadata,
            } => {
                let left_id = left.build_graph_structure(structure, seen_tees, config);
                let right_id = right.build_graph_structure(structure, seen_tees, config);
                let location_id = setup_location(structure, metadata);
                let node_id = structure.add_node(
                    NodeLabel::Static(extract_op_name(self.print_root())),
                    HydroNodeType::Join,
                    location_id,
                );
                structure.add_edge(
                    left_id,
                    node_id,
                    HydroEdgeType::Stream,
                    build_edge_label_single_input(Some("pos"), left, metadata, 0, config),
                );
                structure.add_edge(
                    right_id,
                    node_id,
                    HydroEdgeType::Stream,
                    build_edge_label_single_input(Some("neg"), right, metadata, 1, config),
                );
                node_id
            }

            // Dual expression transforms - consolidated using pattern matching
            HydroNode::Fold {
                init,
                acc,
                input,
                metadata,
            }
            | HydroNode::FoldKeyed {
                init,
                acc,
                input,
                metadata,
            }
            | HydroNode::Scan {
                init,
                acc,
                input,
                metadata,
            } => {
                let node_type = HydroNodeType::Aggregation; // All are aggregation operations

                build_dual_expr_transform(
                    TransformParams {
                        structure,
                        seen_tees,
                        config,
                        input,
                        metadata,
                        op_name: extract_op_name(self.print_root()),
                        node_type,
                        edge_type: HydroEdgeType::Stream,
                    },
                    init,
                    acc,
                )
            }

            // Combination of join and transform
            HydroNode::ReduceKeyedWatermark {
                f,
                input,
                watermark,
                metadata,
            } => {
                let input_id = input.build_graph_structure(structure, seen_tees, config);
                let watermark_id = watermark.build_graph_structure(structure, seen_tees, config);
                let location_id = setup_location(structure, metadata);
                let join_node_id = structure.add_node(
                    NodeLabel::Static(extract_op_name(self.print_root())),
                    HydroNodeType::Join,
                    location_id,
                );
                structure.add_edge(
                    input_id,
                    join_node_id,
                    HydroEdgeType::Stream,
                    build_edge_label_single_input(Some("input"), input, metadata, 0, config),
                );
                structure.add_edge(
                    watermark_id,
                    join_node_id,
                    HydroEdgeType::Stream,
                    build_edge_label_single_input(Some("watermark"), watermark, metadata, 1, config),
                );

                let node_id = structure.add_node(
                    NodeLabel::with_exprs(
                        extract_op_name(self.print_root()).to_string(),
                        vec![f.clone()],
                    ),
                    HydroNodeType::Aggregation,
                    location_id,
                );
                let edge_label = if config.show_metadata {
                    type_label_from_metadata(metadata)
                } else {
                    None
                };
                structure.add_edge(join_node_id, node_id, HydroEdgeType::Stream, edge_label);
                node_id
            }

            HydroNode::Network {
                serialize_fn,
                deserialize_fn,
                input,
                metadata,
                ..
            } => {
                let input_id = input.build_graph_structure(structure, seen_tees, config);
                let _from_location_id = setup_location(structure, metadata);

                let to_location_id = match metadata.location_kind.root() {
                    LocationId::Process(id) => {
                        structure.add_location(*id, "Process".to_string());
                        Some(*id)
                    }
                    LocationId::Cluster(id) => {
                        structure.add_location(*id, "Cluster".to_string());
                        Some(*id)
                    }
                    _ => None,
                };

                let mut label = "network(".to_string();
                if serialize_fn.is_some() {
                    label.push_str("ser");
                }
                if deserialize_fn.is_some() {
                    if serialize_fn.is_some() {
                        label.push_str(" + ");
                    }
                    label.push_str("deser");
                }
                label.push(')');

                let network_id = structure.add_node(
                    NodeLabel::Static(label),
                    HydroNodeType::Network,
                    to_location_id,
                );
                let mut net_edge_label = Some(format!("to {:?}", to_location_id));
                if config.show_metadata {
                    if let Some(t) = type_label_from_metadata(input.metadata()) {
                        net_edge_label = Some(match net_edge_label.take() {
                            Some(base) => format!("{}\n{}", base, t),
                            None => t,
                        });
                    }
                }
                structure.add_edge(input_id, network_id, HydroEdgeType::Network, net_edge_label);
                network_id
            }

            // Handle remaining node types
            HydroNode::Unpersist { inner, .. } => {
                // Unpersist is typically optimized away, just pass through
                inner.build_graph_structure(structure, seen_tees, config)
            }

            HydroNode::Chain {
                first,
                second,
                metadata,
            } => {
                let first_id = first.build_graph_structure(structure, seen_tees, config);
                let second_id = second.build_graph_structure(structure, seen_tees, config);
                let location_id = setup_location(structure, metadata);
                let chain_id = structure.add_node(
                    NodeLabel::Static(extract_op_name(self.print_root())),
                    HydroNodeType::Transform,
                    location_id,
                );
                structure.add_edge(
                    first_id,
                    chain_id,
                    HydroEdgeType::Stream,
                    build_edge_label_single_input(Some("first"), first, metadata, 0, config),
                );
                structure.add_edge(
                    second_id,
                    chain_id,
                    HydroEdgeType::Stream,
                    build_edge_label_single_input(Some("second"), second, metadata, 1, config),
                );
                chain_id
            }

            HydroNode::Counter {
                tag: _,
                duration,
                input,
                metadata,
            } => build_single_expr_transform(
                TransformParams {
                    structure,
                    seen_tees,
                    config,
                    input,
                    metadata,
                    op_name: extract_op_name(self.print_root()),
                    node_type: HydroNodeType::Transform,
                    edge_type: HydroEdgeType::Stream,
                },
                duration,
            ),
        }
    }
}

/// Utility functions for rendering multiple leaves as a single graph.
/// Macro to reduce duplication in render functions.
macro_rules! render_hydro_ir {
    ($name:ident, $write_fn:ident) => {
        pub fn $name(leaves: &[HydroLeaf], config: &HydroWriteConfig) -> String {
            let mut output = String::new();
            $write_fn(&mut output, leaves, config).unwrap();
            output
        }
    };
}

/// Macro to reduce duplication in write functions.
macro_rules! write_hydro_ir {
    ($name:ident, $writer_type:ty, $constructor:expr) => {
        pub fn $name(
            output: impl std::fmt::Write,
            leaves: &[HydroLeaf],
            config: &HydroWriteConfig,
        ) -> std::fmt::Result {
            let mut graph_write: $writer_type = $constructor(output, config);
            write_hydro_ir_graph(&mut graph_write, leaves, config)
        }
    };
}

render_hydro_ir!(render_hydro_ir_mermaid, write_hydro_ir_mermaid);
write_hydro_ir!(
    write_hydro_ir_mermaid,
    HydroMermaid<_>,
    HydroMermaid::new_with_config
);

render_hydro_ir!(render_hydro_ir_dot, write_hydro_ir_dot);
write_hydro_ir!(write_hydro_ir_dot, HydroDot<_>, HydroDot::new_with_config);

render_hydro_ir!(render_hydro_ir_reactflow, write_hydro_ir_reactflow);
write_hydro_ir!(
    write_hydro_ir_reactflow,
    HydroReactFlow<_>,
    HydroReactFlow::new
);

fn write_hydro_ir_graph<W>(
    mut graph_write: W,
    leaves: &[HydroLeaf],
    config: &HydroWriteConfig,
) -> Result<(), W::Err>
where
    W: HydroGraphWrite,
{
    let mut structure = HydroGraphStructure::new();
    let mut seen_tees = HashMap::new();

    // Build the graph structure for all leaves
    for leaf in leaves {
        leaf.build_graph_structure(&mut structure, &mut seen_tees, config);
    }

    // Write the graph using the same logic as individual leaves
    graph_write.write_prologue()?;

    for (&node_id, (label, node_type, location)) in &structure.nodes {
        let (location_id, location_type) = if let Some(loc_id) = location {
            (
                Some(*loc_id),
                structure.locations.get(loc_id).map(|s| s.as_str()),
            )
        } else {
            (None, None)
        };
        graph_write.write_node_definition(
            node_id,
            label,
            *node_type,
            location_id,
            location_type,
        )?;
    }

    if config.show_location_groups {
        let mut nodes_by_location: HashMap<usize, Vec<usize>> = HashMap::new();
        for (&node_id, (_, _, location)) in &structure.nodes {
            if let Some(location_id) = location {
                nodes_by_location
                    .entry(*location_id)
                    .or_default()
                    .push(node_id);
            }
        }

        for (&location_id, node_ids) in &nodes_by_location {
            if let Some(location_type) = structure.locations.get(&location_id) {
                graph_write.write_location_start(location_id, location_type)?;
                for &node_id in node_ids {
                    graph_write.write_node(node_id)?;
                }
                graph_write.write_location_end()?;
            }
        }
    }

    for (src_id, dst_id, edge_type, label) in &structure.edges {
        graph_write.write_edge(*src_id, *dst_id, *edge_type, label.as_deref())?;
    }

    graph_write.write_epilogue()?;
    Ok(())
}
