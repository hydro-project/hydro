use std::collections::HashMap;
use std::error::Error;

use auto_impl::auto_impl;

pub use super::graphviz::{HydroDot, escape_dot};
// Re-export specific implementations
pub use super::mermaid::{HydroMermaid, escape_mermaid};
pub use super::reactflow::HydroReactFlow;
use crate::ir::{HydroLeaf, HydroNode, HydroSource};

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
}

impl Default for HydroWriteConfig {
    fn default() -> Self {
        Self {
            show_metadata: false,
            show_location_groups: true,
            include_tee_ids: true,
        }
    }
}

/// Graph structure tracker for Hydro IR rendering.
#[derive(Debug)]
pub struct HydroGraphStructure {
    pub nodes: HashMap<usize, (String, HydroNodeType, Option<usize>)>, /* node_id -> (label, type, location) */
    pub edges: Vec<(usize, usize, HydroEdgeType, Option<String>)>, // (src, dst, edge_type, label)
    pub locations: HashMap<usize, String>,                         // location_id -> location_type
    pub next_node_id: usize,
}

impl HydroGraphStructure {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
            locations: HashMap::new(),
            next_node_id: 0,
        }
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

impl HydroLeaf {
    /// Generate a mermaid graph representation of this Hydro IR leaf and its subgraph.
    pub fn to_mermaid(&self, config: &HydroWriteConfig) -> String {
        let mut output = String::new();
        self.write_mermaid(&mut output, config).unwrap();
        output
    }

    /// Write mermaid representation to the given writer.
    pub fn write_mermaid(
        &self,
        output: impl std::fmt::Write,
        config: &HydroWriteConfig,
    ) -> std::fmt::Result {
        let mut graph_write = HydroMermaid::new(output);
        self.write_graph(&mut graph_write, config)
    }

    /// Generate a DOT/Graphviz graph representation of this Hydro IR leaf and its subgraph.
    pub fn to_dot(&self, config: &HydroWriteConfig) -> String {
        let mut output = String::new();
        self.write_dot(&mut output, config).unwrap();
        output
    }

    /// Write DOT representation to the given writer.
    pub fn write_dot(
        &self,
        output: impl std::fmt::Write,
        config: &HydroWriteConfig,
    ) -> std::fmt::Result {
        let mut graph_write = HydroDot::new(output);
        self.write_graph(&mut graph_write, config)
    }

    /// Generate a ReactFlow.js JSON representation of this Hydro IR leaf and its subgraph.
    pub fn to_reactflow(&self, config: &HydroWriteConfig) -> String {
        let mut output = String::new();
        self.write_reactflow(&mut output, config).unwrap();
        output
    }

    /// Write ReactFlow.js JSON representation to the given writer.
    pub fn write_reactflow(
        &self,
        output: impl std::fmt::Write,
        config: &HydroWriteConfig,
    ) -> std::fmt::Result {
        let mut graph_write = HydroReactFlow::new(output);
        self.write_graph(&mut graph_write, config)
    }

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
        use crate::location::LocationId;

        // Helper function to extract location without capturing structure
        fn extract_location_id(
            metadata: &crate::ir::HydroIrMetadata,
        ) -> (Option<usize>, Option<String>) {
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

        match self {
            HydroLeaf::ForEach { f, input, metadata } => {
                let input_id = input.build_graph_structure(structure, seen_tees, config);
                let (location_id, location_type) = extract_location_id(metadata);
                if let (Some(loc_id), Some(loc_type)) = (location_id, location_type) {
                    structure.add_location(loc_id, loc_type);
                }
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
                let (location_id, location_type) = extract_location_id(metadata);
                if let (Some(loc_id), Some(loc_type)) = (location_id, location_type) {
                    structure.add_location(loc_id, loc_type);
                }
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
                let (location_id, location_type) = extract_location_id(metadata);
                if let (Some(loc_id), Some(loc_type)) = (location_id, location_type) {
                    structure.add_location(loc_id, loc_type);
                }
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

        // Helper function to extract location without capturing structure
        fn extract_location_id(
            metadata: &crate::ir::HydroIrMetadata,
        ) -> (Option<usize>, Option<String>) {
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

        // Helper function to handle common location setup
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

        // Helper function for single-input transform nodes
        fn build_single_input_transform(
            structure: &mut HydroGraphStructure,
            seen_tees: &mut HashMap<*const std::cell::RefCell<HydroNode>, usize>,
            config: &HydroWriteConfig,
            input: &HydroNode,
            metadata: &crate::ir::HydroIrMetadata,
            label: String,
            node_type: HydroNodeType,
            edge_type: HydroEdgeType,
        ) -> usize {
            let input_id = input.build_graph_structure(structure, seen_tees, config);
            let location_id = setup_location(structure, metadata);
            let node_id = structure.add_node(label, node_type, location_id);
            structure.add_edge(input_id, node_id, edge_type, None);
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
                "persist()".to_string(),
                HydroNodeType::Transform,
                HydroEdgeType::Persistent,
            ),

            HydroNode::Delta { inner, metadata } => build_single_input_transform(
                structure,
                seen_tees,
                config,
                inner,
                metadata,
                "delta()".to_string(),
                HydroNodeType::Transform,
                HydroEdgeType::Stream,
            ),

            HydroNode::Map { f, input, metadata } => build_single_input_transform(
                structure,
                seen_tees,
                config,
                input,
                metadata,
                format!("map({:?})", f),
                HydroNodeType::Transform,
                HydroEdgeType::Stream,
            ),

            HydroNode::Filter { f, input, metadata } => build_single_input_transform(
                structure,
                seen_tees,
                config,
                input,
                metadata,
                format!("filter({:?})", f),
                HydroNodeType::Transform,
                HydroEdgeType::Stream,
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
                format!("fold({:?}, {:?})", init, acc),
                HydroNodeType::Aggregation,
                HydroEdgeType::Stream,
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
                format!("flat_map({:?})", f),
                HydroNodeType::Transform,
                HydroEdgeType::Stream,
            ),

            HydroNode::FilterMap { f, input, metadata } => build_single_input_transform(
                structure,
                seen_tees,
                config,
                input,
                metadata,
                format!("filter_map({:?})", f),
                HydroNodeType::Transform,
                HydroEdgeType::Stream,
            ),

            HydroNode::Unpersist { inner, .. } => {
                // Unpersist is typically optimized away, just pass through
                inner.build_graph_structure(structure, seen_tees, config)
            }

            _ => {
                // For truly unhandled cases, create a basic node
                structure.add_node(
                    format!("{:?}", self)
                        .split(' ')
                        .next()
                        .unwrap_or("unknown")
                        .to_string(),
                    HydroNodeType::Transform,
                    None,
                )
            }
        }
    }
}

/// Utility functions for rendering multiple leaves as a single graph.
pub fn render_hydro_ir_mermaid(leaves: &[HydroLeaf], config: &HydroWriteConfig) -> String {
    let mut output = String::new();
    write_hydro_ir_mermaid(&mut output, leaves, config).unwrap();
    output
}

pub fn write_hydro_ir_mermaid(
    output: impl std::fmt::Write,
    leaves: &[HydroLeaf],
    config: &HydroWriteConfig,
) -> std::fmt::Result {
    let mut graph_write = HydroMermaid::new(output);
    write_hydro_ir_graph(&mut graph_write, leaves, config)
}

pub fn render_hydro_ir_dot(leaves: &[HydroLeaf], config: &HydroWriteConfig) -> String {
    let mut output = String::new();
    write_hydro_ir_dot(&mut output, leaves, config).unwrap();
    output
}

pub fn write_hydro_ir_dot(
    output: impl std::fmt::Write,
    leaves: &[HydroLeaf],
    config: &HydroWriteConfig,
) -> std::fmt::Result {
    let mut graph_write = HydroDot::new(output);
    write_hydro_ir_graph(&mut graph_write, leaves, config)
}

pub fn render_hydro_ir_reactflow(leaves: &[HydroLeaf], config: &HydroWriteConfig) -> String {
    let mut output = String::new();
    write_hydro_ir_reactflow(&mut output, leaves, config).unwrap();
    output
}

pub fn write_hydro_ir_reactflow(
    output: impl std::fmt::Write,
    leaves: &[HydroLeaf],
    config: &HydroWriteConfig,
) -> std::fmt::Result {
    let mut graph_write = HydroReactFlow::new(output);
    write_hydro_ir_graph(&mut graph_write, leaves, config)
}

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
