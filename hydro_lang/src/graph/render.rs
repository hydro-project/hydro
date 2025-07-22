use std::collections::HashMap;
use std::error::Error;
use std::fmt::Write;

use auto_impl::auto_impl;

pub use super::graphviz::{HydroDot, escape_dot};
// Re-export specific implementations
pub use super::mermaid::{HydroMermaid, escape_mermaid};
pub use super::reactflow::HydroReactFlow;
use crate::ir::{HydroLeaf, HydroNode, HydroSource};

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
        node_label: &str,
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
    pub include_tee_ids: bool,
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
            include_tee_ids: true,
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
    pub nodes: HashMap<usize, (String, HydroNodeType, Option<usize>)>, /* node_id -> (label, type, location) */
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
        label: String,
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

/// Extract a short, readable label from the full token stream label
pub fn extract_short_label(full_label: &str) -> String {
    // Look for common patterns and extract just the operation name
    if let Some(op_name) = full_label.split('(').next() {
        match op_name.to_lowercase().as_str() {
            "map" => "map".to_string(),
            "filter" => "filter".to_string(),
            "flat_map" => "flat_map".to_string(),
            "filter_map" => "filter_map".to_string(),
            "for_each" => "for_each".to_string(),
            "fold" => "fold".to_string(),
            "reduce" => "reduce".to_string(),
            "join" => "join".to_string(),
            "persist" => "persist".to_string(),
            "delta" => "delta".to_string(),
            "tee" => "tee".to_string(),
            "source_iter" => "source_iter".to_string(),
            "dest_sink" => "dest_sink".to_string(),
            "cycle_sink" => "cycle_sink".to_string(),
            "external_network" => "network".to_string(),
            "spin" => "spin".to_string(),
            "inspect" => "inspect".to_string(),
            _ if full_label.contains("network") => {
                if full_label.contains("deser") {
                    "network(recv)".to_string()
                } else if full_label.contains("ser") {
                    "network(send)".to_string()
                } else {
                    "network".to_string()
                }
            }
            _ if full_label.contains("send_bincode") => "send_bincode".to_string(),
            _ if full_label.contains("broadcast_bincode") => "broadcast_bincode".to_string(),
            _ if full_label.contains("dest_sink") => "dest_sink".to_string(),
            _ if full_label.contains("source_stream") => "source_stream".to_string(),
            _ => {
                // For other cases, try to get a reasonable short name
                if full_label.len() > 20 {
                    format!("{}...", &full_label[..17])
                } else {
                    full_label.to_string()
                }
            }
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
        match self {
            HydroLeaf::ForEach { f, input, metadata } => {
                let input_id = input.build_graph_structure(structure, seen_tees, config);
                let location_id = setup_location(structure, metadata);
                let sink_id = structure.add_node(
                    format!("for_each({:?})", f),
                    HydroNodeType::Sink,
                    location_id,
                );
                structure.add_edge(input_id, sink_id, HydroEdgeType::Stream, None);
                sink_id
            }
            HydroLeaf::DestSink {
                sink,
                input,
                metadata,
            } => {
                let input_id = input.build_graph_structure(structure, seen_tees, config);
                let location_id = setup_location(structure, metadata);
                let sink_id = structure.add_node(
                    format!("dest_sink({:?})", sink),
                    HydroNodeType::Sink,
                    location_id,
                );
                structure.add_edge(input_id, sink_id, HydroEdgeType::Stream, None);
                sink_id
            }
            HydroLeaf::CycleSink {
                ident,
                input,
                metadata,
                ..
            } => {
                let input_id = input.build_graph_structure(structure, seen_tees, config);
                let location_id = setup_location(structure, metadata);
                let sink_id = structure.add_node(
                    format!("cycle_sink({})", ident),
                    HydroNodeType::Sink,
                    location_id,
                );
                structure.add_edge(input_id, sink_id, HydroEdgeType::Cycle, None);
                sink_id
            }
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

        // Helper struct to group node creation parameters
        struct NodeParams {
            label: String,
            node_type: HydroNodeType,
            edge_type: HydroEdgeType,
        }

        // Helper function for single-input transform nodes
        fn build_single_input_transform(
            structure: &mut HydroGraphStructure,
            seen_tees: &mut HashMap<*const std::cell::RefCell<HydroNode>, usize>,
            config: &HydroWriteConfig,
            input: &HydroNode,
            metadata: &crate::ir::HydroIrMetadata,
            params: NodeParams,
        ) -> usize {
            let input_id = input.build_graph_structure(structure, seen_tees, config);
            let location_id = setup_location(structure, metadata);
            let node_id = structure.add_node(params.label, params.node_type, location_id);
            structure.add_edge(input_id, node_id, params.edge_type, None);
            node_id
        }

        // Helper function for source nodes
        fn build_source_node(
            structure: &mut HydroGraphStructure,
            metadata: &crate::ir::HydroIrMetadata,
            label: String,
        ) -> usize {
            let location_id = setup_location(structure, metadata);
            structure.add_node(label, HydroNodeType::Source, location_id)
        }

        match self {
            HydroNode::Placeholder => {
                structure.add_node("PLACEHOLDER".to_string(), HydroNodeType::Transform, None)
            }

            HydroNode::Source {
                source, metadata, ..
            } => {
                let label = match source {
                    HydroSource::Stream(expr) => format!("source_stream({:?})", expr),
                    HydroSource::ExternalNetwork() => "external_network()".to_string(),
                    HydroSource::Iter(expr) => format!("source_iter({:?})", expr),
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

                let tee_id = if config.include_tee_ids {
                    structure.add_node("tee()".to_string(), HydroNodeType::Tee, location_id)
                } else {
                    input_id // If not showing tee nodes, just return the input
                };

                seen_tees.insert(ptr, tee_id);

                if config.include_tee_ids {
                    structure.add_edge(input_id, tee_id, HydroEdgeType::Stream, None);
                }
                tee_id
            }

            HydroNode::Persist { inner, metadata } => build_single_input_transform(
                structure,
                seen_tees,
                config,
                inner,
                metadata,
                NodeParams {
                    label: "persist()".to_string(),
                    node_type: HydroNodeType::Transform,
                    edge_type: HydroEdgeType::Persistent,
                },
            ),

            HydroNode::Delta { inner, metadata } => build_single_input_transform(
                structure,
                seen_tees,
                config,
                inner,
                metadata,
                NodeParams {
                    label: "delta()".to_string(),
                    node_type: HydroNodeType::Transform,
                    edge_type: HydroEdgeType::Stream,
                },
            ),

            HydroNode::Map { f, input, metadata } => build_single_input_transform(
                structure,
                seen_tees,
                config,
                input,
                metadata,
                NodeParams {
                    label: format!("map({:?})", f),
                    node_type: HydroNodeType::Transform,
                    edge_type: HydroEdgeType::Stream,
                },
            ),

            HydroNode::Filter { f, input, metadata } => build_single_input_transform(
                structure,
                seen_tees,
                config,
                input,
                metadata,
                NodeParams {
                    label: format!("filter({:?})", f),
                    node_type: HydroNodeType::Transform,
                    edge_type: HydroEdgeType::Stream,
                },
            ),

            HydroNode::Join {
                left,
                right,
                metadata,
            } => {
                let left_id = left.build_graph_structure(structure, seen_tees, config);
                let right_id = right.build_graph_structure(structure, seen_tees, config);
                let location_id = setup_location(structure, metadata);
                let join_id =
                    structure.add_node("join()".to_string(), HydroNodeType::Join, location_id);
                structure.add_edge(
                    left_id,
                    join_id,
                    HydroEdgeType::Stream,
                    Some("left".to_string()),
                );
                structure.add_edge(
                    right_id,
                    join_id,
                    HydroEdgeType::Stream,
                    Some("right".to_string()),
                );
                join_id
            }

            HydroNode::Fold {
                init,
                acc,
                input,
                metadata,
            } => build_single_input_transform(
                structure,
                seen_tees,
                config,
                input,
                metadata,
                NodeParams {
                    label: format!("fold({:?}, {:?})", init, acc),
                    node_type: HydroNodeType::Aggregation,
                    edge_type: HydroEdgeType::Stream,
                },
            ),

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

                let network_id = structure.add_node(label, HydroNodeType::Network, to_location_id);
                structure.add_edge(
                    input_id,
                    network_id,
                    HydroEdgeType::Network,
                    Some(format!("to {:?}", to_location_id)),
                );
                network_id
            }

            // Handle remaining node types
            HydroNode::FlatMap { f, input, metadata } => build_single_input_transform(
                structure,
                seen_tees,
                config,
                input,
                metadata,
                NodeParams {
                    label: format!("flat_map({:?})", f),
                    node_type: HydroNodeType::Transform,
                    edge_type: HydroEdgeType::Stream,
                },
            ),

            HydroNode::FilterMap { f, input, metadata } => build_single_input_transform(
                structure,
                seen_tees,
                config,
                input,
                metadata,
                NodeParams {
                    label: format!("filter_map({:?})", f),
                    node_type: HydroNodeType::Transform,
                    edge_type: HydroEdgeType::Stream,
                },
            ),

            HydroNode::Unpersist { inner, .. } => {
                // Unpersist is typically optimized away, just pass through
                inner.build_graph_structure(structure, seen_tees, config)
            }

            HydroNode::Inspect { f, input, metadata } => build_single_input_transform(
                structure,
                seen_tees,
                config,
                input,
                metadata,
                NodeParams {
                    label: format!("inspect({:?})", f),
                    node_type: HydroNodeType::Transform,
                    edge_type: HydroEdgeType::Stream,
                },
            ),

            HydroNode::Chain {
                first,
                second,
                metadata,
            } => {
                let first_id = first.build_graph_structure(structure, seen_tees, config);
                let second_id = second.build_graph_structure(structure, seen_tees, config);
                let location_id = setup_location(structure, metadata);
                let chain_id = structure.add_node(
                    "chain()".to_string(),
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

            HydroNode::CrossProduct {
                left,
                right,
                metadata,
            } => {
                let left_id = left.build_graph_structure(structure, seen_tees, config);
                let right_id = right.build_graph_structure(structure, seen_tees, config);
                let location_id = setup_location(structure, metadata);
                let cross_id = structure.add_node(
                    "cross_product()".to_string(),
                    HydroNodeType::Join,
                    location_id,
                );
                structure.add_edge(
                    left_id,
                    cross_id,
                    HydroEdgeType::Stream,
                    Some("left".to_string()),
                );
                structure.add_edge(
                    right_id,
                    cross_id,
                    HydroEdgeType::Stream,
                    Some("right".to_string()),
                );
                cross_id
            }

            HydroNode::CrossSingleton {
                left,
                right,
                metadata,
            } => {
                let left_id = left.build_graph_structure(structure, seen_tees, config);
                let right_id = right.build_graph_structure(structure, seen_tees, config);
                let location_id = setup_location(structure, metadata);
                let cross_singleton_id = structure.add_node(
                    "cross_singleton()".to_string(),
                    HydroNodeType::Join,
                    location_id,
                );
                structure.add_edge(
                    left_id,
                    cross_singleton_id,
                    HydroEdgeType::Stream,
                    Some("left".to_string()),
                );
                structure.add_edge(
                    right_id,
                    cross_singleton_id,
                    HydroEdgeType::Stream,
                    Some("right".to_string()),
                );
                cross_singleton_id
            }

            HydroNode::Difference { pos, neg, metadata } => {
                let pos_id = pos.build_graph_structure(structure, seen_tees, config);
                let neg_id = neg.build_graph_structure(structure, seen_tees, config);
                let location_id = setup_location(structure, metadata);
                let diff_id = structure.add_node(
                    "difference()".to_string(),
                    HydroNodeType::Join,
                    location_id,
                );
                structure.add_edge(
                    pos_id,
                    diff_id,
                    HydroEdgeType::Stream,
                    Some("pos".to_string()),
                );
                structure.add_edge(
                    neg_id,
                    diff_id,
                    HydroEdgeType::Stream,
                    Some("neg".to_string()),
                );
                diff_id
            }

            HydroNode::AntiJoin { pos, neg, metadata } => {
                let pos_id = pos.build_graph_structure(structure, seen_tees, config);
                let neg_id = neg.build_graph_structure(structure, seen_tees, config);
                let location_id = setup_location(structure, metadata);
                let anti_join_id =
                    structure.add_node("anti_join()".to_string(), HydroNodeType::Join, location_id);
                structure.add_edge(
                    pos_id,
                    anti_join_id,
                    HydroEdgeType::Stream,
                    Some("pos".to_string()),
                );
                structure.add_edge(
                    neg_id,
                    anti_join_id,
                    HydroEdgeType::Stream,
                    Some("neg".to_string()),
                );
                anti_join_id
            }

            HydroNode::DeferTick { input, metadata } => build_single_input_transform(
                structure,
                seen_tees,
                config,
                input,
                metadata,
                NodeParams {
                    label: "defer_tick()".to_string(),
                    node_type: HydroNodeType::Transform,
                    edge_type: HydroEdgeType::Stream,
                },
            ),

            HydroNode::Enumerate {
                is_static,
                input,
                metadata,
            } => build_single_input_transform(
                structure,
                seen_tees,
                config,
                input,
                metadata,
                NodeParams {
                    label: format!("enumerate(static={})", is_static),
                    node_type: HydroNodeType::Transform,
                    edge_type: HydroEdgeType::Stream,
                },
            ),

            HydroNode::Unique { input, metadata } => build_single_input_transform(
                structure,
                seen_tees,
                config,
                input,
                metadata,
                NodeParams {
                    label: "unique()".to_string(),
                    node_type: HydroNodeType::Transform,
                    edge_type: HydroEdgeType::Stream,
                },
            ),

            HydroNode::Sort { input, metadata } => build_single_input_transform(
                structure,
                seen_tees,
                config,
                input,
                metadata,
                NodeParams {
                    label: "sort()".to_string(),
                    node_type: HydroNodeType::Aggregation,
                    edge_type: HydroEdgeType::Stream,
                },
            ),

            HydroNode::FoldKeyed {
                init,
                acc,
                input,
                metadata,
            } => build_single_input_transform(
                structure,
                seen_tees,
                config,
                input,
                metadata,
                NodeParams {
                    label: format!("fold_keyed({:?}, {:?})", init, acc),
                    node_type: HydroNodeType::Aggregation,
                    edge_type: HydroEdgeType::Stream,
                },
            ),

            HydroNode::Reduce { f, input, metadata } => build_single_input_transform(
                structure,
                seen_tees,
                config,
                input,
                metadata,
                NodeParams {
                    label: format!("reduce({:?})", f),
                    node_type: HydroNodeType::Aggregation,
                    edge_type: HydroEdgeType::Stream,
                },
            ),

            HydroNode::ReduceKeyed { f, input, metadata } => build_single_input_transform(
                structure,
                seen_tees,
                config,
                input,
                metadata,
                NodeParams {
                    label: format!("reduce_keyed({:?})", f),
                    node_type: HydroNodeType::Aggregation,
                    edge_type: HydroEdgeType::Stream,
                },
            ),

            HydroNode::ResolveFutures { input, metadata } => build_single_input_transform(
                structure,
                seen_tees,
                config,
                input,
                metadata,
                NodeParams {
                    label: "resolve_futures()".to_string(),
                    node_type: HydroNodeType::Transform,
                    edge_type: HydroEdgeType::Stream,
                },
            ),

            HydroNode::ResolveFuturesOrdered { input, metadata } => build_single_input_transform(
                structure,
                seen_tees,
                config,
                input,
                metadata,
                NodeParams {
                    label: "resolve_futures_ordered()".to_string(),
                    node_type: HydroNodeType::Transform,
                    edge_type: HydroEdgeType::Stream,
                },
            ),

            HydroNode::Counter {
                tag,
                duration,
                input,
                metadata,
            } => build_single_input_transform(
                structure,
                seen_tees,
                config,
                input,
                metadata,
                NodeParams {
                    label: format!("counter({}, {:?})", tag, duration),
                    node_type: HydroNodeType::Transform,
                    edge_type: HydroEdgeType::Stream,
                },
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
