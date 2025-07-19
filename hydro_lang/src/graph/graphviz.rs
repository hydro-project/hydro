use std::borrow::Cow;
use std::fmt::Write;

use super::render::{HydroEdgeType, HydroGraphWrite, HydroNodeType};

/// Escapes a string for use in a DOT graph label.
pub fn escape_dot(string: &str, newline: &str) -> String {
    string.replace('"', "\\\"").replace('\n', newline)
}

/// DOT/Graphviz graph writer for Hydro IR.
pub struct HydroDot<W> {
    write: W,
    indent: usize,
}

impl<W> HydroDot<W> {
    pub fn new(write: W) -> Self {
        Self { write, indent: 0 }
    }
}

impl<W> HydroGraphWrite for HydroDot<W>
where
    W: Write,
{
    type Err = std::fmt::Error;

    fn write_prologue(&mut self) -> Result<(), Self::Err> {
        writeln!(
            self.write,
            "{b:i$}digraph HydroIR {{",
            b = "",
            i = self.indent
        )?;
        self.indent += 4;

        const FONTS: &str = "\"Monaco,Menlo,Consolas,&quot;Droid Sans Mono&quot;,Inconsolata,&quot;Courier New&quot;,monospace\"";
        writeln!(
            self.write,
            "{b:i$}node [fontname={}, style=filled];",
            FONTS,
            b = "",
            i = self.indent
        )?;
        writeln!(
            self.write,
            "{b:i$}edge [fontname={}];",
            FONTS,
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
        let escaped_label = escape_dot(node_label, "\\l");
        let label = format!("n{}", node_id);

        let (shape_str, color_str) = match node_type {
            HydroNodeType::Source => ("ellipse", "\"#88ff88\""),
            HydroNodeType::Transform => ("box", "\"#8888ff\""),
            HydroNodeType::Join => ("diamond", "\"#ff8888\""),
            HydroNodeType::Aggregation => ("house", "\"#ffff88\""),
            HydroNodeType::Network => ("doubleoctagon", "\"#88ffff\""),
            HydroNodeType::Sink => ("invhouse", "\"#ff88ff\""),
            HydroNodeType::Tee => ("circle", "\"#dddddd\""),
        };

        write!(
            self.write,
            "{b:i$}{label} [label=\"({node_id}) {escaped_label}{}\"",
            if escaped_label.contains("\\l") {
                "\\l"
            } else {
                ""
            },
            b = "",
            i = self.indent,
        )?;
        write!(self.write, ", shape={shape_str}, fillcolor={color_str}")?;
        writeln!(self.write, "]")?;
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
            self.write,
            "{b:i$}n{} -> n{}",
            src_id,
            dst_id,
            b = "",
            i = self.indent,
        )?;

        if !properties.is_empty() {
            write!(self.write, " [")?;
            for prop in itertools::Itertools::intersperse(properties.into_iter(), ", ".into()) {
                write!(self.write, "{}", prop)?;
            }
            write!(self.write, "]")?;
        }
        writeln!(self.write)?;
        Ok(())
    }

    fn write_location_start(
        &mut self,
        location_id: usize,
        location_type: &str,
    ) -> Result<(), Self::Err> {
        writeln!(
            self.write,
            "{b:i$}subgraph cluster_loc_{id} {{",
            id = location_id,
            b = "",
            i = self.indent,
        )?;
        self.indent += 4;
        writeln!(
            self.write,
            "{b:i$}label = \"{location_type} {id}\"",
            id = location_id,
            b = "",
            i = self.indent
        )?;
        writeln!(self.write, "{b:i$}style=filled", b = "", i = self.indent)?;
        writeln!(
            self.write,
            "{b:i$}fillcolor=\"#f0f0f0\"",
            b = "",
            i = self.indent
        )?;
        Ok(())
    }

    fn write_node(&mut self, node_id: usize) -> Result<(), Self::Err> {
        writeln!(self.write, "{b:i$}n{node_id}", b = "", i = self.indent)
    }

    fn write_location_end(&mut self) -> Result<(), Self::Err> {
        self.indent -= 4;
        writeln!(self.write, "{b:i$}}}", b = "", i = self.indent)
    }

    fn write_epilogue(&mut self) -> Result<(), Self::Err> {
        self.indent -= 4;
        writeln!(self.write, "{b:i$}}}", b = "", i = self.indent)
    }
}

/// Open DOT/Graphviz visualization in browser for a BuiltFlow
#[cfg(feature = "build")]
pub fn open_browser(built_flow: &crate::builder::built::BuiltFlow) -> Result<(), Box<dyn std::error::Error>> {
    let config = super::render::HydroWriteConfig {
        show_metadata: false,
        show_location_groups: true,
        include_tee_ids: true,
        process_id_name: built_flow.process_id_name().clone(),
        cluster_id_name: built_flow.cluster_id_name().clone(),
        external_id_name: built_flow.external_id_name().clone(),
    };
    
    // Use the existing debug function
    crate::graph::debug::open_hydro_ir_dot(built_flow.ir(), Some(config))?;
    
    Ok(())
}
