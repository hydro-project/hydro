use std::borrow::Cow;
use std::fmt::Write;

use super::render::{
    HydroEdgeProp, HydroGraphWrite, HydroNodeType, HydroWriteConfig, IndentedGraphWriter,
};
use crate::location::{LocationKey, LocationType};
use crate::viz::render::VizNodeKey;

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
pub struct HydroMermaid<'a, W> {
    base: IndentedGraphWriter<'a, W>,
    link_count: usize,
}

impl<'a, W> HydroMermaid<'a, W> {
    pub fn new(write: W) -> Self {
        Self {
            base: IndentedGraphWriter::new(write),
            link_count: 0,
        }
    }

    pub fn new_with_config(write: W, config: HydroWriteConfig<'a>) -> Self {
        Self {
            base: IndentedGraphWriter::new_with_config(write, config),
            link_count: 0,
        }
    }
}

impl<W> HydroGraphWrite for HydroMermaid<'_, W>
where
    W: Write,
{
    type Err = super::render::GraphWriteError;

    fn write_prologue(&mut self) -> Result<(), Self::Err> {
        writeln!(
            self.base.write,
            "{b:i$}%%{{init:{{'theme':'base','themeVariables':{{'clusterBkg':'#fafafa','clusterBorder':'#e0e0e0'}},'elk':{{'algorithm':'mrtree','elk.direction':'DOWN','elk.layered.spacing.nodeNodeBetweenLayers':'30'}}}}}}%%
{b:i$}graph TD
{b:i$}classDef default fill:#f5f5f5,stroke:#bbb,text-align:left,white-space:pre
{b:i$}linkStyle default stroke:#666666",
            b = "",
            i = self.base.indent
        )?;
        Ok(())
    }

    fn write_node_definition(
        &mut self,
        node_id: VizNodeKey,
        node_label: &super::render::NodeLabel,
        _node_type: HydroNodeType,
        _location_id: Option<LocationKey>,
        _location_type: Option<LocationType>,
        _backtrace: Option<&crate::compile::ir::backtrace::Backtrace>,
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

        writeln!(
            self.base.write,
            "{b:i$}n{node_id}[\"{escaped_label}\"]",
            escaped_label = escape_mermaid(&display_label),
            b = "",
            i = self.base.indent
        )?;
        Ok(())
    }

    fn write_edge(
        &mut self,
        src_id: VizNodeKey,
        dst_id: VizNodeKey,
        edge_properties: &std::collections::HashSet<HydroEdgeProp>,
        label: Option<&str>,
    ) -> Result<(), Self::Err> {
        // Use unified edge style system
        let style = super::render::get_unified_edge_style(edge_properties, None, None);

        // Determine arrow style based on edge properties
        let arrow_style = if edge_properties.contains(&HydroEdgeProp::Network) {
            "-.->".to_owned()
        } else {
            match style.line_pattern {
                super::render::LinePattern::Dotted => "-.->".to_owned(),
                super::render::LinePattern::Dashed => "--o".to_owned(),
                _ => {
                    if style.line_width > 1 {
                        "==>".to_owned()
                    } else {
                        "-->".to_owned()
                    }
                }
            }
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

        // Add styling using unified edge style color
        writeln!(
            self.base.write,
            "{b:i$}linkStyle {} stroke:{}",
            self.link_count,
            style.color,
            b = "",
            i = self.base.indent,
        )?;

        self.link_count += 1;
        Ok(())
    }

    fn write_location_start(
        &mut self,
        location_key: LocationKey,
        location_type: LocationType,
    ) -> Result<(), Self::Err> {
        writeln!(
            self.base.write,
            "{b:i$}subgraph {loc} [\"{location_type:?} {loc}\"]",
            loc = location_key,
            b = "",
            i = self.base.indent,
        )?;
        self.base.indent += 4;
        Ok(())
    }

    fn write_node(&mut self, node_id: VizNodeKey) -> Result<(), Self::Err> {
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
