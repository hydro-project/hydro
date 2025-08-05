use std::borrow::Cow;
use std::fmt::Write;

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
        _backtrace: Option<&crate::backtrace::Backtrace>,
    ) -> Result<(), Self::Err> {
        let class_str = super::render::node_type_utils::to_mermaid_class(node_type);
        let (lbracket, rbracket) = super::render::node_type_utils::to_mermaid_shape(node_type);

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
        edge_type: HydroEdgeType,
        label: Option<&str>,
    ) -> Result<(), Self::Err> {
        let arrow_style = match edge_type {
            HydroEdgeType::Stream => "-->",
            HydroEdgeType::Persistent => "==>",
            HydroEdgeType::Network => "-.->",
            HydroEdgeType::Cycle => "--o",
        };

        // Write the edge definition on its own line
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

        // Add styling for different edge types on a separate line
        if !matches!(edge_type, HydroEdgeType::Stream) {
            writeln!(
                self.base.write,
                "{b:i$}linkStyle {} stroke:{}",
                self.link_count,
                match edge_type {
                    HydroEdgeType::Persistent => "#008800",
                    HydroEdgeType::Network => "#880088",
                    HydroEdgeType::Cycle => "#ff0000",
                    HydroEdgeType::Stream => "#666666", /* Should not be used here, but for completeness. */
                },
                b = "",
                i = self.base.indent,
            )?;
        }

        self.link_count += 1;
        Ok(())
    }

    fn write_location_start(
        &mut self,
        location_id: usize,
        location_type: &str,
    ) -> Result<(), Self::Err> {
        writeln!(
            self.base.write,
            "{b:i$}subgraph loc_{id} [\"{location_type} {id}\"]",
            id = location_id,
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
