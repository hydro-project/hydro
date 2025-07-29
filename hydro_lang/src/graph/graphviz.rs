use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::Write;

use serde_json;

use super::render::{HydroEdgeType, HydroGraphWrite, HydroNodeType, IndentedGraphWriter};

/// Escapes a string for use in a DOT graph label.
pub fn escape_dot(string: &str, newline: &str) -> String {
    string.replace('"', "\\\"").replace('\n', newline)
}

/// DOT/Graphviz graph writer for Hydro IR.
pub struct HydroDot<W> {
    base: IndentedGraphWriter<W>,
}

impl<W> HydroDot<W> {
    pub fn new(write: W) -> Self {
        Self {
            base: IndentedGraphWriter::new(write),
        }
    }

    pub fn new_with_config(write: W, config: &super::render::HydroWriteConfig) -> Self {
        Self {
            base: IndentedGraphWriter::new_with_config(write, config),
        }
    }
}

/// JSON graph writer for Hydro IR - generates JSON for the new ReactFlow visualizer.
pub struct HydroJson<W> {
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
            edge_count: 0,
            config: config.clone(),
            process_names,
            cluster_names,
            external_names,
        }
    }

    /// Find which location a node belongs to
    fn find_node_location(&self, node_id: usize) -> Option<usize> {
        for (location_id, (_, node_ids)) in &self.locations {
            if node_ids.contains(&node_id) {
                return Some(*location_id);
            }
        }
        None
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
        self.edge_count = 0;
        Ok(())
    }

    fn write_node_definition(
        &mut self,
        node_id: usize,
        node_label: &super::render::NodeLabel,
        node_type: HydroNodeType,
        location_id: Option<usize>,
        _location_type: Option<&str>,
    ) -> Result<(), Self::Err> {
        // Create the full label string using DebugExpr::Display for expressions
        let full_label = match node_label {
            super::render::NodeLabel::Static(s) => s.clone(),
            super::render::NodeLabel::WithExprs { op_name, exprs } => {
                if exprs.is_empty() {
                    format!("{}()", op_name)
                } else {
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

        // Create the node in the format expected by the visualizer
        let node = serde_json::json!({
            "id": format!("n{}", node_id),
            "data": {
                "label": display_label,
                "nodeType": match node_type {
                    HydroNodeType::Source => "Source",
                    HydroNodeType::Transform => "Transform", 
                    HydroNodeType::Join => "Join",
                    HydroNodeType::Aggregation => "Aggregation",
                    HydroNodeType::Network => "Network",
                    HydroNodeType::Sink => "Sink",
                    HydroNodeType::Tee => "Tee",
                }
            }
        });

        self.nodes.push(node);

        // Track which location this node belongs to
        if let Some(loc_id) = location_id {
            if let Some((_, node_ids)) = self.locations.get_mut(&loc_id) {
                node_ids.push(node_id);
            }
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
        let edge_id = format!("e{}", self.edge_count);
        self.edge_count += 1;

        // Find the locations of source and target nodes
        let src_location = self.find_node_location(src_id);
        let dst_location = self.find_node_location(dst_id);
        
        // Determine if this is a cross-location edge
        let is_cross_location = src_location != dst_location;
        
        // Create edge with styling information
        let mut edge = serde_json::json!({
            "id": edge_id,
            "source": format!("n{}", src_id),
            "target": format!("n{}", dst_id),
            "type": "smoothstep",
            "style": {},
            "animated": false
        });

        // Apply styling based on edge type and cross-location detection
        let mut style = serde_json::Map::new();
        let mut animated = false;

        match edge_type {
            HydroEdgeType::Persistent => {
                style.insert("stroke".to_string(), serde_json::Value::String("#008800".to_string()));
                style.insert("strokeWidth".to_string(), serde_json::Value::Number(serde_json::Number::from(3)));
            }
            HydroEdgeType::Network => {
                style.insert("stroke".to_string(), serde_json::Value::String("#880088".to_string()));
                style.insert("strokeDasharray".to_string(), serde_json::Value::String("5,5".to_string()));
                style.insert("strokeWidth".to_string(), serde_json::Value::Number(serde_json::Number::from(2)));
                animated = true;
            }
            HydroEdgeType::Cycle => {
                style.insert("stroke".to_string(), serde_json::Value::String("#ff0000".to_string()));
                animated = true;
            }
            HydroEdgeType::Stream => {
                style.insert("stroke".to_string(), serde_json::Value::String("#666666".to_string()));
                style.insert("strokeWidth".to_string(), serde_json::Value::Number(serde_json::Number::from(2)));
            }
        }

        // Override styling for cross-location edges (network communication)
        if is_cross_location {
            style.insert("stroke".to_string(), serde_json::Value::String("#880088".to_string()));
            style.insert("strokeDasharray".to_string(), serde_json::Value::String("8,4".to_string()));
            style.insert("strokeWidth".to_string(), serde_json::Value::Number(serde_json::Number::from(2)));
            animated = true;
        }

        edge["style"] = serde_json::Value::Object(style);
        edge["animated"] = serde_json::Value::Bool(animated);

        // Add label if provided
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
            _ => format!("{} {}", location_type, location_id),
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

        // Create node assignments (mapping nodes to their locations)
        let mut node_assignments = serde_json::Map::new();
        for (location_id, (_, node_ids)) in &self.locations {
            let location_key = format!("loc_{}", location_id);
            for &node_id in node_ids {
                let node_key = format!("n{}", node_id);
                node_assignments.insert(node_key, serde_json::Value::String(location_key.clone()));
            }
        }

        // Create the final JSON structure in the format expected by the visualizer
        let output = serde_json::json!({
            "nodes": self.nodes,
            "edges": self.edges,
            "hierarchy": hierarchy,
            "nodeAssignments": node_assignments
        });

        write!(
            self.write,
            "{}",
            serde_json::to_string_pretty(&output).unwrap()
        )?;
        Ok(())
    }
}

impl<W> HydroGraphWrite for HydroDot<W>
where
    W: Write,
{
    type Err = super::render::GraphWriteError;

    fn write_prologue(&mut self) -> Result<(), Self::Err> {
        writeln!(
            self.base.write,
            "{b:i$}digraph HydroIR {{",
            b = "",
            i = self.base.indent
        )?;
        self.base.indent += 4;

        // Use dot layout for better edge routing between subgraphs
        writeln!(
            self.base.write,
            "{b:i$}layout=dot;",
            b = "",
            i = self.base.indent
        )?;
        writeln!(
            self.base.write,
            "{b:i$}compound=true;",
            b = "",
            i = self.base.indent
        )?;
        writeln!(
            self.base.write,
            "{b:i$}concentrate=true;",
            b = "",
            i = self.base.indent
        )?;

        const FONTS: &str = "\"Monaco,Menlo,Consolas,&quot;Droid Sans Mono&quot;,Inconsolata,&quot;Courier New&quot;,monospace\"";
        writeln!(
            self.base.write,
            "{b:i$}node [fontname={}, style=filled];",
            FONTS,
            b = "",
            i = self.base.indent
        )?;
        writeln!(
            self.base.write,
            "{b:i$}edge [fontname={}];",
            FONTS,
            b = "",
            i = self.base.indent
        )?;
        Ok(())
    }

    fn write_node_definition(
        &mut self,
        node_id: usize,
        node_label: &super::render::NodeLabel,
        node_type: HydroNodeType,
        _location_id: Option<usize>,
        _location_type: Option<&str>,
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

        // Determine what label to display based on config
        let display_label = if self.base.config.use_short_labels {
            super::render::extract_short_label(&full_label)
        } else {
            full_label
        };

        let escaped_label = escape_dot(&display_label, "\\l");
        let label = format!("n{}", node_id);

        let (shape_str, color_str) = match node_type {
            // ColorBrewer Set3 palette colors (matching Mermaid and ReactFlow)
            HydroNodeType::Source => ("ellipse", "\"#8dd3c7\""), // Light teal
            HydroNodeType::Transform => ("box", "\"#ffffb3\""),  // Light yellow
            HydroNodeType::Join => ("diamond", "\"#bebada\""),   // Light purple
            HydroNodeType::Aggregation => ("house", "\"#fb8072\""), // Light red/salmon
            HydroNodeType::Network => ("doubleoctagon", "\"#80b1d3\""), // Light blue
            HydroNodeType::Sink => ("invhouse", "\"#fdb462\""),  // Light orange
            HydroNodeType::Tee => ("terminator", "\"#b3de69\""), // Light green
        };

        write!(
            self.base.write,
            "{b:i$}{label} [label=\"({node_id}) {escaped_label}{}\"",
            if escaped_label.contains("\\l") {
                "\\l"
            } else {
                ""
            },
            b = "",
            i = self.base.indent,
        )?;
        write!(
            self.base.write,
            ", shape={shape_str}, fillcolor={color_str}"
        )?;
        writeln!(self.base.write, "]")?;
        Ok(())
    }

    fn write_edge(
        &mut self,
        src_id: usize,
        dst_id: usize,
        edge_type: HydroEdgeType,
        label: Option<&str>,
    ) -> Result<(), Self::Err> {
        let mut properties = Vec::<Cow<'static, str>>::new();

        if let Some(label) = label {
            properties.push(format!("label=\"{}\"", escape_dot(label, "\\n")).into());
        }

        // Styling based on edge type
        match edge_type {
            HydroEdgeType::Persistent => {
                properties.push("color=\"#008800\"".into());
                properties.push("style=\"bold\"".into());
            }
            HydroEdgeType::Network => {
                properties.push("color=\"#880088\"".into());
                properties.push("style=\"dashed\"".into());
            }
            HydroEdgeType::Cycle => {
                properties.push("color=\"#ff8800\"".into());
                properties.push("style=\"dotted\"".into());
            }
            HydroEdgeType::Stream => {}
        }

        write!(
            self.base.write,
            "{b:i$}n{} -> n{}",
            src_id,
            dst_id,
            b = "",
            i = self.base.indent,
        )?;

        if !properties.is_empty() {
            write!(self.base.write, " [")?;
            for prop in itertools::Itertools::intersperse(properties.into_iter(), ", ".into()) {
                write!(self.base.write, "{}", prop)?;
            }
            write!(self.base.write, "]")?;
        }
        writeln!(self.base.write)?;
        Ok(())
    }

    fn write_location_start(
        &mut self,
        location_id: usize,
        location_type: &str,
    ) -> Result<(), Self::Err> {
        writeln!(
            self.base.write,
            "{b:i$}subgraph cluster_loc_{id} {{",
            id = location_id,
            b = "",
            i = self.base.indent,
        )?;
        self.base.indent += 4;

        // Use dot layout for interior nodes within containers
        writeln!(
            self.base.write,
            "{b:i$}layout=dot;",
            b = "",
            i = self.base.indent
        )?;
        writeln!(
            self.base.write,
            "{b:i$}label = \"{location_type} {id}\"",
            id = location_id,
            b = "",
            i = self.base.indent
        )?;
        writeln!(
            self.base.write,
            "{b:i$}style=filled",
            b = "",
            i = self.base.indent
        )?;
        writeln!(
            self.base.write,
            "{b:i$}fillcolor=\"#fafafa\"",
            b = "",
            i = self.base.indent
        )?;
        writeln!(
            self.base.write,
            "{b:i$}color=\"#e0e0e0\"",
            b = "",
            i = self.base.indent
        )?;
        Ok(())
    }

    fn write_node(&mut self, node_id: usize) -> Result<(), Self::Err> {
        writeln!(
            self.base.write,
            "{b:i$}n{node_id}",
            b = "",
            i = self.base.indent
        )
    }

    fn write_location_end(&mut self) -> Result<(), Self::Err> {
        self.base.indent -= 4;
        writeln!(self.base.write, "{b:i$}}}", b = "", i = self.base.indent)
    }

    fn write_epilogue(&mut self) -> Result<(), Self::Err> {
        self.base.indent -= 4;
        writeln!(self.base.write, "{b:i$}}}", b = "", i = self.base.indent)
    }
}

/// Open DOT/Graphviz visualization in browser for a BuiltFlow
#[cfg(feature = "build")]
pub fn open_browser(
    built_flow: &crate::builder::built::BuiltFlow,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = super::render::HydroWriteConfig {
        show_metadata: false,
        show_location_groups: true,
        use_short_labels: true, // Default to short labels
        process_id_name: built_flow.process_id_name().clone(),
        cluster_id_name: built_flow.cluster_id_name().clone(),
        external_id_name: built_flow.external_id_name().clone(),
    };

    // Use the existing debug function
    crate::graph::debug::open_dot(built_flow.ir(), Some(config))?;

    Ok(())
}
