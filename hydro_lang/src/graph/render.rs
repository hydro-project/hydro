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
    pub fn static_label(s: String) -> Self {
        Self::Static(s)
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

/// Helper function to extract location ID and type from metadata.
fn extract_location_id(metadata: &crate::ir::HydroIrMetadata) -> (Option<usize>, Option<String>) {
    use crate::location::LocationId;
    match &metadata.location_kind {
        LocationId::Process(id) => (Some(*id), Some("Process".to_string())),
        LocationId::Cluster(id) => (Some(*id), Some("Cluster".to_string())),
        LocationId::ExternalProcess(id) => (Some(*id), Some("External".to_string())),
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
            metadata: &crate::ir::HydroIrMetadata,
            label: NodeLabel,
            edge_type: HydroEdgeType,
        ) -> usize {
            let input_id = input.build_graph_structure(structure, seen_tees, config);
            let location_id = setup_location(structure, metadata);
            let sink_id = structure.add_node(label, HydroNodeType::Sink, location_id);
            structure.add_edge(input_id, sink_id, edge_type, None);
            sink_id
        }

        match self {
            // Sink operations with Stream edges - grouped by edge type
            HydroLeaf::ForEach { f, input, metadata } => build_sink_node(
                structure,
                seen_tees,
                config,
                input,
                metadata,
                NodeLabel::with_exprs("for_each".to_string(), vec![f.clone()]),
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
                metadata,
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
                metadata,
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
            params
                .structure
                .add_edge(input_id, node_id, params.edge_type, None);
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
            params
                .structure
                .add_edge(input_id, node_id, params.edge_type, None);
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
            params
                .structure
                .add_edge(input_id, node_id, params.edge_type, None);
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

                structure.add_edge(input_id, tee_id, HydroEdgeType::Stream, None);

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
                    Some("left".to_string()),
                );
                structure.add_edge(
                    right_id,
                    node_id,
                    HydroEdgeType::Stream,
                    Some("right".to_string()),
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
                    Some("pos".to_string()),
                );
                structure.add_edge(
                    right_id,
                    node_id,
                    HydroEdgeType::Stream,
                    Some("neg".to_string()),
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

            HydroNode::Network {
                to_location,
                serialize_fn,
                deserialize_fn,
                input,
                metadata,
                ..
            } => {
                let input_id = input.build_graph_structure(structure, seen_tees, config);
                let _from_location_id = setup_location(structure, metadata);

                let to_location_id = match to_location {
                    LocationId::Process(id) => {
                        structure.add_location(*id, "Process".to_string());
                        Some(*id)
                    }
                    LocationId::Cluster(id) => {
                        structure.add_location(*id, "Cluster".to_string());
                        Some(*id)
                    }
                    LocationId::ExternalProcess(id) => {
                        structure.add_location(*id, "External".to_string());
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
                structure.add_edge(
                    input_id,
                    network_id,
                    HydroEdgeType::Network,
                    Some(format!("to {:?}", to_location_id)),
                );
                network_id
            }

            // Handle remaining node types
            HydroNode::Unpersist { .. } => {
                // Unpersist is typically optimized away, just pass through
                panic!(
                    "Unpersist should not be rendered in graph structure: {:?}",
                    self
                );
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
                    Some("first".to_string()),
                );
                structure.add_edge(
                    second_id,
                    chain_id,
                    HydroEdgeType::Stream,
                    Some("second".to_string()),
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
