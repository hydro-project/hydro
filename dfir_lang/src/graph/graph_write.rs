#![warn(missing_docs)]

use std::borrow::Cow;
use std::error::Error;

use auto_impl::auto_impl;
use slotmap::Key;

use super::ops::DelayType;
use super::{Color, GraphLoopId, GraphNodeId, GraphSubgraphId};

/// Trait for writing textual representations of graphs, i.e. mermaid or dot graphs.
#[auto_impl(&mut, Box)]
pub(crate) trait GraphWrite {
    /// Error type emitted by writing.
    type Err: Error;

    /// Begin the graph. First method called.
    fn write_prologue(&mut self) -> Result<(), Self::Err>;

    /// Write a node, with styling.
    fn write_node_definition(
        &mut self,
        node_id: GraphNodeId,
        node: &str,
        node_color: Option<Color>,
    ) -> Result<(), Self::Err>;

    /// Write an edge, with styling.
    fn write_edge(
        &mut self,
        src_id: GraphNodeId,
        dst_id: GraphNodeId,
        delay_type: Option<DelayType>,
        label: Option<&str>,
        is_reference: bool,
    ) -> Result<(), Self::Err>;

    /// Begin writing a loop.
    fn write_loop_start(&mut self, loop_id: GraphLoopId) -> Result<(), Self::Err>;
    /// Begin writing a subgraph.
    fn write_subgraph_start(
        &mut self,
        sg_id: GraphSubgraphId,
        stratum: usize,
    ) -> Result<(), Self::Err>;
    /// Begin writing a varname block.
    fn write_varname_start(
        &mut self,
        varname: &str,
        sg_id: Option<GraphSubgraphId>,
    ) -> Result<(), Self::Err>;

    /// Write a node within a stack of loops, subgraphs, and/or varname blocks.
    fn write_node(&mut self, node_id: GraphNodeId) -> Result<(), Self::Err>;

    // Generic end method for loops, subgraphs, and varname blocks.
    fn write_end(&mut self) -> Result<(), Self::Err>;

    /// End writing a varname block.
    fn write_varname_end(&mut self) -> Result<(), Self::Err> {
        self.write_end()
    }
    /// End writing a subgraph.
    fn write_subgraph_end(&mut self) -> Result<(), Self::Err> {
        self.write_end()
    }
    /// End writing a loop.
    fn write_loop_end(&mut self) -> Result<(), Self::Err> {
        self.write_end()
    }

    /// End the graph. Last method called.
    fn write_epilogue(&mut self) -> Result<(), Self::Err>;
}

/// Escapes a string for use in a mermaid graph label.
pub fn escape_mermaid(string: &str) -> String {
    string
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        // Mermaid entity codes
        // https://mermaid.js.org/syntax/flowchart.html#entity-codes-to-escape-characters
        .replace('#', "&num;")
        // Not really needed, newline literals seem to work
        .replace('\n', "<br>")
        // Mermaid font awesome fa
        // https://github.com/mermaid-js/mermaid/blob/e4d2118d4bfa023628a020b7ab1f8c491e6dc523/packages/mermaid/src/diagrams/flowchart/flowRenderer-v2.js#L62
        .replace("fa:fa", "fa:<wbr>fa")
        .replace("fab:fa", "fab:<wbr>fa")
        .replace("fal:fa", "fal:<wbr>fa")
        .replace("far:fa", "far:<wbr>fa")
        .replace("fas:fa", "fas:<wbr>fa")
}

pub struct Mermaid<W> {
    write: W,
    indent: usize,
    // How many links have been written, for styling
    // https://mermaid.js.org/syntax/flowchart.html#styling-links
    link_count: usize,
}
impl<W> Mermaid<W> {
    pub fn new(write: W) -> Self {
        Self {
            write,
            indent: 0,
            link_count: 0,
        }
    }
}
impl<W> GraphWrite for Mermaid<W>
where
    W: std::fmt::Write,
{
    type Err = std::fmt::Error;

    fn write_prologue(&mut self) -> Result<(), Self::Err> {
        writeln!(
            self.write,
            "{b:i$}%%{{init:{{'theme':'base','themeVariables':{{'clusterBkg':'#ddd','clusterBorder':'#888'}}}}}}%%
{b:i$}flowchart TD
{b:i$}classDef pullClass fill:#8af,stroke:#000,text-align:left,white-space:pre
{b:i$}classDef pushClass fill:#ff8,stroke:#000,text-align:left,white-space:pre
{b:i$}classDef otherClass fill:#fdc,stroke:#000,text-align:left,white-space:pre
{b:i$}linkStyle default stroke:#aaa",
            b = "",
            i = self.indent
        )?;
        Ok(())
    }

    fn write_node_definition(
        &mut self,
        node_id: GraphNodeId,
        node: &str,
        node_color: Option<Color>,
    ) -> Result<(), Self::Err> {
        let class_str = match node_color {
            Some(Color::Push) => "pushClass",
            Some(Color::Pull) => "pullClass",
            _ => "otherClass",
        };
        let label = format!(
            r#"{node_id:?}{lbracket}"{node_label} <code>{code}</code>"{rbracket}:::{class}"#,
            node_id = node_id.data(),
            node_label = if node.contains('\n') {
                format!("<div style=text-align:center>({:?})</div>", node_id.data())
            } else {
                format!("({:?})", node_id.data())
            },
            class = class_str,
            lbracket = match node_color {
                Some(Color::Push) => r"[/",
                Some(Color::Pull) => r"[\",
                _ => "[",
            },
            code = escape_mermaid(node),
            rbracket = match node_color {
                Some(Color::Push) => r"\]",
                Some(Color::Pull) => r"/]",
                _ => "]",
            },
        );
        writeln!(self.write, "{b:i$}{label}", b = "", i = self.indent)?;
        Ok(())
    }

    fn write_edge(
        &mut self,
        src_id: GraphNodeId,
        dst_id: GraphNodeId,
        delay_type: Option<DelayType>,
        label: Option<&str>,
        _is_reference: bool,
    ) -> Result<(), Self::Err> {
        let src_str = format!("{:?}", src_id.data());
        let dest_str = format!("{:?}", dst_id.data());
        #[expect(clippy::write_literal, reason = "code readability")]
        write!(
            self.write,
            "{b:i$}{src}{arrow_body}{arrow_head}{label}{dst}",
            src = src_str.trim(),
            arrow_body = "--",
            arrow_head = match delay_type {
                None | Some(DelayType::MonotoneAccum) => ">",
                Some(DelayType::Stratum) => "x",
                Some(DelayType::Tick | DelayType::TickLazy) => "o",
            },
            label = if let Some(label) = &label {
                Cow::Owned(format!("|{}|", escape_mermaid(label.trim())))
            } else {
                Cow::Borrowed("")
            },
            dst = dest_str.trim(),
            b = "",
            i = self.indent,
        )?;
        if let Some(delay_type) = delay_type {
            write!(
                self.write,
                "; linkStyle {} stroke:{}",
                self.link_count,
                match delay_type {
                    DelayType::Stratum | DelayType::Tick | DelayType::TickLazy => "red",
                    DelayType::MonotoneAccum => "#060",
                }
            )?;
        }
        writeln!(self.write)?;
        self.link_count += 1;
        Ok(())
    }

    fn write_loop_start(&mut self, loop_id: GraphLoopId) -> Result<(), Self::Err> {
        writeln!(
            self.write,
            "{b:i$}subgraph loop_{lid:?} [\"loop_{lid:?}\"]",
            lid = loop_id.data(),
            b = "",
            i = self.indent,
        )?;
        self.indent += 4;
        Ok(())
    }

    fn write_subgraph_start(
        &mut self,
        sg_id: GraphSubgraphId,
        stratum: usize,
    ) -> Result<(), Self::Err> {
        writeln!(
            self.write,
            "{b:i$}subgraph sg_{sg:?} [\"sg_{sg:?} stratum {:?}\"]",
            stratum,
            sg = sg_id.data(),
            b = "",
            i = self.indent,
        )?;
        self.indent += 4;
        Ok(())
    }

    fn write_varname_start(
        &mut self,
        varname: &str,
        sg_id: Option<GraphSubgraphId>,
    ) -> Result<(), Self::Err> {
        let mut varname_id = format!("var_{varname}");
        if let Some(sg_id) = sg_id {
            varname_id = format!("sg_{sg:?}_{varname_id}", sg = sg_id.data());
        }
        writeln!(
            self.write,
            "{b:i$}subgraph {varname_id} [\"var <tt>{varname}</tt>\"]",
            b = "",
            i = self.indent,
        )?;
        self.indent += 4;
        writeln!(
            self.write,
            "{b:i$}style {varname_id} fill:transparent",
            b = "",
            i = self.indent
        )?;
        Ok(())
    }

    fn write_node(&mut self, node_id: GraphNodeId) -> Result<(), Self::Err> {
        writeln!(
            self.write,
            "{b:i$}{nid:?}",
            nid = node_id.data(),
            b = "",
            i = self.indent
        )
    }

    fn write_end(&mut self) -> Result<(), Self::Err> {
        self.write_loop_end()
    }

    fn write_epilogue(&mut self) -> Result<(), Self::Err> {
        // No-op.
        Ok(())
    }
}

/// Escapes a string for use in a DOT graph label.
///
/// Newline can be:
/// * "\\n" for newline.
/// * "\\l" for left-aligned newline.
/// * "\\r" for right-aligned newline.
pub fn escape_dot(string: &str, newline: &str) -> String {
    string.replace('"', "\\\"").replace('\n', newline)
}

pub struct Dot<W> {
    write: W,
    indent: usize,
}
impl<W> Dot<W> {
    pub fn new(write: W) -> Self {
        Self { write, indent: 0 }
    }
}
impl<W> GraphWrite for Dot<W>
where
    W: std::fmt::Write,
{
    type Err = std::fmt::Error;

    fn write_prologue(&mut self) -> Result<(), Self::Err> {
        writeln!(self.write, "{b:i$}digraph {{", b = "", i = self.indent)?;
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
        node_id: GraphNodeId,
        node: &str,
        node_color: Option<Color>,
    ) -> Result<(), Self::Err> {
        let nm = escape_dot(node, "\\l");
        let label = format!("n{:?}", node_id.data());
        let shape_str = match node_color {
            Some(Color::Push) => "house",
            Some(Color::Pull) => "invhouse",
            Some(Color::Hoff) => "parallelogram",
            Some(Color::Comp) => "circle",
            None => "rectangle",
        };
        let color_str = match node_color {
            Some(Color::Push) => "\"#ffff88\"",
            Some(Color::Pull) => "\"#88aaff\"",
            Some(Color::Hoff) => "\"#ddddff\"",
            Some(Color::Comp) => "white",
            None => "\"#ddddff\"",
        };
        write!(
            self.write,
            "{b:i$}{label} [label=\"({label}) {nm}{}\"",
            // if contains linebreak left-justify by appending another "\\l"
            if nm.contains("\\l") { "\\l" } else { "" },
            b = "",
            i = self.indent,
        )?;
        write!(self.write, ", shape={shape_str}, fillcolor={color_str}")?;
        writeln!(self.write, "]")?;
        Ok(())
    }

    fn write_edge(
        &mut self,
        src_id: GraphNodeId,
        dst_id: GraphNodeId,
        delay_type: Option<DelayType>,
        label: Option<&str>,
        _is_reference: bool,
    ) -> Result<(), Self::Err> {
        let mut properties = Vec::<Cow<'static, str>>::new();
        if let Some(label) = label {
            properties.push(format!("label=\"{}\"", escape_dot(label, "\\n")).into());
        };
        // Color
        if delay_type.is_some() {
            properties.push("color=red".into());
        }

        write!(
            self.write,
            "{b:i$}n{:?} -> n{:?}",
            src_id.data(),
            dst_id.data(),
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

    fn write_loop_start(&mut self, loop_id: GraphLoopId) -> Result<(), Self::Err> {
        writeln!(
            self.write,
            "{b:i$}subgraph loop_{lid:?} {{",
            lid = loop_id.data(),
            b = "",
            i = self.indent,
        )?;
        self.indent += 4;
        writeln!(self.write, "{b:i$}cluster=true", b = "", i = self.indent)?;
        writeln!(
            self.write,
            "{b:i$}fillcolor=\"#ee88ee\"",
            b = "",
            i = self.indent,
        )?;
        writeln!(self.write, "{b:i$}style=filled", b = "", i = self.indent)?;
        writeln!(
            self.write,
            "{b:i$}label = loop_{lid:?}",
            lid = loop_id.data(),
            b = "",
            i = self.indent
        )?;
        Ok(())
    }

    fn write_subgraph_start(
        &mut self,
        sg_id: GraphSubgraphId,
        stratum: usize,
    ) -> Result<(), Self::Err> {
        writeln!(
            self.write,
            "{b:i$}subgraph sg_{sg:?} {{",
            sg = sg_id.data(),
            b = "",
            i = self.indent
        )?;
        self.indent += 4;
        writeln!(self.write, "{b:i$}cluster=true", b = "", i = self.indent)?;
        writeln!(
            self.write,
            "{b:i$}fillcolor=\"#dddddd\"",
            b = "",
            i = self.indent
        )?;
        writeln!(self.write, "{b:i$}style=filled", b = "", i = self.indent)?;
        writeln!(
            self.write,
            "{b:i$}label = \"sg_{sg:?}\\nstratum {stratum}\"",
            sg = sg_id.data(),
            b = "",
            i = self.indent
        )?;
        Ok(())
    }

    fn write_varname_start(
        &mut self,
        varname: &str,
        sg_id: Option<GraphSubgraphId>,
    ) -> Result<(), Self::Err> {
        let mut varname_id = format!("var_{varname}");
        if let Some(sg_id) = sg_id {
            varname_id = format!("sg_{sg:?}_{varname_id}", sg = sg_id.data());
        }
        writeln!(
            self.write,
            "{b:i$}subgraph {varname_id} {{",
            b = "",
            i = self.indent
        )?;
        self.indent += 4;
        writeln!(self.write, "{b:i$}cluster=true", b = "", i = self.indent)?;
        writeln!(
            self.write,
            "{b:i$}label=\"var {var}\"",
            var = varname,
            b = "",
            i = self.indent
        )?;
        Ok(())
    }

    fn write_node(&mut self, node_id: GraphNodeId) -> Result<(), Self::Err> {
        writeln!(
            self.write,
            "{b:i$}n{nid:?}",
            nid = node_id.data(),
            b = "",
            i = self.indent
        )
    }

    fn write_end(&mut self) -> Result<(), Self::Err> {
        self.indent -= 4;
        writeln!(self.write, "{b:i$}}}", b = "", i = self.indent)
    }

    fn write_epilogue(&mut self) -> Result<(), Self::Err> {
        self.write_end()
    }
}
