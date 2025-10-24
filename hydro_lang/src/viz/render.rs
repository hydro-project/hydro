use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt::Write;

use auto_impl::auto_impl;

pub use super::graphviz::{HydroDot, escape_dot};
pub use super::json::HydroJson;
// Re-export specific implementations
pub use super::mermaid::{HydroMermaid, escape_mermaid};
use crate::compile::ir::backtrace::Backtrace;
use crate::compile::ir::{DebugExpr, HydroIrMetadata, HydroNode, HydroRoot, HydroSource};
use crate::location::dynamic::LocationId;

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
        backtrace: Option<&Backtrace>,
    ) -> Result<(), Self::Err>;

    /// Write an edge between nodes with optional labeling.
    fn write_edge(
        &mut self,
        src_id: usize,
        dst_id: usize,
        edge_properties: &HashSet<HydroEdgeProp>,
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

/// Node type utilities - centralized handling of HydroNodeType operations
pub mod node_type_utils {
    use super::HydroNodeType;

    /// All node types with their string names
    const NODE_TYPE_DATA: &[(HydroNodeType, &str)] = &[
        (HydroNodeType::Source, "Source"),
        (HydroNodeType::Transform, "Transform"),
        (HydroNodeType::Join, "Join"),
        (HydroNodeType::Aggregation, "Aggregation"),
        (HydroNodeType::Network, "Network"),
        (HydroNodeType::Sink, "Sink"),
        (HydroNodeType::Tee, "Tee"),
    ];

    /// Convert HydroNodeType to string representation (used by JSON format)
    pub fn to_string(node_type: HydroNodeType) -> &'static str {
        NODE_TYPE_DATA
            .iter()
            .find(|(nt, _)| *nt == node_type)
            .map(|(_, name)| *name)
            .unwrap_or("Unknown")
    }

    /// Get all node types with their string representations (used by JSON format)
    pub fn all_types_with_strings() -> Vec<(HydroNodeType, &'static str)> {
        NODE_TYPE_DATA.to_vec()
    }
}

/// Types of nodes in Hydro IR for styling purposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HydroNodeType {
    Source,
    Transform,
    Join,
    Aggregation,
    Network,
    Sink,
    Tee,
}

/// Types of edges in Hydro IR representing stream properties.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HydroEdgeProp {
    Bounded,
    Unbounded,
    TotalOrder,
    NoOrder,
    Keyed,
    // Collection type tags for styling
    Stream,
    KeyedSingleton,
    KeyedStream,
    Singleton,
    Optional,
    Network,
    Cycle,
}

/// Unified edge style representation for all graph formats.
/// This intermediate format allows consistent styling across JSON, DOT, and Mermaid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnifiedEdgeStyle {
    /// Line pattern (solid, dashed)
    pub line_pattern: LinePattern,
    /// Line width (1 = thin, 3 = thick)
    pub line_width: u8,
    /// Arrowhead style
    pub arrowhead: ArrowheadStyle,
    /// Line style (single plain line, or line with hash marks/dots for keyed streams)
    pub line_style: LineStyle,
    /// Halo/background effect for boundedness
    pub halo: HaloStyle,
    /// Line waviness for ordering information
    pub waviness: WavinessStyle,
    /// Whether animation is enabled (JSON only)
    pub animation: AnimationStyle,
    /// Color for the edge
    pub color: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinePattern {
    Solid,
    Dotted,
    Dashed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrowheadStyle {
    TriangleFilled,
    CircleFilled,
    DiamondOpen,
    Default,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineStyle {
    /// Plain single line
    Single,
    /// Single line with hash marks/dots (for keyed streams)
    HashMarks,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HaloStyle {
    None,
    LightBlue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WavinessStyle {
    None,
    Wavy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationStyle {
    Static,
    Animated,
}

impl Default for UnifiedEdgeStyle {
    fn default() -> Self {
        Self {
            line_pattern: LinePattern::Solid,
            line_width: 1,
            arrowhead: ArrowheadStyle::Default,
            line_style: LineStyle::Single,
            halo: HaloStyle::None,
            waviness: WavinessStyle::None,
            animation: AnimationStyle::Static,
            color: "#666666",
        }
    }
}

/// Convert HydroEdgeType properties to unified edge style.
/// This is the core logic for determining edge visual properties.
///
/// # Visual Encoding Mapping
///
/// | Semantic Property | Visual Channel | Values |
/// |------------------|----------------|---------|
/// | Network | Line Pattern + Animation | Local (solid, static), Network (dashed, animated) |
/// | Ordering | Waviness | TotalOrder (straight), NoOrder (wavy) |
/// | Boundedness | Halo | Bounded (none), Unbounded (light-blue transparent) |
/// | Keyedness | Line Style | NotKeyed (plain line), Keyed (line with hash marks/dots) |
/// | Collection Type | Color + Arrowhead | Stream (blue #2563eb, triangle), Singleton (black, circle), Optional (gray, diamond) |
pub fn get_unified_edge_style(
    edge_properties: &HashSet<HydroEdgeProp>,
    src_location: Option<usize>,
    dst_location: Option<usize>,
) -> UnifiedEdgeStyle {
    let mut style = UnifiedEdgeStyle::default();

    // Network communication group - controls line pattern AND animation
    let is_network = edge_properties.contains(&HydroEdgeProp::Network)
        || (src_location.is_some() && dst_location.is_some() && src_location != dst_location);

    if is_network {
        style.line_pattern = LinePattern::Dashed;
        style.animation = AnimationStyle::Animated;
    } else {
        style.line_pattern = LinePattern::Solid;
        style.animation = AnimationStyle::Static;
    }

    // Boundedness group - controls halo
    if edge_properties.contains(&HydroEdgeProp::Unbounded) {
        style.halo = HaloStyle::LightBlue;
    } else {
        style.halo = HaloStyle::None;
    }

    // Collection type group - controls arrowhead and color
    if edge_properties.contains(&HydroEdgeProp::Stream) {
        style.arrowhead = ArrowheadStyle::TriangleFilled;
        style.color = "#2563eb"; // Bright blue for Stream
    } else if edge_properties.contains(&HydroEdgeProp::KeyedStream) {
        style.arrowhead = ArrowheadStyle::TriangleFilled;
        style.color = "#2563eb"; // Bright blue for Stream (keyed variant)
    } else if edge_properties.contains(&HydroEdgeProp::KeyedSingleton) {
        style.arrowhead = ArrowheadStyle::TriangleFilled;
        style.color = "#000000"; // Black for Singleton (keyed variant)
    } else if edge_properties.contains(&HydroEdgeProp::Singleton) {
        style.arrowhead = ArrowheadStyle::CircleFilled;
        style.color = "#000000"; // Black for Singleton
    } else if edge_properties.contains(&HydroEdgeProp::Optional) {
        style.arrowhead = ArrowheadStyle::DiamondOpen;
        style.color = "#6b7280"; // Gray for Optional
    }

    // Keyedness group - controls hash marks on the line
    if edge_properties.contains(&HydroEdgeProp::Keyed) {
        style.line_style = LineStyle::HashMarks; // Renders as hash marks/dots on the line in hydroscope
    } else {
        style.line_style = LineStyle::Single;
    }

    // Ordering group - waviness channel
    if edge_properties.contains(&HydroEdgeProp::NoOrder) {
        style.waviness = WavinessStyle::Wavy;
    } else if edge_properties.contains(&HydroEdgeProp::TotalOrder) {
        style.waviness = WavinessStyle::None;
    }

    style
}

/// Extract semantic edge properties from CollectionKind metadata.
/// This function analyzes the collection type and extracts relevant semantic tags
/// for visualization purposes.
pub fn extract_edge_properties_from_collection_kind(
    collection_kind: &crate::compile::ir::CollectionKind,
) -> HashSet<HydroEdgeProp> {
    use crate::compile::ir::CollectionKind;

    let mut properties = HashSet::new();

    match collection_kind {
        CollectionKind::Stream { bound, order, .. } => {
            properties.insert(HydroEdgeProp::Stream);
            add_bound_property(&mut properties, bound);
            add_order_property(&mut properties, order);
        }
        CollectionKind::KeyedStream {
            bound, value_order, ..
        } => {
            properties.insert(HydroEdgeProp::KeyedStream);
            properties.insert(HydroEdgeProp::Keyed);
            add_bound_property(&mut properties, bound);
            add_order_property(&mut properties, value_order);
        }
        CollectionKind::Singleton { bound, .. } => {
            properties.insert(HydroEdgeProp::Singleton);
            add_bound_property(&mut properties, bound);
            // Singletons have implicit TotalOrder
            properties.insert(HydroEdgeProp::TotalOrder);
        }
        CollectionKind::Optional { bound, .. } => {
            properties.insert(HydroEdgeProp::Optional);
            add_bound_property(&mut properties, bound);
            // Optionals have implicit TotalOrder
            properties.insert(HydroEdgeProp::TotalOrder);
        }
        CollectionKind::KeyedSingleton { bound, .. } => {
            properties.insert(HydroEdgeProp::Singleton);
            properties.insert(HydroEdgeProp::Keyed);
            // KeyedSingletons boundedness depends on the bound kind
            add_keyed_singleton_bound_property(&mut properties, bound);
            properties.insert(HydroEdgeProp::TotalOrder);
        }
    }

    properties
}

/// Helper function to add bound property based on BoundKind.
fn add_bound_property(
    properties: &mut HashSet<HydroEdgeProp>,
    bound: &crate::compile::ir::BoundKind,
) {
    use crate::compile::ir::BoundKind;

    match bound {
        BoundKind::Bounded => {
            properties.insert(HydroEdgeProp::Bounded);
        }
        BoundKind::Unbounded => {
            properties.insert(HydroEdgeProp::Unbounded);
        }
    }
}

/// Helper function to add bound property for KeyedSingleton based on KeyedSingletonBoundKind.
fn add_keyed_singleton_bound_property(
    properties: &mut HashSet<HydroEdgeProp>,
    bound: &crate::compile::ir::KeyedSingletonBoundKind,
) {
    use crate::compile::ir::KeyedSingletonBoundKind;

    match bound {
        KeyedSingletonBoundKind::Bounded | KeyedSingletonBoundKind::BoundedValue => {
            properties.insert(HydroEdgeProp::Bounded);
        }
        KeyedSingletonBoundKind::Unbounded => {
            properties.insert(HydroEdgeProp::Unbounded);
        }
    }
}

/// Helper function to add order property based on StreamOrder.
fn add_order_property(
    properties: &mut HashSet<HydroEdgeProp>,
    order: &crate::compile::ir::StreamOrder,
) {
    use crate::compile::ir::StreamOrder;

    match order {
        StreamOrder::TotalOrder => {
            properties.insert(HydroEdgeProp::TotalOrder);
        }
        StreamOrder::NoOrder => {
            properties.insert(HydroEdgeProp::NoOrder);
        }
    }
}

/// Detect if an edge crosses network boundaries by comparing source and destination locations.
/// Returns true if the edge represents network communication between different locations.
pub fn is_network_edge(src_location: &LocationId, dst_location: &LocationId) -> bool {
    // Compare the root locations to determine if they differ
    src_location.root() != dst_location.root()
}

/// Add network edge tag if source and destination locations differ.
pub fn add_network_edge_tag(
    properties: &mut HashSet<HydroEdgeProp>,
    src_location: &LocationId,
    dst_location: &LocationId,
) {
    if is_network_edge(src_location, dst_location) {
        properties.insert(HydroEdgeProp::Network);
    }
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

/// Node information in the Hydro graph.
#[derive(Clone)]
pub struct HydroGraphNode {
    pub label: NodeLabel,
    pub node_type: HydroNodeType,
    pub location: Option<usize>,
    pub backtrace: Option<Backtrace>,
}

/// Edge information in the Hydro graph.
#[derive(Debug, Clone)]
pub struct HydroGraphEdge {
    pub src: usize,
    pub dst: usize,
    pub edge_properties: HashSet<HydroEdgeProp>,
    pub label: Option<String>,
}

/// Graph structure tracker for Hydro IR rendering.
#[derive(Default)]
pub struct HydroGraphStructure {
    pub nodes: HashMap<usize, HydroGraphNode>,
    pub edges: Vec<HydroGraphEdge>,
    pub locations: HashMap<usize, String>, // location_id -> location_type
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
        self.add_node_with_backtrace(label, node_type, location, None)
    }

    pub fn add_node_with_backtrace(
        &mut self,
        label: NodeLabel,
        node_type: HydroNodeType,
        location: Option<usize>,
        backtrace: Option<Backtrace>,
    ) -> usize {
        let node_id = self.next_node_id;
        self.next_node_id += 1;
        self.nodes.insert(
            node_id,
            HydroGraphNode {
                label,
                node_type,
                location,
                backtrace,
            },
        );
        node_id
    }

    /// Add a node with metadata, extracting backtrace automatically
    pub fn add_node_with_metadata(
        &mut self,
        label: NodeLabel,
        node_type: HydroNodeType,
        metadata: &HydroIrMetadata,
    ) -> usize {
        let location = setup_location(self, metadata);
        let backtrace = Some(metadata.op.backtrace.clone());
        self.add_node_with_backtrace(label, node_type, location, backtrace)
    }

    pub fn add_edge(
        &mut self,
        src: usize,
        dst: usize,
        edge_properties: HashSet<HydroEdgeProp>,
        label: Option<String>,
    ) {
        self.edges.push(HydroGraphEdge {
            src,
            dst,
            edge_properties,
            label,
        });
    }

    // Legacy method for backward compatibility
    pub fn add_edge_single(
        &mut self,
        src: usize,
        dst: usize,
        edge_type: HydroEdgeProp,
        label: Option<String>,
    ) {
        let mut properties = HashSet::new();
        properties.insert(edge_type);
        self.edges.push(HydroGraphEdge {
            src,
            dst,
            edge_properties: properties,
            label,
        });
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
fn extract_location_id(location_id: &LocationId) -> (Option<usize>, Option<String>) {
    match location_id.root() {
        LocationId::Process(id) => (Some(*id), Some("Process".to_string())),
        LocationId::Cluster(id) => (Some(*id), Some("Cluster".to_string())),
        _ => panic!("unexpected location type"),
    }
}

/// Helper function to set up location in structure from metadata.
fn setup_location(
    structure: &mut HydroGraphStructure,
    metadata: &HydroIrMetadata,
) -> Option<usize> {
    let (location_id, location_type) = extract_location_id(&metadata.location_kind);
    if let (Some(loc_id), Some(loc_type)) = (location_id, location_type) {
        structure.add_location(loc_id, loc_type);
    }
    location_id
}

/// Helper function to add an edge with semantic tags extracted from metadata.
/// This function combines collection kind extraction with network detection.
fn add_edge_with_metadata(
    structure: &mut HydroGraphStructure,
    src_id: usize,
    dst_id: usize,
    src_metadata: Option<&HydroIrMetadata>,
    dst_metadata: Option<&HydroIrMetadata>,
    label: Option<String>,
) {
    let mut properties = HashSet::new();

    // Extract semantic tags from source metadata's collection kind
    if let Some(metadata) = src_metadata {
        properties.extend(extract_edge_properties_from_collection_kind(
            &metadata.collection_kind,
        ));
    }

    // Add network edge tag if locations differ
    if let (Some(src_meta), Some(dst_meta)) = (src_metadata, dst_metadata) {
        add_network_edge_tag(
            &mut properties,
            &src_meta.location_kind,
            &dst_meta.location_kind,
        );
    }

    // If no properties were extracted, default to Stream
    if properties.is_empty() {
        properties.insert(HydroEdgeProp::Stream);
    }

    structure.add_edge(src_id, dst_id, properties, label);
}

/// Helper function to write a graph structure using any GraphWrite implementation
fn write_graph_structure<W>(
    structure: &HydroGraphStructure,
    graph_write: W,
    config: &HydroWriteConfig,
) -> Result<(), W::Err>
where
    W: HydroGraphWrite,
{
    let mut graph_write = graph_write;
    // Write the graph
    graph_write.write_prologue()?;

    // Write node definitions
    for (&node_id, node) in &structure.nodes {
        let (location_id, location_type) = if let Some(loc_id) = node.location {
            (
                Some(loc_id),
                structure.locations.get(&loc_id).map(|s| s.as_str()),
            )
        } else {
            (None, None)
        };

        graph_write.write_node_definition(
            node_id,
            &node.label,
            node.node_type,
            location_id,
            location_type,
            node.backtrace.as_ref(),
        )?;
    }

    // Group nodes by location if requested
    if config.show_location_groups {
        let mut nodes_by_location: HashMap<usize, Vec<usize>> = HashMap::new();
        for (&node_id, node) in &structure.nodes {
            if let Some(location_id) = node.location {
                nodes_by_location
                    .entry(location_id)
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
    for edge in &structure.edges {
        graph_write.write_edge(
            edge.src,
            edge.dst,
            &edge.edge_properties,
            edge.label.as_deref(),
        )?;
    }

    graph_write.write_epilogue()?;
    Ok(())
}

impl HydroRoot {
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
            sink_metadata: Option<&HydroIrMetadata>,
            label: NodeLabel,
        ) -> usize {
            let input_id = input.build_graph_structure(structure, seen_tees, config);

            // If no explicit metadata is provided, extract it from the input node
            let effective_metadata = if let Some(meta) = sink_metadata {
                Some(meta)
            } else {
                match input {
                    HydroNode::Placeholder => None,
                    // All other variants have metadata
                    _ => Some(input.metadata()),
                }
            };

            let location_id = effective_metadata.and_then(|m| setup_location(structure, m));
            let sink_id = structure.add_node_with_backtrace(
                label,
                HydroNodeType::Sink,
                location_id,
                effective_metadata.map(|m| m.op.backtrace.clone()),
            );

            // Extract semantic tags from input metadata
            let input_metadata = input.metadata();
            add_edge_with_metadata(
                structure,
                input_id,
                sink_id,
                Some(input_metadata),
                sink_metadata,
                None,
            );

            sink_id
        }

        match self {
            // Sink operations - semantic tags extracted from input metadata
            HydroRoot::ForEach { f, input, .. } => build_sink_node(
                structure,
                seen_tees,
                config,
                input,
                None,
                NodeLabel::with_exprs("for_each".to_string(), vec![f.clone()]),
            ),

            HydroRoot::SendExternal {
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
            ),

            HydroRoot::DestSink { sink, input, .. } => build_sink_node(
                structure,
                seen_tees,
                config,
                input,
                None,
                NodeLabel::with_exprs("dest_sink".to_string(), vec![sink.clone()]),
            ),

            HydroRoot::CycleSink { ident, input, .. } => build_sink_node(
                structure,
                seen_tees,
                config,
                input,
                None,
                NodeLabel::static_label(format!("cycle_sink({})", ident)),
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
        use crate::location::dynamic::LocationId;

        // Helper functions to reduce duplication, categorized by input/expression patterns

        /// Common parameters for transform builder functions to reduce argument count
        struct TransformParams<'a> {
            structure: &'a mut HydroGraphStructure,
            seen_tees: &'a mut HashMap<*const std::cell::RefCell<HydroNode>, usize>,
            config: &'a HydroWriteConfig,
            input: &'a HydroNode,
            metadata: &'a HydroIrMetadata,
            op_name: String,
            node_type: HydroNodeType,
        }

        // Single-input transform with no expressions
        fn build_simple_transform(params: TransformParams) -> usize {
            let input_id = params.input.build_graph_structure(
                params.structure,
                params.seen_tees,
                params.config,
            );
            let node_id = params.structure.add_node_with_metadata(
                NodeLabel::Static(params.op_name.to_string()),
                params.node_type,
                params.metadata,
            );

            // Extract semantic tags from input metadata
            let input_metadata = params.input.metadata();
            add_edge_with_metadata(
                params.structure,
                input_id,
                node_id,
                Some(input_metadata),
                Some(params.metadata),
                None,
            );

            node_id
        }

        // Single-input transform with one expression
        fn build_single_expr_transform(params: TransformParams, expr: &DebugExpr) -> usize {
            let input_id = params.input.build_graph_structure(
                params.structure,
                params.seen_tees,
                params.config,
            );
            let node_id = params.structure.add_node_with_metadata(
                NodeLabel::with_exprs(params.op_name.to_string(), vec![expr.clone()]),
                params.node_type,
                params.metadata,
            );

            // Extract semantic tags from input metadata
            let input_metadata = params.input.metadata();
            add_edge_with_metadata(
                params.structure,
                input_id,
                node_id,
                Some(input_metadata),
                Some(params.metadata),
                None,
            );

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
            let node_id = params.structure.add_node_with_metadata(
                NodeLabel::with_exprs(
                    params.op_name.to_string(),
                    vec![expr1.clone(), expr2.clone()],
                ),
                params.node_type,
                params.metadata,
            );

            // Extract semantic tags from input metadata
            let input_metadata = params.input.metadata();
            add_edge_with_metadata(
                params.structure,
                input_id,
                node_id,
                Some(input_metadata),
                Some(params.metadata),
                None,
            );

            node_id
        }

        // Helper function for source nodes
        fn build_source_node(
            structure: &mut HydroGraphStructure,
            metadata: &HydroIrMetadata,
            label: String,
        ) -> usize {
            structure.add_node_with_metadata(
                NodeLabel::Static(label),
                HydroNodeType::Source,
                metadata,
            )
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

            HydroNode::SingletonSource { value, metadata } => {
                let label = format!("singleton({})", value);
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
                let tee_id = structure.add_node_with_metadata(
                    NodeLabel::Static(extract_op_name(self.print_root())),
                    HydroNodeType::Tee,
                    metadata,
                );

                seen_tees.insert(ptr, tee_id);

                // Extract semantic tags from input
                let inner_borrow = inner.0.borrow();
                let input_metadata = inner_borrow.metadata();
                add_edge_with_metadata(
                    structure,
                    input_id,
                    tee_id,
                    Some(input_metadata),
                    Some(metadata),
                    None,
                );
                drop(inner_borrow);

                tee_id
            }

            // Transform operations with Stream edges - grouped by node/edge type
            HydroNode::Cast { inner, metadata }
            | HydroNode::ObserveNonDet {
                inner, metadata, ..
            }
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
            }),

            // Transform operation - semantic tags extracted from metadata
            HydroNode::Persist { inner, metadata } => build_simple_transform(TransformParams {
                structure,
                seen_tees,
                config,
                input: inner,
                metadata,
                op_name: extract_op_name(self.print_root()),
                node_type: HydroNodeType::Transform,
            }),

            // Aggregation operation - semantic tags extracted from metadata
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
                let node_id = structure.add_node_with_metadata(
                    NodeLabel::Static(extract_op_name(self.print_root())),
                    HydroNodeType::Join,
                    metadata,
                );

                // Extract semantic tags for left edge
                let left_metadata = left.metadata();
                add_edge_with_metadata(
                    structure,
                    left_id,
                    node_id,
                    Some(left_metadata),
                    Some(metadata),
                    Some("left".to_string()),
                );

                // Extract semantic tags for right edge
                let right_metadata = right.metadata();
                add_edge_with_metadata(
                    structure,
                    right_id,
                    node_id,
                    Some(right_metadata),
                    Some(metadata),
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
                let node_id = structure.add_node_with_metadata(
                    NodeLabel::Static(extract_op_name(self.print_root())),
                    HydroNodeType::Join,
                    metadata,
                );

                // Extract semantic tags for pos edge
                let left_metadata = left.metadata();
                add_edge_with_metadata(
                    structure,
                    left_id,
                    node_id,
                    Some(left_metadata),
                    Some(metadata),
                    Some("pos".to_string()),
                );

                // Extract semantic tags for neg edge
                let right_metadata = right.metadata();
                add_edge_with_metadata(
                    structure,
                    right_id,
                    node_id,
                    Some(right_metadata),
                    Some(metadata),
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
                let join_node_id = structure.add_node_with_backtrace(
                    NodeLabel::Static(extract_op_name(self.print_root())),
                    HydroNodeType::Join,
                    location_id,
                    Some(metadata.op.backtrace.clone()),
                );

                // Extract semantic tags for input edge
                let input_metadata = input.metadata();
                add_edge_with_metadata(
                    structure,
                    input_id,
                    join_node_id,
                    Some(input_metadata),
                    Some(metadata),
                    Some("input".to_string()),
                );

                // Extract semantic tags for watermark edge
                let watermark_metadata = watermark.metadata();
                add_edge_with_metadata(
                    structure,
                    watermark_id,
                    join_node_id,
                    Some(watermark_metadata),
                    Some(metadata),
                    Some("watermark".to_string()),
                );

                let node_id = structure.add_node_with_backtrace(
                    NodeLabel::with_exprs(
                        extract_op_name(self.print_root()).to_string(),
                        vec![f.clone()],
                    ),
                    HydroNodeType::Aggregation,
                    location_id,
                    Some(metadata.op.backtrace.clone()),
                );

                // Edge from join to aggregation node
                let join_metadata = metadata; // Use the same metadata
                add_edge_with_metadata(
                    structure,
                    join_node_id,
                    node_id,
                    Some(join_metadata),
                    Some(metadata),
                    None,
                );

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
                    label.push_str("send");
                }
                if deserialize_fn.is_some() {
                    if serialize_fn.is_some() {
                        label.push_str(" + ");
                    }
                    label.push_str("recv");
                }
                label.push(')');

                let network_id = structure.add_node_with_backtrace(
                    NodeLabel::Static(label),
                    HydroNodeType::Network,
                    to_location_id,
                    Some(metadata.op.backtrace.clone()),
                );

                // Extract semantic tags for network edge
                let input_metadata = input.metadata();
                add_edge_with_metadata(
                    structure,
                    input_id,
                    network_id,
                    Some(input_metadata),
                    Some(metadata),
                    Some(format!("to {:?}", to_location_id)),
                );

                network_id
            }

            // Handle remaining node types
            HydroNode::Batch { inner, .. } => {
                // Unpersist is typically optimized away, just pass through
                inner.build_graph_structure(structure, seen_tees, config)
            }

            HydroNode::YieldConcat { inner, .. } => {
                // Unpersist is typically optimized away, just pass through
                inner.build_graph_structure(structure, seen_tees, config)
            }

            HydroNode::BeginAtomic { inner, .. } => {
                inner.build_graph_structure(structure, seen_tees, config)
            }

            HydroNode::EndAtomic { inner, .. } => {
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
                let chain_id = structure.add_node_with_backtrace(
                    NodeLabel::Static(extract_op_name(self.print_root())),
                    HydroNodeType::Transform,
                    location_id,
                    Some(metadata.op.backtrace.clone()),
                );

                // Extract semantic tags for first edge
                let first_metadata = first.metadata();
                add_edge_with_metadata(
                    structure,
                    first_id,
                    chain_id,
                    Some(first_metadata),
                    Some(metadata),
                    Some("first".to_string()),
                );

                // Extract semantic tags for second edge
                let second_metadata = second.metadata();
                add_edge_with_metadata(
                    structure,
                    second_id,
                    chain_id,
                    Some(second_metadata),
                    Some(metadata),
                    Some("second".to_string()),
                );

                chain_id
            }

            HydroNode::ChainFirst {
                first,
                second,
                metadata,
            } => {
                let first_id = first.build_graph_structure(structure, seen_tees, config);
                let second_id = second.build_graph_structure(structure, seen_tees, config);
                let location_id = setup_location(structure, metadata);
                let chain_id = structure.add_node_with_backtrace(
                    NodeLabel::Static(extract_op_name(self.print_root())),
                    HydroNodeType::Transform,
                    location_id,
                    Some(metadata.op.backtrace.clone()),
                );

                // Extract semantic tags for first edge
                let first_metadata = first.metadata();
                add_edge_with_metadata(
                    structure,
                    first_id,
                    chain_id,
                    Some(first_metadata),
                    Some(metadata),
                    Some("first".to_string()),
                );

                // Extract semantic tags for second edge
                let second_metadata = second.metadata();
                add_edge_with_metadata(
                    structure,
                    second_id,
                    chain_id,
                    Some(second_metadata),
                    Some(metadata),
                    Some("second".to_string()),
                );

                chain_id
            }

            HydroNode::Counter {
                tag: _,
                prefix: _,
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
                },
                duration,
            ),
        }
    }
}

/// Utility functions for rendering multiple roots as a single graph.
/// Macro to reduce duplication in render functions.
macro_rules! render_hydro_ir {
    ($name:ident, $write_fn:ident) => {
        pub fn $name(roots: &[HydroRoot], config: &HydroWriteConfig) -> String {
            let mut output = String::new();
            $write_fn(&mut output, roots, config).unwrap();
            output
        }
    };
}

/// Macro to reduce duplication in write functions.
macro_rules! write_hydro_ir {
    ($name:ident, $writer_type:ty, $constructor:expr) => {
        pub fn $name(
            output: impl std::fmt::Write,
            roots: &[HydroRoot],
            config: &HydroWriteConfig,
        ) -> std::fmt::Result {
            let mut graph_write: $writer_type = $constructor(output, config);
            write_hydro_ir_graph(&mut graph_write, roots, config)
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

// Legacy hydroscope function - now uses HydroJson for consistency
render_hydro_ir!(render_hydro_ir_hydroscope, write_hydro_ir_json);

// JSON rendering
render_hydro_ir!(render_hydro_ir_json, write_hydro_ir_json);
write_hydro_ir!(write_hydro_ir_json, HydroJson<_>, HydroJson::new);

fn write_hydro_ir_graph<W>(
    graph_write: W,
    roots: &[HydroRoot],
    config: &HydroWriteConfig,
) -> Result<(), W::Err>
where
    W: HydroGraphWrite,
{
    let mut structure = HydroGraphStructure::new();
    let mut seen_tees = HashMap::new();

    // Build the graph structure for all roots
    for leaf in roots {
        leaf.build_graph_structure(&mut structure, &mut seen_tees, config);
    }

    write_graph_structure(&structure, graph_write, config)
}
