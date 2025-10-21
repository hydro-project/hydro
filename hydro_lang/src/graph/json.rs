use std::collections::{HashMap, HashSet};
use std::fmt::Write;

use super::render::{HydroEdgeType, HydroGraphWrite, HydroNodeType};
use crate::compile::ir::HydroRoot;
use crate::compile::ir::backtrace::Backtrace;

/// JSON graph writer for Hydro IR.
/// Outputs JSON that can be used with interactive graph visualization tools.
pub struct HydroJson<W> {
    write: W,
    nodes: Vec<serde_json::Value>,
    edges: Vec<serde_json::Value>,
    locations: HashMap<usize, (String, Vec<usize>)>, // location_id -> (label, node_ids)
    node_locations: HashMap<usize, usize>,           // node_id -> location_id
    edge_count: usize,
    // Type name mappings
    process_names: HashMap<usize, String>,
    cluster_names: HashMap<usize, String>,
    external_names: HashMap<usize, String>,
    // Store backtraces for hierarchy generation
    node_backtraces: HashMap<usize, Backtrace>,
    // Config flags
    use_short_labels: bool,
}

impl<W> HydroJson<W> {
    pub fn new(write: W, config: &super::render::HydroWriteConfig) -> Self {
        let process_names: HashMap<usize, String> =
            config.process_id_name.iter().cloned().collect();
        let cluster_names: HashMap<usize, String> =
            config.cluster_id_name.iter().cloned().collect();
        let external_names: HashMap<usize, String> =
            config.external_id_name.iter().cloned().collect();

        Self {
            write,
            nodes: Vec::new(),
            edges: Vec::new(),
            locations: HashMap::new(),
            node_locations: HashMap::new(),
            edge_count: 0,
            process_names,
            cluster_names,
            external_names,
            node_backtraces: HashMap::new(),
            use_short_labels: config.use_short_labels,
        }
    }

    /// Convert HydroNodeType to string representation
    fn node_type_to_string(node_type: HydroNodeType) -> &'static str {
        super::render::node_type_utils::to_string(node_type)
    }

    /// Convert HydroEdgeType to string representation for semantic tags
    fn edge_type_to_string(edge_type: HydroEdgeType) -> String {
        match edge_type {
            HydroEdgeType::Bounded => "Bounded".to_string(),
            HydroEdgeType::Unbounded => "Unbounded".to_string(),
            HydroEdgeType::TotalOrder => "TotalOrder".to_string(),
            HydroEdgeType::NoOrder => "NoOrder".to_string(),
            HydroEdgeType::Keyed => "Keyed".to_string(),
            HydroEdgeType::Stream => "Stream".to_string(),
            HydroEdgeType::KeyedStream => "KeyedStream".to_string(),
            HydroEdgeType::Singleton => "Singleton".to_string(),
            HydroEdgeType::Optional => "Optional".to_string(),
            HydroEdgeType::Network => "Network".to_string(),
            HydroEdgeType::Cycle => "Cycle".to_string(),
            HydroEdgeType::Persistent => "Persistent".to_string(),
        }
    }

    /// Get all node type definitions for JSON output
    fn get_node_type_definitions() -> Vec<serde_json::Value> {
        // Ensure deterministic ordering by sorting by type string
        let mut types: Vec<(usize, &'static str)> = super::render::node_type_utils::all_types_with_strings()
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

                // Flow group - controls flow direction or persistence
                "FlowGroup": {
                    "Transient": {
                        "persistence": "none"
                    },
                    "Persistent": {
                        "persistence": "stored"
                    }
                }
            },
            "note": "Edge styles are now computed per-edge using the unified edge style system. This config is provided for reference and compatibility."
        })
    }

    /// Optimize backtrace data for size efficiency
    /// 1. Remove redundant/non-essential frames
    /// 2. Truncate paths
    /// 3. Remove memory addresses (not useful for visualization)
    /// 4. Limit frame count for size efficiency
    fn optimize_backtrace(&self, backtrace: &Backtrace) -> serde_json::Value {
        #[cfg(feature = "build")]
        {
            let elements = backtrace.elements();

            // filter out obviously internal frames
            let relevant_frames: Vec<_> = elements
                .iter()
                .filter(|elem| {
                    let filename = elem.filename.as_deref().unwrap_or("");
                    let fn_name = &elem.fn_name;

                    // Filter out obviously internal/system frames
                    !(filename.contains(".cargo/registry/")
                        || filename.contains(".rustup/toolchains/")
                        || fn_name.starts_with("std::")
                        || fn_name.starts_with("core::")
                        || fn_name.starts_with("alloc::")
                        || fn_name.contains("rust_begin_unwind")
                        || fn_name.contains("rust_panic")
                        || fn_name.contains("__rust_")
                        || filename.ends_with("panic.rs")
                        || filename.ends_with("/rustc/"))
                })
                .map(|elem| {
                    // Truncate paths and function names for size
                    let short_filename = elem
                        .filename
                        .as_deref()
                        .map(|f| Self::truncate_path(f))
                        .unwrap_or_else(|| "unknown".to_string());

                    let short_fn_name = Self::truncate_function_name(&elem.fn_name);

                    serde_json::json!({
                        "fn": short_fn_name,
                        "file": short_filename,
                        "line": elem.lineno
                        // Removed "addr" - not useful for visualization and saves space
                    })
                })
                .collect();

            serde_json::Value::Array(relevant_frames)
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
            path.to_string()
        }
    }

    /// Truncate function names to remove module paths
    fn truncate_function_name(fn_name: &str) -> String {
        // Remove everything before the last "::" to get just the function name
        fn_name.split("::").last().unwrap_or(fn_name).to_string()
    }
}

impl<W> HydroGraphWrite for HydroJson<W>
where
    W: Write,
{
    type Err = super::render::GraphWriteError;

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
        node_id: usize,
        node_label: &super::render::NodeLabel,
        node_type: HydroNodeType,
        location_id: Option<usize>,
        location_type: Option<&str>,
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
        let enhanced_full_label = if short_label.len() >= full_label.len() - 2 {
            // If they're nearly the same length, add more context to full label
            match short_label.as_str() {
                "inspect" => "inspect [debug output]".to_string(),
                "persist" => "persist [state storage]".to_string(),
                "tee" => "tee [branch dataflow]".to_string(),
                "delta" => "delta [change detection]".to_string(),
                "spin" => "spin [delay/buffer]".to_string(),
                "send_bincode" => "send_bincode [send data to process/cluster]".to_string(),
                "broadcast_bincode" => {
                    "broadcast_bincode [send data to all cluster members]".to_string()
                }
                "source_iter" => "source_iter [iterate over collection]".to_string(),
                "source_stream" => "source_stream [receive external data stream]".to_string(),
                "network(recv)" => "network(recv) [receive from network]".to_string(),
                "network(send)" => "network(send) [send to network]".to_string(),
                "dest_sink" => "dest_sink [output destination]".to_string(),
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

        // Add semantic tags for nodes (node type as semantic tag)
        let node_type_str = Self::node_type_to_string(node_type);
        let semantic_tags = vec![node_type_str];

        let node = serde_json::json!({
            "id": node_id.to_string(),
            "nodeType": node_type_str,
            "fullLabel": enhanced_full_label,
            "shortLabel": short_label,
            // Primary display label follows configuration (defaults to short)
            "label": if self.use_short_labels { serde_json::Value::String(super::render::extract_short_label(&full_label)) } else { serde_json::Value::String(full_label) },
            "semanticTags": semantic_tags,
            "data": {
                "locationId": location_id,
                "locationType": location_type,
                "backtrace": backtrace_json
            }
        });
        self.nodes.push(node);

        // Track node location for cross-location edge detection
        if let Some(loc_id) = location_id {
            self.node_locations.insert(node_id, loc_id);
        }

        Ok(())
    }

    fn write_edge(
        &mut self,
        src_id: usize,
        dst_id: usize,
        edge_properties: &HashSet<HydroEdgeType>,
        label: Option<&str>,
    ) -> Result<(), Self::Err> {
        let edge_id = format!("e{}", self.edge_count);
        self.edge_count += 1;

        // Convert edge properties to semantic tags (string array)
        let mut semantic_tags: Vec<String> = edge_properties
            .iter()
            .map(|p| Self::edge_type_to_string(*p))
            .collect();

        // Get location information for styling
        let src_loc = self.node_locations.get(&src_id).copied();
        let dst_loc = self.node_locations.get(&dst_id).copied();

        // Add Network tag if edge crosses locations; otherwise add Local for completeness
        if let (Some(src), Some(dst)) = (src_loc, dst_loc)
            && src != dst
            && !semantic_tags.contains(&"Network".to_string())
        {
            semantic_tags.push("Network".to_string());
        } else if semantic_tags.iter().all(|t| t != "Network") {
            // Only add Local if Network not present (complement for styling)
            semantic_tags.push("Local".to_string());
        }

        // Ensure deterministic ordering of semantic tags
        semantic_tags.sort();

        let mut edge = serde_json::json!({
            "id": edge_id,
            "source": src_id.to_string(),
            "target": dst_id.to_string(),
            "semanticTags": semantic_tags,
        });

        if let Some(label_text) = label {
            edge["label"] = serde_json::Value::String(label_text.to_string());
        }

        self.edges.push(edge);
        Ok(())
    }

    fn write_location_start(
        &mut self,
        location_id: usize,
        location_type: &str,
    ) -> Result<(), Self::Err> {
        let location_label = match location_type {
            "Process" => {
                if let Some(name) = self.process_names.get(&location_id) {
                    // Use default name if the type name is just "()" (unit type)
                    if name == "()" {
                        format!("Process {}", location_id)
                    } else {
                        name.clone()
                    }
                } else {
                    format!("Process {}", location_id)
                }
            }
            "Cluster" => {
                if let Some(name) = self.cluster_names.get(&location_id) {
                    name.clone()
                } else {
                    format!("Cluster {}", location_id)
                }
            }
            "External" => {
                if let Some(name) = self.external_names.get(&location_id) {
                    name.clone()
                } else {
                    format!("External {}", location_id)
                }
            }
            _ => location_type.to_string(),
        };

        self.locations
            .insert(location_id, (location_label, Vec::new()));
        Ok(())
    }

    fn write_node(&mut self, node_id: usize) -> Result<(), Self::Err> {
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
            "location".to_string(),
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
                "backtrace".to_string(),
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

        // Build JSON string manually to guarantee field ordering
        let mut json_parts = Vec::new();

        // 1. nodes (required field first)
        json_parts.push(format!(
            "\"nodes\": {}",
            serde_json::to_string_pretty(&nodes_sorted).unwrap()
        ));

        // 2. edges (required field second)
        json_parts.push(format!(
            "\"edges\": {}",
            serde_json::to_string_pretty(&edges_sorted).unwrap()
        ));

        // 3. hierarchyChoices
        json_parts.push(format!(
            "\"hierarchyChoices\": {}",
            serde_json::to_string_pretty(&hierarchy_choices).unwrap()
        ));

        // 4. nodeAssignments
        json_parts.push(format!(
            "\"nodeAssignments\": {}",
            serde_json::to_string_pretty(&node_assignments_choices).unwrap()
        ));

        // 5. edgeStyleConfig
        json_parts.push(format!(
            "\"edgeStyleConfig\": {}",
            serde_json::to_string_pretty(&Self::get_edge_style_config()).unwrap()
        ));

        // 6. nodeTypeConfig
        let node_type_config = serde_json::json!({
            "types": node_type_definitions,
            "defaultType": "Transform"
        });
        json_parts.push(format!(
            "\"nodeTypeConfig\": {}",
            serde_json::to_string_pretty(&node_type_config).unwrap()
        ));

        // 7. legend
        let legend = serde_json::json!({
            "title": "Node Types",
            "items": legend_items
        });
        json_parts.push(format!(
            "\"legend\": {}",
            serde_json::to_string_pretty(&legend).unwrap()
        ));

        let final_json = format!("{{\n  {}\n}}", json_parts.join(",\n  "));

        write!(self.write, "{}", final_json)
    }
}

impl<W> HydroJson<W> {
    /// Check if any nodes have meaningful backtrace data
    fn has_backtrace_data(&self) -> bool {
        self.nodes.iter().any(|node| {
            if let Some(backtrace_array) = node["data"]["backtrace"].as_array() {
                // Check if any frame has meaningful filename or fn_name data
                backtrace_array.iter().any(|frame| {
                    let filename = frame["file"].as_str().unwrap_or("");
                    let fn_name = frame["fn"].as_str().unwrap_or("");
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
        let mut locs: Vec<(&usize, &(String, Vec<usize>))> = self.locations.iter().collect();
        locs.sort_by(|a, b| a.0.cmp(b.0));
        let hierarchy: Vec<serde_json::Value> = locs
            .into_iter()
            .map(|(location_id, (label, _))| {
                serde_json::json!({
                    "id": format!("loc_{}", location_id),
                    "name": label,
                    "children": [] // Single level hierarchy - no nested children
                })
            })
            .collect();

        // Create node assignments by reading locationId from each node's data
        // This is more reliable than using the write_node tracking which depends on HashMap iteration order
        // Build and then sort assignments deterministically by node id key
        let mut tmp: Vec<(String, String)> = Vec::new();
        for node in &self.nodes {
            if let (Some(node_id), Some(location_id)) =
                (node["id"].as_str(), node["data"]["locationId"].as_u64())
            {
                let location_key = format!("loc_{}", location_id);
                tmp.push((node_id.to_string(), location_key));
            }
        }
        tmp.sort_by(|a, b| a.0.cmp(&b.0));
        let mut node_assignments = serde_json::Map::new();
        for (k, v) in tmp {
            node_assignments.insert(k, serde_json::Value::String(v));
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
        for node in &self.nodes {
            if let Some(node_id_str) = node["id"].as_str()
                && let Ok(node_id) = node_id_str.parse::<usize>()
                && let Some(backtrace) = self.node_backtraces.get(&node_id)
            {
                let elements = backtrace.elements();
                if elements.is_empty() {
                    continue;
                }

                // Filter to user-relevant frames using structured data
                let user_frames: Vec<_> = elements
                    .iter()
                    .filter(|elem| {
                        let filename = elem.filename.as_deref().unwrap_or("");
                        let fn_name = &elem.fn_name;
                        // Include frames that are from user code (more precise filtering)
                        filename.contains("hydro_test")
                            || filename.contains("/src/")
                            || (filename.contains("examples/") && !filename.contains(".cargo/"))
                            || (!filename.contains(".cargo/registry/")
                                && !filename.contains(".rustup/toolchains/")
                                && !fn_name.starts_with("std::")
                                && !fn_name.starts_with("core::")
                                && !fn_name.contains("tokio::"))
                    })
                    .take(5)
                    .collect();
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
                        Self::truncate_function_name(&elem.fn_name)
                    };
                    hierarchy_path.push(label);
                }

                // Create hierarchy nodes for this path
                let mut current_path = String::new();
                let mut parent_path: Option<String> = None;
                let mut deepest_path = String::new();
                // Deduplicate consecutive identical labels for cleanliness
                let mut path_iter = hierarchy_path.into_iter().peekable();
                let mut deduped: Vec<String> = Vec::new();
                while let Some(seg) = path_iter.next() {
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
                        .push(node_id_str.to_string());
                }
            }
        }
        // Build hierarchy tree and create proper ID mapping
        // Build hierarchy tree and create proper ID mapping (deterministic)
        let (mut hierarchy, mut path_to_id_map) =
            self.build_hierarchy_tree_with_ids(&hierarchy_map);

        // Create a root node for nodes without backtraces
        let root_id = "bt_root".to_string();
        let mut nodes_without_backtrace = Vec::new();

        // Collect all node IDs
        for node in &self.nodes {
            if let Some(node_id_str) = node["id"].as_str() {
                nodes_without_backtrace.push(node_id_str.to_string());
            }
        }

        // Remove nodes that already have backtrace assignments
        for node_ids in path_to_node_assignments.values() {
            for node_id in node_ids {
                nodes_without_backtrace.retain(|id| id != node_id);
            }
        }

        // If there are nodes without backtraces, create a root container for them
        if !nodes_without_backtrace.is_empty() {
            hierarchy.push(serde_json::json!({
                "id": root_id.clone(),
                "name": "(no backtrace)",
                "children": []
            }));
            path_to_id_map.insert("__root__".to_string(), root_id.clone());
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
            node_assignments.insert(node_id, serde_json::Value::String(root_id.clone()));
        }

        (hierarchy, node_assignments)
    }

    /// Build a tree structure and return both the tree and path-to-ID mapping
    fn build_hierarchy_tree_with_ids(
        &self,
        hierarchy_map: &HashMap<String, (String, usize, Option<String>)>,
    ) -> (Vec<serde_json::Value>, HashMap<String, String>) {
        // Assign IDs deterministically based on sorted path names
        let mut keys: Vec<&String> = hierarchy_map.keys().collect();
        keys.sort();
        let mut path_to_id: HashMap<String, String> = HashMap::new();
        for (i, path) in keys.iter().enumerate() {
            path_to_id.insert((***path).to_string(), format!("bt_{}", i + 1));
        }

        // Find root items (depth 0) and sort by name
        let mut roots: Vec<(String, String)> = hierarchy_map
            .iter()
            .filter_map(|(path, (name, depth, _))| if *depth == 0 { Some((path.clone(), name.clone())) } else { None })
            .collect();
        roots.sort_by(|a, b| a.1.cmp(&b.1));
        let mut root_nodes = Vec::new();
        for (path, name) in roots {
            let tree_node = Self::build_tree_node(&path, &name, hierarchy_map, &path_to_id);
            root_nodes.push(tree_node);
        }

        (root_nodes, path_to_id)
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
        let mut child_specs: Vec<(&String, &String)> = hierarchy_map
            .iter()
            .filter_map(|(child_path, (child_name, _, parent_path))| {
                if let Some(parent) = parent_path {
                    if parent == current_path { Some((child_path, child_name)) } else { None }
                } else { None }
            })
            .collect();
        child_specs.sort_by(|a, b| a.1.cmp(b.1));
        let mut children = Vec::new();
        for (child_path, child_name) in child_specs {
            let child_node = Self::build_tree_node(child_path, child_name, hierarchy_map, path_to_id);
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

    /// Extract meaningful file path
    fn extract_file_path(filename: &str) -> String {
        if filename.is_empty() {
            return "unknown".to_string();
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
    process_names: Vec<(usize, String)>,
    cluster_names: Vec<(usize, String)>,
    external_names: Vec<(usize, String)>,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut output = String::new();

    let config = super::render::HydroWriteConfig {
        show_metadata: false,
        show_location_groups: true,
        use_short_labels: true, // Default to short labels
        process_id_name: process_names,
        cluster_id_name: cluster_names,
        external_id_name: external_names,
    };

    super::render::write_hydro_ir_json(&mut output, ir, &config)?;

    Ok(output)
}

/// Open JSON visualization in browser using the docs visualizer with URL-encoded data
#[cfg(feature = "viz")]
pub fn open_json_browser(
    ir: &[HydroRoot],
    process_names: Vec<(usize, String)>,
    cluster_names: Vec<(usize, String)>,
    external_names: Vec<(usize, String)>,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = super::render::HydroWriteConfig {
        process_id_name: process_names,
        cluster_id_name: cluster_names,
        external_id_name: external_names,
        ..Default::default()
    };

    super::debug::open_json_visualizer(ir, Some(config))
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
}

/// Save JSON to file using the consolidated debug utilities
pub fn save_json(
    ir: &[HydroRoot],
    process_names: Vec<(usize, String)>,
    cluster_names: Vec<(usize, String)>,
    external_names: Vec<(usize, String)>,
    filename: &str,
) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let config = super::render::HydroWriteConfig {
        process_id_name: process_names,
        cluster_id_name: cluster_names,
        external_id_name: external_names,
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
    open_json_browser(
        built_flow.ir(),
        built_flow.process_id_name().clone(),
        built_flow.cluster_id_name().clone(),
        built_flow.external_id_name().clone(),
    )
}
