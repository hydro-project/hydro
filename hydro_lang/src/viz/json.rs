use std::collections::{HashMap, HashSet};
use std::fmt::Write;

use serde::Serialize;
use slotmap::{SecondaryMap, SparseSecondaryMap};

use super::render::{
    GraphWriteError, HydroEdgeProp, HydroGraphWrite, HydroNodeType, HydroWriteConfig,
    write_hydro_ir_json,
};
use crate::compile::ir::HydroRoot;
use crate::compile::ir::backtrace::Backtrace;
use crate::location::{LocationKey, LocationType};
use crate::viz::render::VizNodeKey;

/// A serializable backtrace frame for JSON output.
/// Includes compatibility aliases to match potential viewer expectations.
#[derive(Serialize)]
struct BacktraceFrame {
    /// Function name (truncated)
    #[serde(rename = "fn")]
    fn_name: String,
    /// Function name alias for compatibility
    function: String,
    /// File path (truncated)
    file: String,
    /// File path alias for compatibility
    filename: String,
    /// Line number
    line: Option<u32>,
    /// Line number alias for compatibility
    #[serde(rename = "lineNumber")]
    line_number: Option<u32>,
}

/// Node data for JSON output.
#[derive(Serialize)]
struct NodeData {
    #[serde(rename = "locationKey")]
    location_key: Option<LocationKey>,
    #[serde(rename = "locationType")]
    location_type: Option<LocationType>,
    backtrace: serde_json::Value,
}

/// A serializable node for JSON output.
#[derive(Serialize)]
struct Node {
    id: String,
    #[serde(rename = "nodeType")]
    node_type: String,
    #[serde(rename = "fullLabel")]
    full_label: String,
    #[serde(rename = "shortLabel")]
    short_label: String,
    label: String,
    data: NodeData,
}

/// A serializable edge for JSON output.
#[derive(Serialize)]
struct Edge {
    id: String,
    source: String,
    target: String,
    #[serde(rename = "semanticTags")]
    semantic_tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    label: Option<String>,
}

/// JSON graph writer for Hydro IR.
/// Outputs JSON that can be used with interactive graph visualization tools.
pub struct HydroJson<'a, W> {
    write: W,
    nodes: Vec<serde_json::Value>,
    edges: Vec<serde_json::Value>,
    /// location_id -> (label, node_ids)
    locations: SecondaryMap<LocationKey, (String, Vec<VizNodeKey>)>,
    /// node_id -> location_id
    node_locations: SecondaryMap<VizNodeKey, LocationKey>,
    edge_count: usize,
    /// Map from raw location IDs to location names.
    location_names: &'a SecondaryMap<LocationKey, String>,
    /// Store backtraces for hierarchy generation.
    node_backtraces: SparseSecondaryMap<VizNodeKey, Backtrace>,
    /// Config flags.
    use_short_labels: bool,
}

impl<'a, W> HydroJson<'a, W> {
    pub fn new(write: W, config: HydroWriteConfig<'a>) -> Self {
        Self {
            write,
            nodes: Vec::new(),
            edges: Vec::new(),
            locations: SecondaryMap::new(),
            node_locations: SecondaryMap::new(),
            edge_count: 0,
            location_names: config.location_names,
            node_backtraces: SparseSecondaryMap::new(),
            use_short_labels: config.use_short_labels,
        }
    }

    /// Convert HydroNodeType to string representation
    fn node_type_to_string(node_type: HydroNodeType) -> &'static str {
        super::render::node_type_utils::to_string(node_type)
    }

    /// Convert HydroEdgeType to string representation for semantic tags
    fn edge_type_to_string(edge_type: HydroEdgeProp) -> String {
        match edge_type {
            HydroEdgeProp::Bounded => "Bounded".to_owned(),
            HydroEdgeProp::Unbounded => "Unbounded".to_owned(),
            HydroEdgeProp::TotalOrder => "TotalOrder".to_owned(),
            HydroEdgeProp::NoOrder => "NoOrder".to_owned(),
            HydroEdgeProp::Keyed => "Keyed".to_owned(),
            HydroEdgeProp::Stream => "Stream".to_owned(),
            HydroEdgeProp::KeyedSingleton => "KeyedSingleton".to_owned(),
            HydroEdgeProp::KeyedStream => "KeyedStream".to_owned(),
            HydroEdgeProp::Singleton => "Singleton".to_owned(),
            HydroEdgeProp::Optional => "Optional".to_owned(),
            HydroEdgeProp::Network => "Network".to_owned(),
            HydroEdgeProp::Cycle => "Cycle".to_owned(),
        }
    }

    /// Get all node type definitions for JSON output
    fn get_node_type_definitions() -> Vec<serde_json::Value> {
        // Ensure deterministic ordering by sorting by type string
        let mut types: Vec<(usize, &'static str)> =
            super::render::node_type_utils::all_types_with_strings()
                .into_iter()
                .enumerate()
                .map(|(idx, (_, type_str))| (idx, type_str))
                .collect();
        types.sort_by(|a, b| a.1.cmp(b.1));
        types
            .into_iter()
            .enumerate()
            .map(|(color_index, (_, type_str))| {
                serde_json::json!({
                    "id": type_str,
                    "label": type_str,
                    "colorIndex": color_index
                })
            })
            .collect()
    }

    /// Get legend items for JSON output (simplified version of node type definitions)
    fn get_legend_items() -> Vec<serde_json::Value> {
        Self::get_node_type_definitions()
            .into_iter()
            .map(|def| {
                serde_json::json!({
                    "type": def["id"],
                    "label": def["label"]
                })
            })
            .collect()
    }

    /// Get edge style configuration with semantic→style mappings.
    fn get_edge_style_config() -> serde_json::Value {
        serde_json::json!({
            "semanticPriorities": [
                ["Unbounded", "Bounded"],
                ["NoOrder", "TotalOrder"],
                ["Keyed", "NotKeyed"],
                ["Network", "Local"]
            ],
            "semanticMappings": {
                // Network communication group - controls line pattern AND animation
                "NetworkGroup": {
                    "Local": {
                        "line-pattern": "solid",
                        "animation": "static"
                    },
                    "Network": {
                        "line-pattern": "dashed",
                        "animation": "animated"
                    }
                },

                // Ordering group - controls waviness
                "OrderingGroup": {
                    "TotalOrder": {
                        "waviness": "straight"
                    },
                    "NoOrder": {
                        "waviness": "wavy"
                    }
                },

                // Boundedness group - controls halo
                "BoundednessGroup": {
                    "Bounded": {
                        "halo": "none"
                    },
                    "Unbounded": {
                        "halo": "light-blue"
                    }
                },

                // Keyedness group - controls vertical hash marks on the line
                "KeyednessGroup": {
                    "NotKeyed": {
                        "line-style": "single"
                    },
                    "Keyed": {
                        "line-style": "hash-marks"
                    }
                },

                // Collection type group - controls color
                "CollectionGroup": {
                    "Stream": {
                        "color": "#2563eb",
                        "arrowhead": "triangle-filled"
                    },
                    "Singleton": {
                        "color": "#000000",
                        "arrowhead": "circle-filled"
                    },
                    "Optional": {
                        "color": "#6b7280",
                        "arrowhead": "diamond-open"
                    }
                },
            },
            "note": "Edge styles are now computed per-edge using the unified edge style system. This config is provided for reference and compatibility."
        })
    }

    /// Optimize backtrace data for size efficiency
    /// 1. Remove redundant/non-essential frames
    /// 2. Truncate paths
    /// 3. Remove memory addresses (not useful for visualization)
    fn optimize_backtrace(&self, backtrace: &Backtrace) -> serde_json::Value {
        #[cfg(feature = "build")]
        {
            let elements = backtrace.elements();

            // filter out obviously internal frames
            let relevant_frames: Vec<BacktraceFrame> = elements
                .map(|elem| {
                    // Truncate paths and function names for size
                    let short_filename = elem
                        .filename
                        .as_deref()
                        .map(|f| Self::truncate_path(f))
                        .unwrap_or_else(|| "unknown".to_owned());

                    let short_fn_name = Self::truncate_function_name(&elem.fn_name).to_owned();

                    BacktraceFrame {
                        fn_name: short_fn_name.to_owned(),
                        function: short_fn_name,
                        file: short_filename.clone(),
                        filename: short_filename,
                        line: elem.lineno,
                        line_number: elem.lineno,
                    }
                })
                .collect();

            serde_json::to_value(relevant_frames).unwrap_or_else(|_| serde_json::json!([]))
        }
        #[cfg(not(feature = "build"))]
        {
            serde_json::json!([])
        }
    }

    /// Truncate file paths to keep only the relevant parts
    fn truncate_path(path: &str) -> String {
        let parts: Vec<&str> = path.split('/').collect();

        // For paths like "/Users/foo/project/src/main.rs", keep "src/main.rs"
        if let Some(src_idx) = parts.iter().rposition(|&p| p == "src") {
            parts[src_idx..].join("/")
        } else if parts.len() > 2 {
            // Keep last 2 components
            parts[parts.len().saturating_sub(2)..].join("/")
        } else {
            path.to_owned()
        }
    }

    /// Truncate function names to remove module paths
    fn truncate_function_name(fn_name: &str) -> &str {
        // Remove everything before the last "::" to get just the function name
        fn_name.split("::").last().unwrap_or(fn_name)
    }
}

impl<W> HydroGraphWrite for HydroJson<'_, W>
where
    W: Write,
{
    type Err = GraphWriteError;

    fn write_prologue(&mut self) -> Result<(), Self::Err> {
        // Clear any existing data
        self.nodes.clear();
        self.edges.clear();
        self.locations.clear();
        self.node_locations.clear();
        self.edge_count = 0;
        Ok(())
    }

    fn write_node_definition(
        &mut self,
        node_id: VizNodeKey,
        node_label: &super::render::NodeLabel,
        node_type: HydroNodeType,
        location_key: Option<LocationKey>,
        location_type: Option<LocationType>,
        backtrace: Option<&Backtrace>,
    ) -> Result<(), Self::Err> {
        // Create the full label string using DebugExpr::Display for expressions
        let full_label = match node_label {
            super::render::NodeLabel::Static(s) => s.clone(),
            super::render::NodeLabel::WithExprs { op_name, exprs } => {
                if exprs.is_empty() {
                    format!("{}()", op_name)
                } else {
                    // This is where DebugExpr::Display gets called with q! macro cleanup
                    let expr_strs: Vec<String> = exprs.iter().map(|e| e.to_string()).collect();
                    format!("{}({})", op_name, expr_strs.join(", "))
                }
            }
        };

        // Always extract short label for UI toggle functionality
        let short_label = super::render::extract_short_label(&full_label);

        // If short and full labels are the same or very similar, enhance the full label
        // Use saturating comparison to avoid underflow when full_label is very short
        let full_len = full_label.len();
        let enhanced_full_label = if short_label.len() >= full_len.saturating_sub(2) {
            // If they're nearly the same length, add more context to full label
            match short_label.as_str() {
                "inspect" => "inspect [debug output]".to_owned(),
                "persist" => "persist [state storage]".to_owned(),
                "tee" => "tee [branch dataflow]".to_owned(),
                "delta" => "delta [change detection]".to_owned(),
                "spin" => "spin [delay/buffer]".to_owned(),
                "send_bincode" => "send_bincode [send data to process/cluster]".to_owned(),
                "broadcast_bincode" => {
                    "broadcast_bincode [send data to all cluster members]".to_owned()
                }
                "source_iter" => "source_iter [iterate over collection]".to_owned(),
                "source_stream" => "source_stream [receive external data stream]".to_owned(),
                "network(recv)" => "network(recv) [receive from network]".to_owned(),
                "network(send)" => "network(send) [send to network]".to_owned(),
                "dest_sink" => "dest_sink [output destination]".to_owned(),
                _ => {
                    if full_label.len() < 15 {
                        format!("{} [{}]", node_label, "hydro operator")
                    } else {
                        node_label.to_string()
                    }
                }
            }
        } else {
            node_label.to_string()
        };

        // Convert backtrace to JSON if available (optimized for size)
        let backtrace_json = if let Some(bt) = backtrace {
            // Store backtrace for hierarchy generation
            self.node_backtraces.insert(node_id, bt.clone());
            self.optimize_backtrace(bt)
        } else {
            serde_json::json!([])
        };

        // Node type string for styling/legend
        let node_type_str = Self::node_type_to_string(node_type);

        let node = Node {
            id: node_id.to_string(),
            node_type: node_type_str.to_owned(),
            full_label: enhanced_full_label,
            short_label: short_label.clone(),
            // Primary display label follows configuration (defaults to short)
            label: if self.use_short_labels {
                short_label
            } else {
                full_label
            },
            data: NodeData {
                location_key,
                location_type,
                backtrace: backtrace_json,
            },
        };
        self.nodes
            .push(serde_json::to_value(node).expect("Node serialization should not fail"));

        // Track node location for cross-location edge detection
        if let Some(loc_key) = location_key {
            self.node_locations.insert(node_id, loc_key);
        }

        Ok(())
    }

    fn write_edge(
        &mut self,
        src_id: VizNodeKey,
        dst_id: VizNodeKey,
        edge_properties: &HashSet<HydroEdgeProp>,
        label: Option<&str>,
    ) -> Result<(), Self::Err> {
        let edge_id = format!("e{}", self.edge_count);
        self.edge_count = self.edge_count.saturating_add(1);

        // Convert edge properties to semantic tags (string array)
        #[expect(
            clippy::disallowed_methods,
            reason = "nondeterministic iteration order, TODO(mingwei)"
        )]
        let mut semantic_tags: Vec<String> = edge_properties
            .iter()
            .map(|p| Self::edge_type_to_string(*p))
            .collect();

        // Get location information for styling
        let src_loc = self.node_locations.get(src_id).copied();
        let dst_loc = self.node_locations.get(dst_id).copied();

        // Add Network tag if edge crosses locations; otherwise add Local for completeness
        if let (Some(src), Some(dst)) = (src_loc, dst_loc)
            && src != dst
            && !semantic_tags.iter().any(|t| t == "Network")
        {
            semantic_tags.push("Network".to_owned());
        } else if semantic_tags.iter().all(|t| t != "Network") {
            // Only add Local if Network not present (complement for styling)
            semantic_tags.push("Local".to_owned());
        }

        // Ensure deterministic ordering of semantic tags
        semantic_tags.sort();

        let edge = Edge {
            id: edge_id,
            source: src_id.to_string(),
            target: dst_id.to_string(),
            semantic_tags,
            label: label.map(|s| s.to_owned()),
        };

        self.edges
            .push(serde_json::to_value(edge).expect("Edge serialization should not fail"));
        Ok(())
    }

    fn write_location_start(
        &mut self,
        location_key: LocationKey,
        location_type: LocationType,
    ) -> Result<(), Self::Err> {
        let location_label = if let Some(location_name) = self.location_names.get(location_key)
            && "()" != location_name
        // Use default name if the type name is just "()" (unit type)
        {
            format!("{:?} {}", location_type, location_name)
        } else {
            format!("{:?} {:?}", location_type, location_key)
        };
        self.locations
            .insert(location_key, (location_label, Vec::new()));
        Ok(())
    }

    fn write_node(&mut self, node_id: VizNodeKey) -> Result<(), Self::Err> {
        // Find the current location being written and add this node to it
        if let Some((_, node_ids)) = self.locations.values_mut().last() {
            node_ids.push(node_id);
        }
        Ok(())
    }

    fn write_location_end(&mut self) -> Result<(), Self::Err> {
        // Location grouping complete - nothing to do for JSON
        Ok(())
    }

    fn write_epilogue(&mut self) -> Result<(), Self::Err> {
        // Create multiple hierarchy options
        let mut hierarchy_choices = Vec::new();
        let mut node_assignments_choices = serde_json::Map::new();

        // Always add location-based hierarchy
        let (location_hierarchy, location_assignments) = self.create_location_hierarchy();
        hierarchy_choices.push(serde_json::json!({
            "id": "location",
            "name": "Location",
            "children": location_hierarchy
        }));
        node_assignments_choices.insert(
            "location".to_owned(),
            serde_json::Value::Object(location_assignments),
        );

        // Add backtrace-based hierarchy if available
        if self.has_backtrace_data() {
            let (backtrace_hierarchy, backtrace_assignments) = self.create_backtrace_hierarchy();
            hierarchy_choices.push(serde_json::json!({
                "id": "backtrace",
                "name": "Backtrace",
                "children": backtrace_hierarchy
            }));
            node_assignments_choices.insert(
                "backtrace".to_owned(),
                serde_json::Value::Object(backtrace_assignments),
            );
        }

        // Before serialization, enforce deterministic ordering for nodes and edges
        let mut nodes_sorted = self.nodes.clone();
        nodes_sorted.sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));
        let mut edges_sorted = self.edges.clone();
        edges_sorted.sort_by(|a, b| {
            let a_src = a["source"].as_str();
            let b_src = b["source"].as_str();
            match a_src.cmp(&b_src) {
                std::cmp::Ordering::Equal => {
                    let a_dst = a["target"].as_str();
                    let b_dst = b["target"].as_str();
                    match a_dst.cmp(&b_dst) {
                        std::cmp::Ordering::Equal => a["id"].as_str().cmp(&b["id"].as_str()),
                        other => other,
                    }
                }
                other => other,
            }
        });

        // Create the final JSON structure in the format expected by the visualizer
        let node_type_definitions = Self::get_node_type_definitions();
        let legend_items = Self::get_legend_items();

        let node_type_config = serde_json::json!({
            "types": node_type_definitions,
            "defaultType": "Transform"
        });
        let legend = serde_json::json!({
            "title": "Node Types",
            "items": legend_items
        });

        // Determine the selected hierarchy (first one is default)
        let selected_hierarchy = if !hierarchy_choices.is_empty() {
            hierarchy_choices[0]["id"].as_str()
        } else {
            None
        };

        #[derive(serde::Serialize)]
        struct GraphPayload<'a> {
            nodes: Vec<serde_json::Value>,
            edges: Vec<serde_json::Value>,
            #[serde(rename = "hierarchyChoices")]
            hierarchy_choices: &'a [serde_json::Value],
            #[serde(rename = "nodeAssignments")]
            node_assignments: serde_json::Map<String, serde_json::Value>,
            #[serde(rename = "selectedHierarchy", skip_serializing_if = "Option::is_none")]
            selected_hierarchy: Option<&'a str>,
            #[serde(rename = "edgeStyleConfig")]
            edge_style_config: serde_json::Value,
            #[serde(rename = "nodeTypeConfig")]
            node_type_config: serde_json::Value,
            legend: serde_json::Value,
        }

        let payload = GraphPayload {
            nodes: nodes_sorted,
            edges: edges_sorted,
            hierarchy_choices: &hierarchy_choices,
            node_assignments: node_assignments_choices,
            selected_hierarchy,
            edge_style_config: Self::get_edge_style_config(),
            node_type_config,
            legend,
        };

        let final_json = serde_json::to_string_pretty(&payload).unwrap();

        write!(self.write, "{}", final_json)
    }
}

impl<W> HydroJson<'_, W> {
    /// Check if any nodes have meaningful backtrace data
    fn has_backtrace_data(&self) -> bool {
        self.nodes.iter().any(|node| {
            if let Some(backtrace_array) = node["data"]["backtrace"].as_array() {
                // Check if any frame has meaningful filename or fn_name data
                backtrace_array.iter().any(|frame| {
                    let filename = frame["file"].as_str().unwrap_or_default();
                    let fn_name = frame["fn"].as_str().unwrap_or_default();
                    !filename.is_empty() || !fn_name.is_empty()
                })
            } else {
                false
            }
        })
    }

    /// Create location-based hierarchy (original behavior)
    fn create_location_hierarchy(
        &self,
    ) -> (
        Vec<serde_json::Value>,
        serde_json::Map<String, serde_json::Value>,
    ) {
        // Create hierarchy structure (single level: locations as parents, nodes as children)
        let mut locs: Vec<(LocationKey, &(String, Vec<VizNodeKey>))> =
            self.locations.iter().collect();
        locs.sort_by(|a, b| a.0.cmp(&b.0));
        let hierarchy: Vec<serde_json::Value> = locs
            .into_iter()
            .map(|(location_key, (label, _))| {
                serde_json::json!({
                    "key": location_key.to_string(),
                    "name": label,
                    "children": [] // Single level hierarchy - no nested children
                })
            })
            .collect();

        // Create node assignments by reading locationId from each node's data
        // This is more reliable than using the write_node tracking which depends on HashMap iteration order
        // Build and then sort assignments deterministically by node id key
        let mut tmp: Vec<(String, serde_json::Value)> = Vec::new();
        for node in self.nodes.iter() {
            if let (Some(node_id), location_key) =
                (node["id"].as_str(), &node["data"]["locationKey"])
            {
                tmp.push((node_id.to_owned(), location_key.clone()));
            }
        }
        tmp.sort_by(|a, b| a.0.cmp(&b.0));
        let mut node_assignments = serde_json::Map::new();
        for (k, v) in tmp {
            node_assignments.insert(k, v);
        }

        (hierarchy, node_assignments)
    }

    /// Create backtrace-based hierarchy using structured backtrace data
    fn create_backtrace_hierarchy(
        &self,
    ) -> (
        Vec<serde_json::Value>,
        serde_json::Map<String, serde_json::Value>,
    ) {
        use std::collections::HashMap;

        let mut hierarchy_map: HashMap<String, (String, usize, Option<String>)> = HashMap::new(); // path -> (name, depth, parent_path)
        let mut path_to_node_assignments: HashMap<String, Vec<String>> = HashMap::new(); // path -> [node_ids]

        // Process each node's backtrace using the stored backtraces
        for node in self.nodes.iter() {
            if let Some(node_id_str) = node["id"].as_str()
                && let Ok(node_id) = node_id_str.parse::<VizNodeKey>()
                && let Some(backtrace) = self.node_backtraces.get(node_id)
            {
                let elements = backtrace.elements().collect::<Vec<_>>();
                if elements.is_empty() {
                    continue;
                }

                // Do not filter frames for now
                let user_frames = elements;
                if user_frames.is_empty() {
                    continue;
                }

                // Build hierarchy path from backtrace frames (reverse order for call stack)
                let mut hierarchy_path = Vec::new();
                for (i, elem) in user_frames.iter().rev().enumerate() {
                    let label = if i == 0 {
                        if let Some(filename) = &elem.filename {
                            Self::extract_file_path(filename)
                        } else {
                            format!("fn_{}", Self::truncate_function_name(&elem.fn_name))
                        }
                    } else {
                        Self::truncate_function_name(&elem.fn_name).to_owned()
                    };
                    hierarchy_path.push(label);
                }

                // Create hierarchy nodes for this path
                let mut current_path = String::new();
                let mut parent_path: Option<String> = None;
                let mut deepest_path = String::new();
                // Deduplicate consecutive identical labels for cleanliness
                let mut deduped: Vec<String> = Vec::new();
                for seg in hierarchy_path {
                    if deduped.last().map(|s| s == &seg).unwrap_or(false) {
                        continue;
                    }
                    deduped.push(seg);
                }
                for (depth, label) in deduped.iter().enumerate() {
                    current_path = if current_path.is_empty() {
                        label.clone()
                    } else {
                        format!("{}/{}", current_path, label)
                    };
                    if !hierarchy_map.contains_key(&current_path) {
                        hierarchy_map.insert(
                            current_path.clone(),
                            (label.clone(), depth, parent_path.clone()),
                        );
                    }
                    deepest_path = current_path.clone();
                    parent_path = Some(current_path.clone());
                }

                if !deepest_path.is_empty() {
                    path_to_node_assignments
                        .entry(deepest_path)
                        .or_default()
                        .push(node_id_str.to_owned());
                }
            }
        }
        // Build hierarchy tree and create proper ID mapping (deterministic)
        let (mut hierarchy, mut path_to_id_map, id_remapping) =
            self.build_hierarchy_tree_with_ids(&hierarchy_map);

        // Create a root node for nodes without backtraces
        let root_id = "bt_root";
        let mut nodes_without_backtrace = Vec::new();

        // Collect all node IDs
        for node in self.nodes.iter() {
            if let Some(node_id_str) = node["id"].as_str() {
                nodes_without_backtrace.push(node_id_str.to_owned());
            }
        }

        // Remove nodes that already have backtrace assignments
        #[expect(
            clippy::disallowed_methods,
            reason = "nondeterministic iteration order, TODO(mingwei)"
        )]
        for node_ids in path_to_node_assignments.values() {
            for node_id in node_ids {
                nodes_without_backtrace.retain(|id| id != node_id);
            }
        }

        // If there are nodes without backtraces, create a root container for them
        if !nodes_without_backtrace.is_empty() {
            hierarchy.push(serde_json::json!({
                "id": root_id,
                "name": "(no backtrace)",
                "children": []
            }));
            path_to_id_map.insert("__root__".to_owned(), root_id.to_owned());
        }

        // Create node assignments using the actual hierarchy IDs
        let mut node_assignments = serde_json::Map::new();
        let mut pairs: Vec<(String, Vec<String>)> = path_to_node_assignments.into_iter().collect();
        pairs.sort_by(|a, b| a.0.cmp(&b.0));
        for (path, mut node_ids) in pairs {
            node_ids.sort();
            if let Some(hierarchy_id) = path_to_id_map.get(&path) {
                for node_id in node_ids {
                    node_assignments
                        .insert(node_id, serde_json::Value::String(hierarchy_id.clone()));
                }
            }
        }

        // Assign nodes without backtraces to the root
        for node_id in nodes_without_backtrace {
            node_assignments.insert(node_id, serde_json::Value::String(root_id.to_owned()));
        }

        // CRITICAL FIX: Apply ID remapping to node assignments
        // When containers are collapsed, their IDs change, but nodeAssignments still reference old IDs
        // We need to update all assignments to use the new (collapsed) container IDs
        let mut remapped_assignments = serde_json::Map::new();
        for (node_id, container_id_value) in node_assignments.iter() {
            if let Some(container_id) = container_id_value.as_str() {
                // Check if this container ID was remapped during collapsing
                let final_container_id = id_remapping
                    .get(container_id)
                    .map(|s| &**s)
                    .unwrap_or(container_id);
                remapped_assignments.insert(
                    node_id.clone(),
                    serde_json::Value::String(final_container_id.to_owned()),
                );
            }
        }

        (hierarchy, remapped_assignments)
    }

    /// Build a tree structure and return both the tree and path-to-ID mapping
    fn build_hierarchy_tree_with_ids(
        &self,
        hierarchy_map: &HashMap<String, (String, usize, Option<String>)>,
    ) -> (
        Vec<serde_json::Value>,
        HashMap<String, String>,
        HashMap<String, String>,
    ) {
        // Assign IDs deterministically based on sorted path names
        #[expect(
            clippy::disallowed_methods,
            reason = "nondeterministic iteration order, TODO(mingwei)"
        )]
        let mut keys: Vec<&String> = hierarchy_map.keys().collect();
        keys.sort();
        let mut path_to_id: HashMap<String, String> = HashMap::new();
        for (i, path) in keys.iter().enumerate() {
            path_to_id.insert((*path).clone(), format!("bt_{}", i.saturating_add(1)));
        }

        // Find root items (depth 0) and sort by name
        #[expect(
            clippy::disallowed_methods,
            reason = "nondeterministic iteration order, TODO(mingwei)"
        )]
        let mut roots: Vec<(String, String)> = hierarchy_map
            .iter()
            .filter_map(|(path, (name, depth, _))| {
                if *depth == 0 {
                    Some((path.clone(), name.clone()))
                } else {
                    None
                }
            })
            .collect();
        roots.sort_by(|a, b| a.1.cmp(&b.1));
        let mut root_nodes = Vec::new();
        for (path, name) in roots {
            let tree_node = Self::build_tree_node(&path, &name, hierarchy_map, &path_to_id);
            root_nodes.push(tree_node);
        }

        // Apply top-down collapsing of single-child container chains
        // and build a mapping of old IDs to new IDs
        let mut id_remapping: HashMap<String, String> = HashMap::new();
        root_nodes = root_nodes
            .into_iter()
            .map(|node| Self::collapse_single_child_containers(node, None, &mut id_remapping))
            .collect();

        // Update path_to_id with remappings
        let mut updated_path_to_id = path_to_id.clone();
        #[expect(
            clippy::disallowed_methods,
            reason = "nondeterministic iteration order, TODO(mingwei)"
        )]
        for (path, old_id) in path_to_id.iter() {
            if let Some(new_id) = id_remapping.get(old_id) {
                updated_path_to_id.insert(path.clone(), new_id.clone());
            }
        }

        (root_nodes, updated_path_to_id, id_remapping)
    }

    /// Build a single tree node recursively
    fn build_tree_node(
        current_path: &str,
        name: &str,
        hierarchy_map: &HashMap<String, (String, usize, Option<String>)>,
        path_to_id: &HashMap<String, String>,
    ) -> serde_json::Value {
        let current_id = path_to_id.get(current_path).unwrap().clone();

        // Find children (paths that have this path as parent)
        #[expect(
            clippy::disallowed_methods,
            reason = "nondeterministic iteration order, TODO(mingwei)"
        )]
        let mut child_specs: Vec<(&String, &String)> = hierarchy_map
            .iter()
            .filter_map(|(child_path, (child_name, _, parent_path))| {
                if let Some(parent) = parent_path {
                    if parent == current_path {
                        Some((child_path, child_name))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();
        child_specs.sort_by(|a, b| a.1.cmp(b.1));
        let mut children = Vec::new();
        for (child_path, child_name) in child_specs {
            let child_node =
                Self::build_tree_node(child_path, child_name, hierarchy_map, path_to_id);
            children.push(child_node);
        }

        if children.is_empty() {
            serde_json::json!({
                "id": current_id,
                "name": name
            })
        } else {
            serde_json::json!({
                "id": current_id,
                "name": name,
                "children": children
            })
        }
    }

    /// Collapse single-child container chains (top-down)
    /// When a container has exactly one child AND that child is also a container,
    /// we collapse them by keeping the child's ID and combining names.
    /// parent_name is used to accumulate names during recursion (None for roots)
    /// id_remapping tracks which old IDs map to which new IDs after collapsing
    fn collapse_single_child_containers(
        node: serde_json::Value,
        parent_name: Option<&str>,
        id_remapping: &mut HashMap<String, String>,
    ) -> serde_json::Value {
        let mut node_obj = match node {
            serde_json::Value::Object(obj) => obj,
            _ => return node,
        };

        let current_name = node_obj
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or_default();

        let current_id = node_obj
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or_default();

        // Determine the effective name (combined with parent if collapsing)
        // Use → to show call chain (parent called child)
        let effective_name = if let Some(parent) = parent_name {
            format!("{} → {}", parent, current_name)
        } else {
            current_name.to_owned()
        };

        // Check if this node has children (is a container)
        if let Some(serde_json::Value::Array(children)) = node_obj.get("children") {
            // If exactly one child AND that child is also a container
            if children.len() == 1
                && let Some(child) = children.first()
            {
                let child_is_container = child
                    .get("children")
                    .and_then(|v| v.as_array())
                    .is_some_and(|arr| !arr.is_empty());

                if child_is_container {
                    let child_id = child.get("id").and_then(|v| v.as_str()).unwrap_or_default();

                    // Record that this parent's ID should map to the child's ID
                    if !current_id.is_empty() && !child_id.is_empty() {
                        id_remapping.insert(current_id.to_owned(), child_id.to_owned());
                    }

                    // Collapse: recursively process the child with accumulated name
                    return Self::collapse_single_child_containers(
                        child.clone(),
                        Some(&effective_name),
                        id_remapping,
                    );
                }
            }

            // Not collapsing: process children normally and update name if accumulated
            let processed_children: Vec<serde_json::Value> = children
                .iter()
                .map(|child| {
                    Self::collapse_single_child_containers(child.clone(), None, id_remapping)
                })
                .collect();

            node_obj.insert("name".to_owned(), serde_json::Value::String(effective_name));
            node_obj.insert(
                "children".to_owned(),
                serde_json::Value::Array(processed_children),
            );
        } else {
            // Leaf node: just update name if accumulated
            node_obj.insert("name".to_owned(), serde_json::Value::String(effective_name));
        }

        serde_json::Value::Object(node_obj)
    }

    /// Extract meaningful file path
    fn extract_file_path(filename: &str) -> String {
        if filename.is_empty() {
            return "unknown".to_owned();
        }

        // Extract the most relevant part of the file path
        let parts: Vec<&str> = filename.split('/').collect();
        let file_name = parts.last().unwrap_or(&"unknown");

        // If it's a source file, include the parent directory for context
        if file_name.ends_with(".rs") && parts.len() > 1 {
            let parent_dir = parts[parts.len() - 2];
            format!("{}/{}", parent_dir, file_name)
        } else {
            file_name.to_string()
        }
    }
}

/// Create JSON from Hydro IR with type names
pub fn hydro_ir_to_json(
    ir: &[HydroRoot],
    location_names: &SecondaryMap<LocationKey, String>,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut output = String::new();

    let config = HydroWriteConfig {
        show_metadata: false,
        show_location_groups: true,
        use_short_labels: true, // Default to short labels
        location_names,
    };

    write_hydro_ir_json(&mut output, ir, config)?;

    Ok(output)
}

/// Open JSON visualization in browser using the docs visualizer with URL-encoded data
pub fn open_json_browser(
    ir: &[HydroRoot],
    location_names: &SecondaryMap<LocationKey, String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = HydroWriteConfig {
        location_names,
        ..Default::default()
    };

    super::debug::open_json_visualizer(ir, Some(config))
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
}

/// Save JSON to file using the consolidated debug utilities
pub fn save_json(
    ir: &[HydroRoot],
    location_names: &SecondaryMap<LocationKey, String>,
    filename: &str,
) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let config = HydroWriteConfig {
        location_names,
        ..Default::default()
    };

    super::debug::save_json(ir, Some(filename), Some(config))
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
}

/// Open JSON visualization in browser for a BuiltFlow
#[cfg(feature = "build")]
pub fn open_browser(
    built_flow: &crate::compile::built::BuiltFlow,
) -> Result<(), Box<dyn std::error::Error>> {
    open_json_browser(built_flow.ir(), built_flow.location_names())
}
