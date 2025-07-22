use std::collections::HashMap;
use std::fmt::Write;

use serde_json;

use super::render::{HydroEdgeType, HydroGraphWrite, HydroNodeType};

/// ReactFlow.js graph writer for Hydro IR.
/// Outputs JSON that can be directly used with ReactFlow.js for interactive graph visualization.
pub struct HydroReactFlow<W> {
    write: W,
    nodes: Vec<serde_json::Value>,
    edges: Vec<serde_json::Value>,
    locations: HashMap<usize, (String, Vec<usize>)>, // location_id -> (label, node_ids)
    edge_count: usize,
    config: super::render::HydroWriteConfig,
    // Type name mappings
    process_names: HashMap<usize, String>,
    cluster_names: HashMap<usize, String>,
    external_names: HashMap<usize, String>,
}

impl<W> HydroReactFlow<W> {
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
            edge_count: 0,
            config: config.clone(),
            process_names,
            cluster_names,
            external_names,
        }
    }

    fn node_type_to_style(&self, _node_type: HydroNodeType) -> serde_json::Value {
        // Base template for all nodes with modern card styling
        let base_style = serde_json::json!({
            "color": "#2d3748",
            "border": "1px solid rgba(0, 0, 0, 0.1)",
            "borderRadius": "12px",
            "padding": "12px 16px",
            "fontSize": "13px",
            "fontFamily": "-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif",
            "fontWeight": "500",
            "boxShadow": "0 4px 6px -1px rgba(0, 0, 0, 0.1), 0 2px 4px -1px rgba(0, 0, 0, 0.06)",
            "transition": "all 0.2s ease-in-out"
        });

        // Store node type for the frontend ColorBrewer system to use
        // The actual colors will be applied dynamically by the JavaScript based on the selected palette
        let mut style = base_style;

        // Add hover effect styling
        style["&:hover"] = serde_json::json!({
            "transform": "translateY(-2px)",
            "boxShadow": "0 8px 25px -5px rgba(0, 0, 0, 0.1), 0 4px 6px -2px rgba(0, 0, 0, 0.05)"
        });

        style
    }
    fn edge_type_to_style(&self, edge_type: HydroEdgeType) -> serde_json::Value {
        // Base template for all edges
        let mut style = serde_json::json!({
            "strokeWidth": 2,
            "animated": false
        });

        // Apply type-specific overrides
        match edge_type {
            HydroEdgeType::Stream => {
                style["stroke"] = serde_json::Value::String("#666666".to_string());
            }
            HydroEdgeType::Persistent => {
                style["stroke"] = serde_json::Value::String("#008800".to_string());
                style["strokeWidth"] = serde_json::Value::Number(serde_json::Number::from(3));
            }
            HydroEdgeType::Network => {
                style["stroke"] = serde_json::Value::String("#880088".to_string());
                style["strokeDasharray"] = serde_json::Value::String("5,5".to_string());
                style["animated"] = serde_json::Value::Bool(true);
            }
            HydroEdgeType::Cycle => {
                style["stroke"] = serde_json::Value::String("#ff0000".to_string());
                style["animated"] = serde_json::Value::Bool(true);
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

impl<W> HydroGraphWrite for HydroReactFlow<W>
where
    W: Write,
{
    type Err = super::render::GraphWriteError;

    fn write_prologue(&mut self) -> Result<(), Self::Err> {
        // Clear any existing data
        self.nodes.clear();
        self.edges.clear();
        self.locations.clear();
        self.edge_count = 0;
        Ok(())
    }

    fn write_node_definition(
        &mut self,
        node_id: usize,
        node_label: &str,
        node_type: HydroNodeType,
        location_id: Option<usize>,
        location_type: Option<&str>,
    ) -> Result<(), Self::Err> {
        let style = self.node_type_to_style(node_type);

        // Determine what label to display based on config
        let display_label = if self.config.use_short_labels {
            super::render::extract_short_label(node_label)
        } else {
            node_label.to_string()
        };

        // Always extract short label for UI toggle functionality
        let short_label = super::render::extract_short_label(node_label);

        // If short and full labels are the same or very similar, enhance the full label
        let enhanced_full_label = if short_label.len() >= node_label.len() - 2 {
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
                    if node_label.len() < 15 {
                        format!("{} [{}]", node_label, "hydro operator")
                    } else {
                        node_label.to_string()
                    }
                }
            }
        } else {
            node_label.to_string()
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
                "nodeType": match node_type {
                    HydroNodeType::Source => "Source",
                    HydroNodeType::Transform => "Transform",
                    HydroNodeType::Join => "Join",
                    HydroNodeType::Aggregation => "Aggregation",
                    HydroNodeType::Network => "Network",
                    HydroNodeType::Sink => "Sink",
                    HydroNodeType::Tee => "Tee",
                }
            },
            "position": {
                "x": 0,
                "y": 0
            },
            "style": style
        });
        self.nodes.push(node);
        Ok(())
    }

    fn write_edge(
        &mut self,
        src_id: usize,
        dst_id: usize,
        edge_type: HydroEdgeType,
        label: Option<&str>,
    ) -> Result<(), Self::Err> {
        let style = self.edge_type_to_style(edge_type);
        let edge_id = format!("e{}", self.edge_count);
        self.edge_count += 1;

        let mut edge = serde_json::json!({
            "id": edge_id,
            "source": src_id.to_string(),
            "target": dst_id.to_string(),
            "style": style,
            // Use smart edge type for better routing and flexible connection points
            "type": "smoothstep",
            // Let ReactFlow choose optimal connection points
            // Remove fixed sourceHandle/targetHandle to enable flexible connections
            "animated": false
        });

        // Add animation for certain edge types
        if matches!(edge_type, HydroEdgeType::Network | HydroEdgeType::Cycle) {
            edge["animated"] = serde_json::Value::Bool(true);
        }

        if let Some(label_text) = label {
            edge["label"] = serde_json::Value::String(label_text.to_string());
            edge["labelStyle"] = serde_json::json!({
                "fontSize": "10px",
                "fontFamily": "monospace",
                "fill": "#333333",
                "backgroundColor": "rgba(255, 255, 255, 0.8)",
                "padding": "2px 4px",
                "borderRadius": "3px"
            });
            // Center the label on the edge
            edge["labelShowBg"] = serde_json::Value::Bool(true);
            edge["labelBgStyle"] = serde_json::json!({
                "fill": "rgba(255, 255, 255, 0.8)",
                "fillOpacity": 0.8
            });
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
        // Location grouping complete - nothing to do for ReactFlow
        Ok(())
    }

    fn write_epilogue(&mut self) -> Result<(), Self::Err> {
        // Apply automatic layout using a simple algorithm
        self.apply_layout();

        // Create the final JSON structure
        let output = serde_json::json!({
            "nodes": self.nodes,
            "edges": self.edges,
            "locations": self.locations.iter().map(|(id, (label, nodes))| {
                serde_json::json!({
                    "id": id.to_string(),
                    "label": label,
                    "nodes": nodes
                })
            }).collect::<Vec<_>>()
        });

        write!(
            self.write,
            "{}",
            serde_json::to_string_pretty(&output).unwrap()
        )
    }
}

/// Create ReactFlow JSON from Hydro IR with type names
pub fn hydro_ir_to_reactflow(
    ir: &[crate::ir::HydroLeaf],
    process_names: Vec<(usize, String)>,
    cluster_names: Vec<(usize, String)>,
    external_names: Vec<(usize, String)>,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut output = String::new();

    let config = super::render::HydroWriteConfig {
        show_metadata: false,
        show_location_groups: true,
        include_tee_ids: true,
        use_short_labels: true, // Default to short labels
        process_id_name: process_names,
        cluster_id_name: cluster_names,
        external_id_name: external_names,
    };

    super::render::write_hydro_ir_reactflow(&mut output, ir, &config)?;

    Ok(output)
}

/// Open ReactFlow visualization in browser using the consolidated debug utilities
pub fn open_reactflow_browser(
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

    super::debug::open_reactflow_browser(ir, Some("hydro_graph.html"), Some(config))
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
}

/// Save ReactFlow JSON to file using the consolidated debug utilities
pub fn save_reactflow_json(
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

    super::debug::save_reactflow_json(ir, Some(filename), Some(config))
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
}

/// Open ReactFlow visualization in browser for a BuiltFlow
#[cfg(feature = "build")]
pub fn open_browser(
    built_flow: &crate::builder::built::BuiltFlow,
) -> Result<(), Box<dyn std::error::Error>> {
    open_reactflow_browser(
        built_flow.ir(),
        built_flow.process_id_name().clone(),
        built_flow.cluster_id_name().clone(),
        built_flow.external_id_name().clone(),
    )
}
