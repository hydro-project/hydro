use std::borrow::Cow;
use std::fmt::Write;

use super::render::{HydroEdgeType, HydroGraphWrite, HydroNodeType};

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
    write: W,
    indent: usize,
    link_count: usize,
}

impl<W> HydroMermaid<W> {
    pub fn new(write: W) -> Self {
        Self {
            write,
            indent: 0,
            link_count: 0,
        }
    }
}

impl<W> HydroGraphWrite for HydroMermaid<W>
where
    W: Write,
{
    type Err = std::fmt::Error;

    fn write_prologue(&mut self) -> Result<(), Self::Err> {
        writeln!(
            self.write,
            "{b:i$}%%{{init:{{'theme':'base','themeVariables':{{'clusterBkg':'#fafafa','clusterBorder':'#e0e0e0'}}}}}}%%
{b:i$}flowchart TD
{b:i$}classDef sourceClass fill:#8dd3c7,stroke:#86c8bd,text-align:left,white-space:pre
{b:i$}classDef transformClass fill:#ffffb3,stroke:#f5f5a8,text-align:left,white-space:pre
{b:i$}classDef joinClass fill:#bebada,stroke:#b5b1cf,text-align:left,white-space:pre
{b:i$}classDef aggClass fill:#fb8072,stroke:#ee796b,text-align:left,white-space:pre
{b:i$}classDef networkClass fill:#80b1d3,stroke:#79a8c8,text-align:left,white-space:pre
{b:i$}classDef sinkClass fill:#fdb462,stroke:#f0aa5b,text-align:left,white-space:pre
{b:i$}classDef teeClass fill:#b3de69,stroke:#aad362,text-align:left,white-space:pre
{b:i$}style loc_* fill:#fafafa,stroke:#e0e0e0,rx:8,ry:8
{b:i$}linkStyle default stroke:#666666",
            b = "",
            i = self.indent
        )?;
        Ok(())
    }

    fn write_node_definition(
        &mut self,
        node_id: usize,
        node_label: &str,
        node_type: HydroNodeType,
        _location_id: Option<usize>,
        _location_type: Option<&str>,
    ) -> Result<(), Self::Err> {
        let class_str = match node_type {
            HydroNodeType::Source => "sourceClass",
            HydroNodeType::Transform => "transformClass",
            HydroNodeType::Join => "joinClass",
            HydroNodeType::Aggregation => "aggClass",
            HydroNodeType::Network => "networkClass",
            HydroNodeType::Sink => "sinkClass",
            HydroNodeType::Tee => "teeClass",
        };

        let (lbracket, rbracket) = match node_type {
            HydroNodeType::Source => ("[[", "]]"),
            HydroNodeType::Sink => ("[/", "/]"),
            HydroNodeType::Network => ("[[", "]]"),
            HydroNodeType::Tee => ("(", ")"),
            _ => ("[", "]"),
        };

        let label = format!(
            r#"n{node_id}{lbracket}"{escaped_label}"{rbracket}:::{class}"#,
            escaped_label = escape_mermaid(node_label),
            class = class_str,
        );

        writeln!(self.write, "{b:i$}{label}", b = "", i = self.indent)?;
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
            self.write,
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
            i = self.indent,
        )?;

        // Add styling for different edge types on a separate line
        if !matches!(edge_type, HydroEdgeType::Stream) {
            writeln!(
                self.write,
                "{b:i$}linkStyle {} stroke:{}",
                self.link_count,
                match edge_type {
                    HydroEdgeType::Persistent => "#008800",
                    HydroEdgeType::Network => "#880088",
                    HydroEdgeType::Cycle => "#ff0000",
                    HydroEdgeType::Stream => "#666666", /* Should not be used here, but for completeness. */
                },
                b = "",
                i = self.indent,
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
            self.write,
            "{b:i$}subgraph loc_{id} [\"{location_type} {id}\"]",
            id = location_id,
            b = "",
            i = self.indent,
        )?;
        self.indent += 4;
        Ok(())
    }

    fn write_node(&mut self, node_id: usize) -> Result<(), Self::Err> {
        writeln!(self.write, "{b:i$}n{node_id}", b = "", i = self.indent)
    }

    fn write_location_end(&mut self) -> Result<(), Self::Err> {
        self.indent -= 4;
        writeln!(self.write, "{b:i$}end", b = "", i = self.indent)
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
        include_tee_ids: true,
        process_id_name: built_flow.process_id_name().clone(),
        cluster_id_name: built_flow.cluster_id_name().clone(),
        external_id_name: built_flow.external_id_name().clone(),
    };

    // Use the existing debug function
    crate::graph::debug::open_hydro_ir_mermaid(built_flow.ir(), Some(config))?;

    Ok(())
}
