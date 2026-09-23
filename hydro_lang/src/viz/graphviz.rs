use std::borrow::Cow;
use std::fmt::Write;

use super::render::{
    HydroEdgeProp, HydroGraphWrite, HydroNodeType, HydroWriteConfig, IndentedGraphWriter,
};
use crate::location::{LocationKey, LocationType};
use crate::viz::render::VizNodeKey;

/// Escapes a string for use in a DOT graph label.
pub fn escape_dot(string: &str, newline: &str) -> String {
    string.replace('"', "\\\"").replace('\n', newline)
}

/// DOT/Graphviz graph writer for Hydro IR.
pub struct HydroDot<'a, W> {
    base: IndentedGraphWriter<'a, W>,
}

impl<'a, W> HydroDot<'a, W> {
    pub fn new(write: W) -> Self {
        Self {
            base: IndentedGraphWriter::new(write),
        }
    }

    pub fn new_with_config(write: W, config: HydroWriteConfig<'a>) -> Self {
        Self {
            base: IndentedGraphWriter::new_with_config(write, config),
        }
    }
}

impl<W> HydroGraphWrite for HydroDot<'_, W>
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

        let escaped_label = escape_dot(&display_label, "\\l");
        let label = format!("n{}", node_id);

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
        write!(self.base.write, ", shape=box, fillcolor=\"#f5f5f5\"")?;
        writeln!(self.base.write, "]")?;
        Ok(())
    }

    fn write_edge(
        &mut self,
        src_id: VizNodeKey,
        dst_id: VizNodeKey,
        edge_properties: &std::collections::HashSet<HydroEdgeProp>,
        label: Option<&str>,
    ) -> Result<(), Self::Err> {
        let mut properties = Vec::<Cow<'static, str>>::new();

        if let Some(label) = label {
            properties.push(format!("label=\"{}\"", escape_dot(label, "\\n")).into());
        }

        let style = super::render::get_unified_edge_style(edge_properties, None, None);

        properties.push(format!("color=\"{}\"", style.color).into());

        if style.line_width > 1 {
            properties.push("style=\"bold\"".into());
        }

        match style.line_pattern {
            super::render::LinePattern::Dotted => {
                properties.push("style=\"dotted\"".into());
            }
            super::render::LinePattern::Dashed => {
                properties.push("style=\"dashed\"".into());
            }
            _ => {}
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
        location_key: LocationKey,
        location_type: LocationType,
    ) -> Result<(), Self::Err> {
        writeln!(
            self.base.write,
            "{b:i$}subgraph cluster_{location_key} {{",
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
            "{b:i$}label = \"{location_type:?} {location_key}\"",
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
        writeln!(self.base.write, "{b:i$}}}", b = "", i = self.base.indent)
    }

    fn write_epilogue(&mut self) -> Result<(), Self::Err> {
        self.base.indent -= 4;
        writeln!(self.base.write, "{b:i$}}}", b = "", i = self.base.indent)
    }
}
