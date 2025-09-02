use std::borrow::Cow;
use std::collections::HashSet;
use std::fmt::Write;

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
        _backtrace: Option<&crate::backtrace::Backtrace>,
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

        let (shape_str, color_str) = super::render::node_type_utils::to_dot_style(node_type);

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
        edge_properties: &HashSet<HydroEdgeType>,
        label: Option<&str>,
    ) -> Result<(), Self::Err> {
        let mut properties = Vec::<Cow<'static, str>>::new();

        if let Some(label) = label {
            properties.push(format!("label=\"{}\"", escape_dot(label, "\\n")).into());
        }

        // Styling based on edge properties
        if edge_properties.contains(&HydroEdgeType::Network) {
            properties.push("color=\"#880088\"".into());
            properties.push("style=\"dashed\"".into());
        } else if edge_properties.contains(&HydroEdgeType::Cycle) {
            properties.push("color=\"#ff8800\"".into());
            properties.push("style=\"dotted\"".into());
        } else if edge_properties.contains(&HydroEdgeType::Bounded) {
            properties.push("color=\"#008800\"".into());
            properties.push("style=\"bold\"".into());
        } else if edge_properties.contains(&HydroEdgeType::NoOrder) {
            properties.push("color=\"#ff0000\"".into());
            properties.push("style=\"dashed\"".into());
        } else if edge_properties.contains(&HydroEdgeType::Keyed) {
            properties.push("color=\"#0088ff\"".into());
            properties.push("style=\"bold\"".into());
        }

        // Add tooltip with all properties
        if !edge_properties.is_empty() {
            let props: Vec<String> = edge_properties.iter().map(|p| format!("{:?}", p)).collect();
            properties.push(format!("tooltip=\"{}\"", props.join(", ")).into());
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
        ..Default::default()
    };

    // Use the existing debug function
    crate::graph::debug::open_dot(built_flow.ir(), Some(config))?;

    Ok(())
}
