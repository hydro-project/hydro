use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt::Write;

use auto_impl::auto_impl;

// Re-export specific implementations
pub use super::dot::{HydroDot, escape_dot};
pub use super::json::HydroJson;
pub use super::mermaid::{HydroMermaid, escape_mermaid};
use crate::builder::ir::backtrace::Backtrace;
use crate::builder::ir::{DebugExpr, HydroIrMetadata, HydroNode, HydroRoot, HydroSource};

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
        edge_properties: &HashSet<HydroEdgeType>,
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
        NODE_TYPE_DATA
            .iter()
            .map(|(nt, name)| (*nt, *name))
            .collect()
    }
}

/// Types of nodes in Hydro IR for styling purposes.
#[derive(Debug, Clone, Copy, PartialEq)]
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
pub enum HydroEdgeType {
    Bounded,
    Unbounded,
    TotalOrder,
    NoOrder,
    Keyed,
    // Collection type tags for styling
    Stream,
    KeyedStream,
    Singleton,
    Optional,
    Network,
    Cycle,
}

/// Unified edge style representation for all graph formats.
/// This intermediate format allows consistent styling across JSON, DOT, and Mermaid.
#[derive(Debug, Clone, PartialEq)]
pub struct UnifiedEdgeStyle {
    /// Line pattern (solid, dotted, dashed)
    pub line_pattern: LinePattern,
    /// Line width (1 = thin, 3 = thick)
    pub line_width: u8,
    /// Arrowhead style
    pub arrowhead: ArrowheadStyle,
    /// Line style for rails (single, double)
    pub line_style: LineStyle,
    /// Halo/background effect
    pub halo: HaloStyle,
    /// Line waviness for ordering information
    pub waviness: WavinessStyle,
    /// Whether animation is enabled (JSON only)
    pub animation: AnimationStyle,
    /// Color for the edge
    pub color: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LinePattern {
    Solid,
    Dotted,
    Dashed,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ArrowheadStyle {
    TriangleFilled,
    CircleFilled,
    DiamondOpen,
    Default,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LineStyle {
    Single,
    Double,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HaloStyle {
    None,
    LightRed,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WavinessStyle {
    None,
    Wavy,
}

#[derive(Debug, Clone, Copy, PartialEq)]
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
/// This is the core logic moved from json.rs get_edge_style_config.
pub fn get_unified_edge_style(
    edge_properties: &HashSet<HydroEdgeType>,
    src_location: Option<usize>,
    dst_location: Option<usize>,
) -> UnifiedEdgeStyle {
    let mut style = UnifiedEdgeStyle::default();

    // Network communication group - controls line pattern AND animation
    let is_network = edge_properties.contains(&HydroEdgeType::Network)
        || (src_location.is_some() && dst_location.is_some() && src_location != dst_location);

    if is_network {
        style.line_pattern = LinePattern::Dotted;
        style.animation = AnimationStyle::Animated;
        style.color = "#880088";
    } else {
        style.line_pattern = LinePattern::Solid;
        style.animation = AnimationStyle::Static;
    }

    // Boundedness group - controls line width
    if edge_properties.contains(&HydroEdgeType::Bounded) {
        style.line_width = 3;
        if !is_network {
            style.color = "#008800";
        }
    } else if edge_properties.contains(&HydroEdgeType::Unbounded) {
        style.line_width = 1;
    }

    // Collection type group - controls arrowhead and rails (line-style)
    if edge_properties.contains(&HydroEdgeType::Stream) {
        style.arrowhead = ArrowheadStyle::TriangleFilled;
        style.line_style = LineStyle::Single;
    } else if edge_properties.contains(&HydroEdgeType::KeyedStream) {
        style.arrowhead = ArrowheadStyle::TriangleFilled;
        style.line_style = LineStyle::Double;
        if !is_network && !edge_properties.contains(&HydroEdgeType::Bounded) {
            style.color = "#0088ff";
        }
    } else if edge_properties.contains(&HydroEdgeType::Singleton) {
        style.arrowhead = ArrowheadStyle::CircleFilled;
        style.line_style = LineStyle::Single;
    } else if edge_properties.contains(&HydroEdgeType::Optional) {
        style.arrowhead = ArrowheadStyle::DiamondOpen;
        style.line_style = LineStyle::Single;
    }

    // Flow control group - controls halo
    if edge_properties.contains(&HydroEdgeType::Cycle) {
        style.halo = HaloStyle::LightRed;
        style.color = "#ff8800";
    }

    // Ordering group - waviness channel
    if edge_properties.contains(&HydroEdgeType::NoOrder) {
        style.waviness = WavinessStyle::Wavy;
        if !is_network
            && !edge_properties.contains(&HydroEdgeType::Bounded)
            && !edge_properties.contains(&HydroEdgeType::KeyedStream)
            && !edge_properties.contains(&HydroEdgeType::Cycle)
        {
            style.color = "#ff0000";
        }
    } else if edge_properties.contains(&HydroEdgeType::TotalOrder) {
        style.waviness = WavinessStyle::None;
    }

    style
}

/// Generate a proper location label using the configuration's name mappings.
/// This ensures consistent location labeling across all graph renderers.
pub fn get_location_label(
    location_id: usize,
    location_type: &str,
    config: &HydroWriteConfig,
) -> String {
    match location_type {
        "Process" => {
            // Convert to HashMap for efficient lookup
            let process_names: HashMap<usize, String> =
                config.process_id_name.iter().cloned().collect();
            if let Some(name) = process_names.get(&location_id) {
                name.clone()
            } else {
                format!("Process {}", location_id)
            }
        }
        "Cluster" => {
            let cluster_names: HashMap<usize, String> =
                config.cluster_id_name.iter().cloned().collect();
            if let Some(name) = cluster_names.get(&location_id) {
                name.clone()
            } else {
                format!("Cluster {}", location_id)
            }
        }
        "External" => {
            let external_names: HashMap<usize, String> =
                config.external_id_name.iter().cloned().collect();
            if let Some(name) = external_names.get(&location_id) {
                name.clone()
            } else {
                format!("External {}", location_id)
            }
        }
        _ => location_type.to_string(),
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

/// Graph structure tracker for Hydro IR rendering.
#[derive(Default)]
pub struct HydroGraphStructure {
    pub nodes: HashMap<usize, (NodeLabel, HydroNodeType, Option<usize>, Option<Backtrace>)>, // node_id -> (label, type, location, backtrace)
    pub edges: Vec<(usize, usize, HashSet<HydroEdgeType>, Option<String>)>, /* (src, dst, edge_properties, label) */
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
        backtrace: Option<Backtrace>,
    ) -> usize {
        let node_id = self.next_node_id;
        self.next_node_id += 1;
        self.nodes
            .insert(node_id, (label, node_type, location, backtrace));
        node_id
    }

    pub fn add_edge(
        &mut self,
        src: usize,
        dst: usize,
        edge_properties: HashSet<HydroEdgeType>,
        label: Option<String>,
    ) {
        self.edges.push((src, dst, edge_properties, label));
    }

    pub fn add_location(&mut self, location_id: usize, location_type: String) {
        self.locations.insert(location_id, location_type);
    }
}

/// Extract edge properties from a Hydro type (e.g., Stream<T, L, Bounded, TotalOrder>).
pub fn extract_edge_properties_from_type(ty: &syn::Type) -> HashSet<HydroEdgeType> {
    let mut properties = HashSet::new();

    // Fast path: unwrap wrappers and combine properties recursively
    match ty {
        // Handle tuples introduced by optimizers like (ClusterId<()>, T)
        syn::Type::Tuple(t) => {
            for elem in &t.elems {
                properties.extend(extract_edge_properties_from_type(elem));
            }
            return properties;
        }
        // Handle &T
        syn::Type::Reference(r) => {
            properties.extend(extract_edge_properties_from_type(&r.elem));
            return properties;
        }
        // Handle (T)
        syn::Type::Paren(p) => {
            properties.extend(extract_edge_properties_from_type(&p.elem));
            return properties;
        }
        // Handle grouped types
        syn::Type::Group(g) => {
            properties.extend(extract_edge_properties_from_type(&g.elem));
            return properties;
        }
        _ => {}
    }

    // Parse the type to extract stream properties
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            let type_name = segment.ident.to_string();

            match type_name.as_str() {
                "Stream" => {
                    // Tag collection type
                    properties.insert(HydroEdgeType::Stream);
                    // Stream<T, L, Bound, Order, Retries>
                    if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                        let type_args: Vec<_> = args.args.iter().collect();

                        // Extract boundedness (3rd type param: index 2)
                        if let Some(syn::GenericArgument::Type(bound_ty)) = type_args.get(2)
                            && let syn::Type::Path(bound_path) = bound_ty
                            && let Some(bound_segment) = bound_path.path.segments.last()
                        {
                            match bound_segment.ident.to_string().as_str() {
                                "Bounded" => {
                                    properties.insert(HydroEdgeType::Bounded);
                                }
                                "Unbounded" => {
                                    properties.insert(HydroEdgeType::Unbounded);
                                }
                                _ => {}
                            }
                        }

                        // Extract ordering (4th type param: index 3)
                        if let Some(syn::GenericArgument::Type(order_ty)) = type_args.get(3)
                            && let syn::Type::Path(order_path) = order_ty
                            && let Some(order_segment) = order_path.path.segments.last()
                        {
                            match order_segment.ident.to_string().as_str() {
                                "TotalOrder" => {
                                    properties.insert(HydroEdgeType::TotalOrder);
                                }
                                "NoOrder" => {
                                    properties.insert(HydroEdgeType::NoOrder);
                                }
                                _ => {}
                            }
                        }
                    }
                }
                "KeyedStream" => {
                    // KeyedStream<K, V, L, Bound, Order, Retries>
                    properties.insert(HydroEdgeType::KeyedStream);
                    properties.insert(HydroEdgeType::Keyed);

                    if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                        let type_args: Vec<_> = args.args.iter().collect();

                        // Extract boundedness (4th type param: index 3)
                        if let Some(syn::GenericArgument::Type(bound_ty)) = type_args.get(3)
                            && let syn::Type::Path(bound_path) = bound_ty
                            && let Some(bound_segment) = bound_path.path.segments.last()
                        {
                            match bound_segment.ident.to_string().as_str() {
                                "Bounded" => {
                                    properties.insert(HydroEdgeType::Bounded);
                                }
                                "Unbounded" => {
                                    properties.insert(HydroEdgeType::Unbounded);
                                }
                                _ => {}
                            }
                        }

                        // Extract ordering (5th type param: index 4)
                        if let Some(syn::GenericArgument::Type(order_ty)) = type_args.get(4)
                            && let syn::Type::Path(order_path) = order_ty
                            && let Some(order_segment) = order_path.path.segments.last()
                        {
                            match order_segment.ident.to_string().as_str() {
                                "TotalOrder" => {
                                    properties.insert(HydroEdgeType::TotalOrder);
                                }
                                "NoOrder" => {
                                    properties.insert(HydroEdgeType::NoOrder);
                                }
                                _ => {}
                            }
                        }
                    }
                }
                "Singleton" | "Optional" => {
                    if type_name == "Singleton" {
                        properties.insert(HydroEdgeType::Singleton);
                    } else {
                        properties.insert(HydroEdgeType::Optional);
                    }
                    // Singletons/Optionals can have Bound/Order type params too
                    // Singleton<T, L, Bound, Order, Retries>
                    if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                        let type_args: Vec<_> = args.args.iter().collect();

                        // Extract boundedness (3rd type param: index 2) - defaults to Bounded
                        if let Some(syn::GenericArgument::Type(bound_ty)) = type_args.get(2)
                            && let syn::Type::Path(bound_path) = bound_ty
                            && let Some(bound_segment) = bound_path.path.segments.last()
                        {
                            match bound_segment.ident.to_string().as_str() {
                                "Bounded" => {
                                    properties.insert(HydroEdgeType::Bounded);
                                }
                                "Unbounded" => {
                                    properties.insert(HydroEdgeType::Unbounded);
                                }
                                _ => {
                                    properties.insert(HydroEdgeType::Bounded);
                                }
                            };
                        } else {
                            properties.insert(HydroEdgeType::Bounded);
                        }

                        // Extract ordering (4th type param: index 3) - defaults to TotalOrder
                        if let Some(syn::GenericArgument::Type(order_ty)) = type_args.get(3)
                            && let syn::Type::Path(order_path) = order_ty
                            && let Some(order_segment) = order_path.path.segments.last()
                        {
                            match order_segment.ident.to_string().as_str() {
                                "TotalOrder" => {
                                    properties.insert(HydroEdgeType::TotalOrder);
                                }
                                "NoOrder" => {
                                    properties.insert(HydroEdgeType::NoOrder);
                                }
                                _ => {
                                    properties.insert(HydroEdgeType::TotalOrder);
                                }
                            };
                        } else {
                            properties.insert(HydroEdgeType::TotalOrder);
                        }
                    } else {
                        // No type args, use defaults for Singleton
                        properties.insert(HydroEdgeType::Bounded);
                        properties.insert(HydroEdgeType::TotalOrder);
                    }
                }
                _ => {
                    // Unknown/wrapper type. If it has generic type arguments, recurse into them
                    // so we can pick up inner Stream/KeyedStream/Singleton/Optional semantics.
                    if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                        for arg in &args.args {
                            if let syn::GenericArgument::Type(inner_ty) = arg {
                                properties.extend(extract_edge_properties_from_type(inner_ty));
                            }
                        }
                    }
                    // Do not set defaults here; allow final fallback below if still empty.
                }
            }
        }
    } else {
        // Non-path type (and not covered above), use defaults
        properties.insert(HydroEdgeType::Unbounded);
        properties.insert(HydroEdgeType::TotalOrder);
    }

    // If no specific properties were found, assume basic stream properties
    if properties.is_empty() {
        properties.insert(HydroEdgeType::Unbounded);
        properties.insert(HydroEdgeType::TotalOrder);
    }

    properties
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
fn extract_location_id(metadata: &HydroIrMetadata) -> (Option<usize>, Option<String>) {
    use crate::location::dynamic::LocationId;
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
    metadata: &HydroIrMetadata,
) -> Option<usize> {
    let (location_id, location_type) = extract_location_id(metadata);
    if let (Some(loc_id), Some(loc_type)) = (location_id, location_type) {
        structure.add_location(loc_id, loc_type);
    }
    location_id
}

/// Helper function to extract edge properties from a node's output type.
fn extract_edge_properties_from_node(node: &HydroNode) -> HashSet<HydroEdgeType> {
    // Get the metadata from the node
    let metadata = match node {
        HydroNode::Placeholder => return HashSet::new(),
        // All other variants have metadata
        _ => node.metadata(),
    };

    // Extract properties from the output type if available
    if let Some(output_type) = &metadata.output_type {
        let mut properties = extract_edge_properties_from_type(output_type);

        // Add special properties based on node type
        match node {
            HydroNode::Network { .. } => {
                properties.insert(HydroEdgeType::Network);
            }
            HydroNode::CycleSource { .. } => {
                properties.insert(HydroEdgeType::Cycle);
            }
            _ => {}
        }

        properties
    } else {
        // Fallback: basic properties
        let mut properties = HashSet::new();
        properties.insert(HydroEdgeType::Unbounded);
        properties.insert(HydroEdgeType::TotalOrder);
        properties
    }
}

impl HydroRoot {
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
        for (&node_id, (label, node_type, location, backtrace)) in &structure.nodes {
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
                backtrace.as_ref(),
            )?;
        }

        // Group nodes by location if requested
        if config.show_location_groups {
            let mut nodes_by_location: HashMap<usize, Vec<usize>> = HashMap::new();
            for (&node_id, (_, _, location, _)) in &structure.nodes {
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
        for (src_id, dst_id, edge_properties, label) in &structure.edges {
            graph_write.write_edge(*src_id, *dst_id, edge_properties, label.as_deref())?;
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
            metadata: Option<&HydroIrMetadata>,
            label: NodeLabel,
        ) -> usize {
            let input_id = input.build_graph_structure(structure, seen_tees, config);

            // If no explicit metadata is provided, extract it from the input node
            let effective_metadata = if let Some(meta) = metadata {
                Some(meta)
            } else {
                match input {
                    HydroNode::Placeholder => None,
                    // All other variants have metadata
                    _ => Some(input.metadata()),
                }
            };

            let location_id = effective_metadata.and_then(|m| setup_location(structure, m));
            let sink_id = structure.add_node(
                label,
                HydroNodeType::Sink,
                location_id,
                effective_metadata.map(|m| m.op.backtrace.clone()),
            );

            // Extract edge properties from the input node's output type
            let edge_properties = extract_edge_properties_from_node(input);
            structure.add_edge(input_id, sink_id, edge_properties, None);
            sink_id
        }

        match self {
            // Sink operations with Stream edges - grouped by edge type
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

            // Sink operation with Cycle edge - grouped by edge type
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

        // Helper function for single-input transforms
        fn build_single_input_transform(
            structure: &mut HydroGraphStructure,
            seen_tees: &mut HashMap<*const std::cell::RefCell<HydroNode>, usize>,
            config: &HydroWriteConfig,
            input: &HydroNode,
            metadata: &HydroIrMetadata,
            label: NodeLabel,
            node_type: HydroNodeType,
        ) -> usize {
            let input_id = input.build_graph_structure(structure, seen_tees, config);
            let location_id = setup_location(structure, metadata);
            let node_id = structure.add_node(
                label,
                node_type,
                location_id,
                Some(metadata.op.backtrace.clone()),
            );

            // Extract edge properties from the input node's output type
            let edge_properties = extract_edge_properties_from_node(input);
            structure.add_edge(input_id, node_id, edge_properties, None);
            node_id
        }

        // Helper function for source nodes
        fn build_source_node(
            structure: &mut HydroGraphStructure,
            metadata: &HydroIrMetadata,
            label: String,
        ) -> usize {
            let location_id = setup_location(structure, metadata);
            structure.add_node(
                NodeLabel::Static(label),
                HydroNodeType::Source,
                location_id,
                Some(metadata.op.backtrace.clone()),
            )
        }

        // Helper struct to group parameters for two-input operations
        struct TwoInputParams<'a> {
            structure: &'a mut HydroGraphStructure,
            seen_tees: &'a mut HashMap<*const std::cell::RefCell<HydroNode>, usize>,
            config: &'a HydroWriteConfig,
            metadata: &'a HydroIrMetadata,
            op_name: String,
            node_type: HydroNodeType,
        }

        // Helper function for two-input operations with left/right labels
        fn build_left_right_join(
            params: TwoInputParams<'_>,
            left: &HydroNode,
            right: &HydroNode,
        ) -> usize {
            let left_id =
                left.build_graph_structure(params.structure, params.seen_tees, params.config);
            let right_id =
                right.build_graph_structure(params.structure, params.seen_tees, params.config);
            let location_id = setup_location(params.structure, params.metadata);
            let node_id = params.structure.add_node(
                NodeLabel::Static(params.op_name),
                params.node_type,
                location_id,
                Some(params.metadata.op.backtrace.clone()),
            );

            let left_properties = extract_edge_properties_from_node(left);
            let right_properties = extract_edge_properties_from_node(right);

            params
                .structure
                .add_edge(left_id, node_id, left_properties, Some("left".to_string()));
            params.structure.add_edge(
                right_id,
                node_id,
                right_properties,
                Some("right".to_string()),
            );
            node_id
        }

        // Helper function for two-input operations with pos/neg labels
        fn build_pos_neg_join(
            params: TwoInputParams<'_>,
            pos: &HydroNode,
            neg: &HydroNode,
        ) -> usize {
            let pos_id =
                pos.build_graph_structure(params.structure, params.seen_tees, params.config);
            let neg_id =
                neg.build_graph_structure(params.structure, params.seen_tees, params.config);
            let location_id = setup_location(params.structure, params.metadata);
            let node_id = params.structure.add_node(
                NodeLabel::Static(params.op_name),
                params.node_type,
                location_id,
                Some(params.metadata.op.backtrace.clone()),
            );

            let pos_properties = extract_edge_properties_from_node(pos);
            let neg_properties = extract_edge_properties_from_node(neg);

            params
                .structure
                .add_edge(pos_id, node_id, pos_properties, Some("pos".to_string()));
            params
                .structure
                .add_edge(neg_id, node_id, neg_properties, Some("neg".to_string()));
            node_id
        }

        match self {
            HydroNode::Placeholder => structure.add_node(
                NodeLabel::Static("PLACEHOLDER".to_string()),
                HydroNodeType::Transform,
                None,
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
                    Some(metadata.op.backtrace.clone()),
                );

                seen_tees.insert(ptr, tee_id);

                // Extract edge properties from the input node
                let edge_properties = extract_edge_properties_from_node(&inner.0.borrow());
                structure.add_edge(input_id, tee_id, edge_properties, None);

                tee_id
            }
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
            }
            | HydroNode::Persist { inner, metadata } => build_single_input_transform(
                structure,
                seen_tees,
                config,
                inner,
                metadata,
                NodeLabel::Static(extract_op_name(self.print_root())),
                HydroNodeType::Transform,
            ),
            HydroNode::Sort {
                input: inner,
                metadata,
            } => build_single_input_transform(
                structure,
                seen_tees,
                config,
                inner,
                metadata,
                NodeLabel::Static(extract_op_name(self.print_root())),
                HydroNodeType::Aggregation,
            ),
            HydroNode::Map { f, input, metadata }
            | HydroNode::Filter { f, input, metadata }
            | HydroNode::FlatMap { f, input, metadata }
            | HydroNode::FilterMap { f, input, metadata }
            | HydroNode::Inspect { f, input, metadata } => build_single_input_transform(
                structure,
                seen_tees,
                config,
                input,
                metadata,
                NodeLabel::with_exprs(extract_op_name(self.print_root()), vec![f.clone()]),
                HydroNodeType::Transform,
            ),
            HydroNode::Reduce { f, input, metadata }
            | HydroNode::ReduceKeyed { f, input, metadata } => build_single_input_transform(
                structure,
                seen_tees,
                config,
                input,
                metadata,
                NodeLabel::with_exprs(extract_op_name(self.print_root()), vec![f.clone()]),
                HydroNodeType::Aggregation,
            ),
            HydroNode::ReduceKeyedWatermark {
                f,
                input,
                watermark,
                metadata,
            } => {
                let input_id = input.build_graph_structure(structure, seen_tees, config);
                let watermark_id = watermark.build_graph_structure(structure, seen_tees, config);
                let location_id = setup_location(structure, metadata);
                let node_id = structure.add_node(
                    NodeLabel::with_exprs(extract_op_name(self.print_root()), vec![f.clone()]),
                    HydroNodeType::Aggregation,
                    location_id,
                    Some(metadata.op.backtrace.clone()),
                );

                let input_properties = extract_edge_properties_from_node(input);
                let watermark_properties = extract_edge_properties_from_node(watermark);

                structure.add_edge(
                    input_id,
                    node_id,
                    input_properties,
                    Some("input".to_string()),
                );
                structure.add_edge(
                    watermark_id,
                    node_id,
                    watermark_properties,
                    Some("watermark".to_string()),
                );
                node_id
            }
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
            } => build_left_right_join(
                TwoInputParams {
                    structure,
                    seen_tees,
                    config,
                    metadata,
                    op_name: extract_op_name(self.print_root()),
                    node_type: HydroNodeType::Join,
                },
                left,
                right,
            ),
            HydroNode::Difference {
                pos: left,
                neg: right,
                metadata,
            }
            | HydroNode::AntiJoin {
                pos: left,
                neg: right,
                metadata,
            } => build_pos_neg_join(
                TwoInputParams {
                    structure,
                    seen_tees,
                    config,
                    metadata,
                    op_name: extract_op_name(self.print_root()),
                    node_type: HydroNodeType::Join,
                },
                left,
                right,
            ),
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
            } => build_single_input_transform(
                structure,
                seen_tees,
                config,
                input,
                metadata,
                NodeLabel::with_exprs(
                    extract_op_name(self.print_root()),
                    vec![init.clone(), acc.clone()],
                ),
                HydroNodeType::Transform,
            ),
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
                    Some(metadata.op.backtrace.clone()),
                );
                // Extract edge properties and add Network property
                let mut edge_properties = extract_edge_properties_from_node(input);
                edge_properties.insert(HydroEdgeType::Network);
                structure.add_edge(
                    input_id,
                    network_id,
                    edge_properties,
                    Some(format!("to {:?}", to_location_id)),
                );
                network_id
            }
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
                    Some(metadata.op.backtrace.clone()),
                );
                let first_properties = extract_edge_properties_from_node(first);
                let second_properties = extract_edge_properties_from_node(second);

                structure.add_edge(
                    first_id,
                    chain_id,
                    first_properties,
                    Some("first".to_string()),
                );
                structure.add_edge(
                    second_id,
                    chain_id,
                    second_properties,
                    Some("second".to_string()),
                );
                chain_id
            }
            HydroNode::Counter {
                prefix: _,
                duration,
                input,
                metadata,
                tag,
            } => build_single_input_transform(
                structure,
                seen_tees,
                config,
                input,
                metadata,
                NodeLabel::with_exprs(
                    extract_op_name(self.print_root()),
                    vec![
                        syn::parse_str::<syn::Expr>(&format!("{:?}", tag))
                            .unwrap()
                            .into(),
                        duration.clone(),
                    ],
                ),
                HydroNodeType::Transform,
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

render_hydro_ir!(render_hydro_ir_json, write_hydro_ir_json);
write_hydro_ir!(write_hydro_ir_json, HydroJson<_>, HydroJson::new);

fn write_hydro_ir_graph<W>(
    mut graph_write: W,
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

    // Write the graph using the same logic as individual roots
    graph_write.write_prologue()?;

    for (&node_id, (label, node_type, location, backtrace)) in &structure.nodes {
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
            backtrace.as_ref(),
        )?;
    }

    if config.show_location_groups {
        let mut nodes_by_location: HashMap<usize, Vec<usize>> = HashMap::new();
        for (&node_id, (_, _, location, _)) in &structure.nodes {
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

    for (src_id, dst_id, edge_properties, label) in &structure.edges {
        graph_write.write_edge(*src_id, *dst_id, edge_properties, label.as_deref())?;
    }

    graph_write.write_epilogue()?;
    Ok(())
}
