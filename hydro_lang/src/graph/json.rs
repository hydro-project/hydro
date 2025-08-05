use std::collections::HashMap;
use std::fmt::Write;

use serde_json;

use super::render::{HydroEdgeType, HydroGraphWrite, HydroNodeType};

/// JSON graph writer for Hydro IR.
/// Outputs JSON that can be used with interactive graph visualization tools.
pub struct HydroJson<W> {
    write: W,
    nodes: Vec<serde_json::Value>,
    edges: Vec<serde_json::Value>,
    locations: HashMap<usize, (String, Vec<usize>)>, // location_id -> (label, node_ids)
    node_locations: HashMap<usize, usize>, // node_id -> location_id
    edge_count: usize,
    config: super::render::HydroWriteConfig,
    // Type name mappings
    process_names: HashMap<usize, String>,
    cluster_names: HashMap<usize, String>,
    external_names: HashMap<usize, String>,
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
            config: config.clone(),
            process_names,
            cluster_names,
            external_names,
        }
    }

    /// Convert HydroNodeType to string representation
    fn node_type_to_string(node_type: HydroNodeType) -> &'static str {
        super::render::node_type_utils::to_string(node_type)
    }

    /// Get all node type definitions for JSON output
    fn get_node_type_definitions() -> Vec<serde_json::Value> {
        super::render::node_type_utils::all_types_with_strings()
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
            .map(|def| serde_json::json!({
                "type": def["id"],
                "label": def["label"]
            }))
            .collect()
    }

    fn node_type_to_style(&self, _node_type: HydroNodeType) -> serde_json::Value {
        // No styling in backend - let the visualizer handle all presentation
        serde_json::json!({})
    }
    fn edge_type_to_style(&self, edge_type: HydroEdgeType) -> serde_json::Value {
        // Minimal styling - let the visualizer handle presentation
        // Only include essential behavior hints for the frontend
        let mut style = serde_json::json!({});

        // Apply type-specific behavior hints only (not colors/styling)
        match edge_type {
            HydroEdgeType::Network => {
                // Network edges should be animated and dashed to show cross-location communication
                style["animated"] = serde_json::Value::Bool(true);
                style["isDashed"] = serde_json::Value::Bool(true);
            }
            HydroEdgeType::Cycle => {
                // Cycle edges should be animated to show feedback loops
                style["animated"] = serde_json::Value::Bool(true);
            }
            _ => {
                // All other edge types use default visualizer styling
            }
        }

        style
    }

    /// Apply elk.js layout via browser - nodes start at origin for elk.js to position
    fn apply_layout(&mut self) {
        // Set all nodes to default position - elk.js will handle layout in browser
        for node in &mut self.nodes {
            node["position"]["x"] = serde_json::Value::Number(serde_json::Number::from(0));
            node["position"]["y"] = serde_json::Value::Number(serde_json::Number::from(0));
        }
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
        backtrace: Option<&crate::backtrace::Backtrace>,
    ) -> Result<(), Self::Err> {
        let style = self.node_type_to_style(node_type);

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

        // Determine what label to display based on config
        let display_label = if self.config.use_short_labels {
            super::render::extract_short_label(&full_label)
        } else {
            full_label.clone()
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

        // Convert backtrace to JSON if available
        let backtrace_json = if let Some(bt) = backtrace {
            #[cfg(feature = "build")]
            {
                let elements = bt.elements();
                serde_json::json!(elements.into_iter().map(|elem| {
                    serde_json::json!({
                        "fn_name": elem.fn_name,
                        "filename": elem.filename,
                        "lineno": elem.lineno,
                        "addr": elem.addr
                    })
                }).collect::<Vec<_>>())
            }
            #[cfg(not(feature = "build"))]
            {
                serde_json::json!([])
            }
        } else {
            serde_json::json!([])
        };

        let node = serde_json::json!({
            "id": node_id.to_string(),
            "type": "default",
            "data": {
                "label": display_label,
                "shortLabel": short_label,
                "fullLabel": enhanced_full_label,
                "expanded": false,
                "locationId": location_id,
                "locationType": location_type,
                "nodeType": Self::node_type_to_string(node_type),
                "backtrace": backtrace_json
            },
            "position": {
                "x": 0,
                "y": 0
            },
            "style": style
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
        edge_type: HydroEdgeType,
        label: Option<&str>,
    ) -> Result<(), Self::Err> {
        let mut style = self.edge_type_to_style(edge_type);
        let edge_id = format!("e{}", self.edge_count);
        self.edge_count += 1;

        // Check if this edge crosses location boundaries
        let is_cross_location = if let (Some(src_location), Some(dst_location)) = 
            (self.node_locations.get(&src_id), self.node_locations.get(&dst_id)) {
            src_location != dst_location
        } else {
            false
        };

        // Mark cross-location edges with special styling
        if is_cross_location {
            style["animated"] = serde_json::Value::Bool(true);
            style["isDashed"] = serde_json::Value::Bool(true);
        }

        let mut edge = serde_json::json!({
            "id": edge_id,
            "source": src_id.to_string(),
            "target": dst_id.to_string(),
            "style": style
        });

        // Add animation for certain edge types or cross-location edges
        if matches!(edge_type, HydroEdgeType::Network | HydroEdgeType::Cycle) || is_cross_location {
            if let Some(style_obj) = edge["style"].as_object_mut() {
                style_obj.insert("animated".to_string(), serde_json::Value::Bool(true));
            }
        }

        if let Some(label_text) = label {
            edge["label"] = serde_json::Value::String(label_text.to_string());
            // Remove label styling - let the visualizer handle it
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
                    name.clone()
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
        // Apply automatic layout using a simple algorithm
        self.apply_layout();

        // Create multiple hierarchy options
        let mut hierarchy_choices = Vec::new();
        let mut node_assignments_choices = serde_json::Map::new();

        // Always add location-based hierarchy
        let (location_hierarchy, location_assignments) = self.create_location_hierarchy();
        hierarchy_choices.push(serde_json::json!({
            "id": "location",
            "name": "Location",
            "hierarchy": location_hierarchy
        }));
        node_assignments_choices.insert("location".to_string(), serde_json::Value::Object(location_assignments));

        // Add backtrace-based hierarchy if available
        if self.has_backtrace_data() {
            let (backtrace_hierarchy, backtrace_assignments) = self.create_backtrace_hierarchy();
            hierarchy_choices.push(serde_json::json!({
                "id": "backtrace",
                "name": "Backtrace",
                "hierarchy": backtrace_hierarchy
            }));
            node_assignments_choices.insert("backtrace".to_string(), serde_json::Value::Object(backtrace_assignments));
        }

        // Create the final JSON structure in the format expected by the visualizer
        let node_type_definitions = Self::get_node_type_definitions();
        let legend_items = Self::get_legend_items();

        let output = serde_json::json!({
            "nodes": self.nodes,
            "edges": self.edges,
            "hierarchyChoices": hierarchy_choices,
            "nodeAssignments": node_assignments_choices,
            "nodeTypeConfig": {
                "types": node_type_definitions,
                "defaultType": "Transform"
            },
            "legend": {
                "title": "Node Types",
                "items": legend_items
            }
        });

        write!(
            self.write,
            "{}",
            serde_json::to_string_pretty(&output).unwrap()
        )
    }
}

impl<W> HydroJson<W> {
    /// Check if any nodes have backtrace data
    fn has_backtrace_data(&self) -> bool {
        self.nodes.iter().any(|node| {
            node["data"]["backtrace"]
                .as_array()
                .is_some_and(|bt| !bt.is_empty())
        })
    }

    /// Create location-based hierarchy (original behavior)
    fn create_location_hierarchy(&self) -> (Vec<serde_json::Value>, serde_json::Map<String, serde_json::Value>) {
        // Create hierarchy structure (single level: locations as parents, nodes as children)
        let hierarchy: Vec<serde_json::Value> = self
            .locations
            .iter()
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
        let mut node_assignments = serde_json::Map::new();
        for node in &self.nodes {
            if let (Some(node_id), Some(location_id)) = (
                node["id"].as_str(),
                node["data"]["locationId"].as_u64(),
            ) {
                let location_key = format!("loc_{}", location_id);
                node_assignments.insert(node_id.to_string(), serde_json::Value::String(location_key));
            }
        }

        (hierarchy, node_assignments)
    }

    /// Create backtrace-based hierarchy
    fn create_backtrace_hierarchy(&self) -> (Vec<serde_json::Value>, serde_json::Map<String, serde_json::Value>) {
        use std::collections::HashMap;

        let mut hierarchy_map: HashMap<String, (String, usize, Option<String>)> = HashMap::new(); // path -> (name, depth, parent_path)
        let mut path_to_node_assignments: HashMap<String, Vec<String>> = HashMap::new(); // path -> [node_ids]

        // Process each node's backtrace
        for node in &self.nodes {
            if let (Some(node_id), Some(backtrace_array)) = (
                node["id"].as_str(),
                node["data"]["backtrace"].as_array(),
            ) {
                if backtrace_array.is_empty() {
                    continue;
                }

                // Extract user-relevant frames from backtrace
                let user_frames: Vec<_> = backtrace_array
                    .iter()
                    .filter_map(|frame| {
                        let filename = frame["filename"].as_str().unwrap_or("");
                        let fn_name = frame["fn_name"].as_str().unwrap_or("");
                        
                        // Include frames that are from user code
                        if filename.contains("hydro_test") 
                            || filename.contains("src/")
                            || (!filename.contains(".cargo/") 
                                && !filename.contains(".rustup/")
                                && !fn_name.contains("tokio")) {
                            Some((filename, fn_name))
                        } else {
                            None
                        }
                    })
                    .take(3) // Take top 3 user frames
                    .collect();

                if user_frames.is_empty() {
                    continue;
                }

                // Build hierarchy path from backtrace frames (reverse order for call stack)
                let mut hierarchy_path = Vec::new();
                for (i, (filename, fn_name)) in user_frames.iter().rev().enumerate() {
                    let label = if i == 0 {
                        // Top level: show file
                        Self::extract_file_path(filename)
                    } else {
                        // Function levels: show function name
                        Self::extract_function_name(fn_name)
                    };
                    hierarchy_path.push(label);
                }

                // Create hierarchy nodes for this path
                let mut current_path = String::new();
                let mut parent_path: Option<String> = None;
                let mut deepest_path = String::new();

                for (depth, label) in hierarchy_path.iter().enumerate() {
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

                // Assign node to the deepest hierarchy level
                if !deepest_path.is_empty() {
                    path_to_node_assignments
                        .entry(deepest_path)
                        .or_default()
                        .push(node_id.to_string());
                }
            }
        }

        // Build hierarchy tree and create proper ID mapping
        let (hierarchy, path_to_id_map) = self.build_hierarchy_tree_with_ids(&hierarchy_map);
        
        // Create node assignments using the actual hierarchy IDs
        let mut node_assignments = serde_json::Map::new();
        for (path, node_ids) in path_to_node_assignments {
            if let Some(hierarchy_id) = path_to_id_map.get(&path) {
                for node_id in node_ids {
                    node_assignments.insert(node_id, serde_json::Value::String(hierarchy_id.clone()));
                }
            }
        }
        
        (hierarchy, node_assignments)
    }

    /// Build a tree structure and return both the tree and path-to-ID mapping
    fn build_hierarchy_tree_with_ids(&self, hierarchy_map: &HashMap<String, (String, usize, Option<String>)>) -> (Vec<serde_json::Value>, HashMap<String, String>) {
        let mut path_to_id: HashMap<String, String> = HashMap::new();
        let mut id_counter = 1;
        
        for path in hierarchy_map.keys() {
            path_to_id.insert(path.clone(), format!("bt_{}", id_counter));
            id_counter += 1;
        }

        // Find root items (depth 0)
        let mut roots = Vec::new();
        
        for (path, (name, depth, _)) in hierarchy_map {
            if *depth == 0 {
                let tree_node = Self::build_tree_node(path, name, hierarchy_map, &path_to_id);
                roots.push(tree_node);
            }
        }
        
        (roots, path_to_id)
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
        let mut children = Vec::new();
        for (child_path, (child_name, _, parent_path)) in hierarchy_map {
            if let Some(parent) = parent_path {
                if parent == current_path {
                    let child_node = Self::build_tree_node(child_path, child_name, hierarchy_map, path_to_id);
                    children.push(child_node);
                }
            }
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

    /// Extract meaningful function name from backtrace
    fn extract_function_name(fn_name: &str) -> String {
        if fn_name.is_empty() {
            return "unknown".to_string();
        }
        
        // Remove module paths and keep just the function name
        let parts: Vec<&str> = fn_name.split("::").collect();
        let last_part = parts.last().unwrap_or(&"unknown");
        
        // Handle closure syntax
        if *last_part == "{{closure}}" && parts.len() > 1 {
            parts[parts.len() - 2].to_string()
        } else {
            last_part.to_string()
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
    ir: &[crate::ir::HydroLeaf],
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
    ir: &[crate::ir::HydroLeaf],
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

    // Use the centralized debug functionality instead of duplicating logic
    super::debug::open_json_visualizer(ir, Some(config))
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
}

/// Save JSON to file using the consolidated debug utilities
pub fn save_json(
    ir: &[crate::ir::HydroLeaf],
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
    built_flow: &crate::builder::built::BuiltFlow,
) -> Result<(), Box<dyn std::error::Error>> {
    open_json_browser(
        built_flow.ir(),
        built_flow.process_id_name().clone(),
        built_flow.cluster_id_name().clone(),
        built_flow.external_id_name().clone(),
    )
}


