use std::borrow::Cow;
use std::collections::HashSet;
use std::fmt::Write;

use crate::builder::ir::backtrace::Backtrace;

use super::render::{HydroEdgeType, HydroGraphWrite, HydroNodeType, IndentedGraphWriter};

/// Escapes a string for use in a mermaid graph label.
pub fn escape_mermaid(string: &str) -> String {
    string
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('#', "&num;")
        .replace('\n', "<br>")
        // Handle code block markers
        .replace("`", "&#96;")
        // Handle parentheses that can conflict with Mermaid syntax
        .replace('(', "&#40;")
        .replace(')', "&#41;")
        // Handle pipes that can conflict with Mermaid edge labels
        .replace('|', "&#124;")
}

/// Mermaid node type data: (type, class, left_bracket, right_bracket)
const MERMAID_NODE_DATA: &[(HydroNodeType, &str, &str, &str)] = &[
    (HydroNodeType::Source, "sourceClass", "[[", "]]"),
    (HydroNodeType::Transform, "transformClass", "(", ")"),
    (HydroNodeType::Join, "joinClass", "(", ")"),
    (HydroNodeType::Aggregation, "aggClass", "(", ")"),
    (HydroNodeType::Network, "networkClass", "[[", "]]"),
    (HydroNodeType::Sink, "sinkClass", "[/", "/]"),
    (HydroNodeType::Tee, "teeClass", "(", ")"),
];

/// Get Mermaid class name for node type
fn to_mermaid_class(node_type: HydroNodeType) -> &'static str {
    MERMAID_NODE_DATA
        .iter()
        .find(|(nt, _, _, _)| *nt == node_type)
        .map(|(_, class, _, _)| *class)
        .unwrap_or("defaultClass")
}

/// Get Mermaid shape for node type
fn to_mermaid_shape(node_type: HydroNodeType) -> (&'static str, &'static str) {
    MERMAID_NODE_DATA
        .iter()
        .find(|(nt, _, _, _)| *nt == node_type)
        .map(|(_, _, left, right)| (*left, *right))
        .unwrap_or(("(", ")"))
}

/// Mermaid graph writer for Hydro IR.
pub struct HydroMermaid<W> {
    base: IndentedGraphWriter<W>,
    link_count: usize,
}

impl<W> HydroMermaid<W> {
    pub fn new(write: W) -> Self {
        Self {
            base: IndentedGraphWriter::new(write),
            link_count: 0,
        }
    }

    pub fn new_with_config(write: W, config: &super::render::HydroWriteConfig) -> Self {
        Self {
            base: IndentedGraphWriter::new_with_config(write, config),
            link_count: 0,
        }
    }
}

impl<W> HydroGraphWrite for HydroMermaid<W>
where
    W: Write,
{
    type Err = super::render::GraphWriteError;

    fn write_prologue(&mut self) -> Result<(), Self::Err> {
        writeln!(
            self.base.write,
            "{b:i$}%%{{init:{{'theme':'base','themeVariables':{{'clusterBkg':'#fafafa','clusterBorder':'#e0e0e0'}},'elk':{{'algorithm':'mrtree','elk.direction':'DOWN','elk.layered.spacing.nodeNodeBetweenLayers':'30'}}}}}}%%
{b:i$}graph TD
{b:i$}classDef sourceClass fill:#8dd3c7,stroke:#86c8bd,text-align:left,white-space:pre
{b:i$}classDef transformClass fill:#ffffb3,stroke:#f5f5a8,text-align:left,white-space:pre
{b:i$}classDef joinClass fill:#bebada,stroke:#b5b1cf,text-align:left,white-space:pre
{b:i$}classDef aggClass fill:#fb8072,stroke:#ee796b,text-align:left,white-space:pre
{b:i$}classDef networkClass fill:#80b1d3,stroke:#79a8c8,text-align:left,white-space:pre
{b:i$}classDef sinkClass fill:#fdb462,stroke:#f0aa5b,text-align:left,white-space:pre
{b:i$}classDef teeClass fill:#b3de69,stroke:#aad362,text-align:left,white-space:pre
{b:i$}linkStyle default stroke:#666666",
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
        _backtrace: Option<&Backtrace>,
    ) -> Result<(), Self::Err> {
        let class_str = to_mermaid_class(node_type);
        let (lbracket, rbracket) = to_mermaid_shape(node_type);

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

        let label = format!(
            r#"n{node_id}{lbracket}"{escaped_label}"{rbracket}:::{class}"#,
            escaped_label = escape_mermaid(&display_label),
            class = class_str,
        );

        writeln!(
            self.base.write,
            "{b:i$}{label}",
            b = "",
            i = self.base.indent
        )?;
        Ok(())
    }

    fn write_edge(
        &mut self,
        src_id: usize,
        dst_id: usize,
        edge_properties: &HashSet<HydroEdgeType>,
        label: Option<&str>,
    ) -> Result<(), Self::Err> {
        // Get unified edge style (Mermaid doesn't have location context, so pass None)
        let style = super::render::get_unified_edge_style(edge_properties, None, None);

        // Choose arrow style based on unified style patterns
        let arrow_style = match (style.line_pattern, style.arrowhead) {
            // Dotted patterns for network/remote connections
            (super::render::LinePattern::Dotted, super::render::ArrowheadStyle::CircleFilled) => {
                "-.->"
            }
            (super::render::LinePattern::Dotted, _) => "-.->",

            // Circle arrowhead for singleton/special data
            (_, super::render::ArrowheadStyle::CircleFilled) => "--o",

            // Cross arrowhead for optional/nullable data
            (_, super::render::ArrowheadStyle::DiamondOpen) => "--x",

            // Thick lines for bounded/heavy data flows
            _ if style.line_width > 1 => "==>",

            // Default arrow
            _ => "-->",
        };

        // For double line style (keyed streams), we'll add a visual indicator in the linkStyle
        // For wavy lines (no order), we'll use stroke-dasharray to create a wavy pattern

        // Write the edge definition
        writeln!(
            self.base.write,
            "{b:i$}n{src}{arrow}{label}n{dst}",
            src = src_id,
            arrow = arrow_style,
            label = if let Some(label) = label {
                Cow::Owned(format!("|{}|", escape_mermaid(label)))
            } else {
                Cow::Borrowed("")
            },
            dst = dst_id,
            b = "",
            i = self.base.indent,
        )?;

        // Apply advanced styling using linkStyle
        let link_num = self.link_count;
        self.link_count += 1;

        // Build linkStyle properties
        let mut link_style_parts = vec![format!("stroke:{}", style.color)];

        // Apply stroke width
        if style.line_width > 1 {
            link_style_parts.push(format!("stroke-width:{}px", style.line_width));
        }

        // Apply special patterns for semantic meaning
        match (style.line_style, style.waviness) {
            // Double lines for keyed streams - use stroke-dasharray to simulate
            (super::render::LineStyle::Double, _) => {
                link_style_parts.push("stroke-dasharray:8 2 2 2".to_string());
            }
            // Wavy lines for no-order - use a wavy dash pattern
            (_, super::render::WavinessStyle::Wavy) => {
                link_style_parts.push("stroke-dasharray:4 4".to_string());
            }
            _ => {}
        }

        // Apply the combined linkStyle
        writeln!(
            self.base.write,
            "{b:i$}linkStyle {link_num} {style}",
            style = link_style_parts.join(","),
            b = "",
            i = self.base.indent,
        )?;

        Ok(())
    }

    fn write_location_start(
        &mut self,
        location_id: usize,
        location_type: &str,
    ) -> Result<(), Self::Err> {
        // Use the common location labeling utility
        let location_label =
            super::render::get_location_label(location_id, location_type, &self.base.config);
        writeln!(
            self.base.write,
            "{b:i$}subgraph {id} [\"{label}\"]",
            id = location_id,
            label = location_label,
            b = "",
            i = self.base.indent,
        )?;
        self.base.indent += 4;
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
        writeln!(self.base.write, "{b:i$}end", b = "", i = self.base.indent)
    }

    fn write_epilogue(&mut self) -> Result<(), Self::Err> {
        Ok(())
    }
}

/// Open mermaid visualization in browser for a BuiltFlow
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
    crate::graph::debug::open_mermaid(built_flow.ir(), Some(config))?;

    Ok(())
}
