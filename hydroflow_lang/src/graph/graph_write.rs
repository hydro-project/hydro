#![deny(missing_docs)]

use std::error::Error;

use auto_impl::auto_impl;
use slotmap::Key;

use super::ops::DelayType;
use super::{Color, GraphNodeId, GraphSubgraphId};

/// Trait for writing textual representations of graphs, i.e. mermaid or dot graphs.
#[auto_impl(&mut, Box)]
pub trait GraphWrite {
    /// Error type emitted by writing.
    type Err: Error;

    /// Begin the graph. First method called.
    fn write_prologue(&mut self) -> Result<(), Self::Err>;

    /// Begin writing a subgraph.
    fn write_subgraph_start(
        &mut self,
        sg_id: GraphSubgraphId,
        stratum: usize,
    ) -> Result<(), Self::Err>;
    /// Write a node, possibly within a subgraph.
    fn write_node(
        &mut self,
        node_id: GraphNodeId,
        node: &str, //&Node,
        node_color: Color,
        in_subgraph: Option<GraphSubgraphId>,
    ) -> Result<(), Self::Err>;
    /// Write an edge, possibly within a subgraph.
    fn write_edge(
        &mut self,
        src_id: GraphNodeId,
        dst_id: GraphNodeId,
        delay_type: Option<DelayType>,
        label: Option<&str>,
        in_subgraph: Option<GraphSubgraphId>,
    ) -> Result<(), Self::Err>;
    /// Write the nodes associated with a single variable name, within a subgraph.
    fn write_subgraph_varname(
        &mut self,
        sg_id: GraphSubgraphId,
        varname: &str,
        varname_nodes: impl Iterator<Item = GraphNodeId>,
    ) -> Result<(), Self::Err>;
    /// End writing a subgraph.
    fn write_subgraph_end(&mut self) -> Result<(), Self::Err>;

    /// End the graph. Last method called.
    fn write_epilogue(&mut self) -> Result<(), Self::Err>;
}

pub struct Mermaid<W> {
    write: W,
}
impl<W> Mermaid<W> {
    pub fn new(write: W) -> Self {
        Self { write }
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
            r"%%{{init:{{'theme':'base','themeVariables':{{'clusterBkg':'#ddd','clusterBorder':'#888'}}}}}}%%",
        )?;
        writeln!(self.write, "flowchart TD")?;
        writeln!(
            self.write,
            "classDef pullClass fill:#02f,color:#fff,stroke:#000,text-align:left,white-space:pre"
        )?;
        writeln!(
            self.write,
            "classDef pushClass fill:#ff0,stroke:#000,text-align:left,white-space:pre"
        )?;

        writeln!(
            self.write,
            "linkStyle default stroke:#aaa,stroke-width:4px,color:red,font-size:1.5em;"
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
            "{:t$}subgraph sg_{sg:?} [\"sg_{sg:?} stratum {:?}\"]",
            "",
            stratum,
            sg = sg_id.data(),
            t = 0,
        )?;
        Ok(())
    }

    fn write_node(
        &mut self,
        node_id: GraphNodeId,
        node: &str, //&Node,
        node_color: Color,
        in_subgraph: Option<GraphSubgraphId>,
    ) -> Result<(), Self::Err> {
        let class_str = match node_color {
            Color::Push => "pushClass",
            Color::Pull => "pullClass",
            _ => "otherClass",
        };
        let label = format!(
            r#"{:t$}{id:?}{lbracket}"{id_label} <code>{code}</code>"{rbracket}:::{class}"#,
            "",
            id = node_id.data(),
            id_label = if node.contains('\n') {
                format!("<div style=text-align:center>({:?})</div>", node_id.data())
            } else {
                format!("({:?})", node_id.data())
            },
            class = class_str,
            lbracket = match node_color {
                Color::Push => r"[/",
                Color::Pull => r"[\",
                _ => "[",
            },
            code = node
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
                .replace("fas:fa", "fas:<wbr>fa"),
            rbracket = match node_color {
                Color::Push => r"\]",
                Color::Pull => r"/]",
                _ => "]",
            },
            t = if in_subgraph.is_some() { 4 } else { 0 },
        );
        writeln!(self.write, "{}", label)?;
        Ok(())
    }

    fn write_edge(
        &mut self,
        src_id: GraphNodeId,
        dst_id: GraphNodeId,
        delay_type: Option<DelayType>,
        label: Option<&str>,
        in_subgraph: Option<GraphSubgraphId>,
    ) -> Result<(), Self::Err> {
        let src_str = format!("{:?}", src_id.data());
        let dest_str = format!("{:?}", dst_id.data());
        writeln!(
            self.write,
            "{:t$}{src}{label}{delay}{dst}",
            "",
            src = src_str.trim(),
            label = if let Some(label) = &label {
                if Some(DelayType::Stratum) == delay_type {
                    format!("=={}", label.trim())
                } else {
                    format!("--{}", label.trim())
                }
            } else {
                "".to_string()
            },
            delay = if Some(DelayType::Stratum) == delay_type {
                "===o"
            } else {
                "--->"
            },
            dst = dest_str.trim(),
            t = if in_subgraph.is_some() { 4 } else { 0 },
        )?;
        Ok(())
    }

    fn write_subgraph_varname(
        &mut self,
        sg_id: GraphSubgraphId,
        varname: &str,
        varname_nodes: impl Iterator<Item = GraphNodeId>,
    ) -> Result<(), Self::Err> {
        writeln!(
            self.write,
            "{:t$}subgraph sg_{sg:?}_var_{var} [\"var <tt>{var}</tt>\"]",
            "",
            sg = sg_id.data(),
            var = varname,
            t = 4,
        )?;
        for local_named_node in varname_nodes {
            writeln!(self.write, "{:t$}{:?}", "", local_named_node.data(), t = 8)?;
        }
        writeln!(self.write, "{:t$}end", "", t = 4)?;
        Ok(())
    }

    fn write_subgraph_end(&mut self) -> Result<(), Self::Err> {
        writeln!(self.write, "{:t$}end", "", t = 0)?;
        Ok(())
    }

    fn write_epilogue(&mut self) -> Result<(), Self::Err> {
        // No-op.
        Ok(())
    }
}

pub struct Dot<W> {
    write: W,
}
impl<W> Dot<W> {
    pub fn new(write: W) -> Self {
        Self { write }
    }
}
impl<W> GraphWrite for Dot<W>
where
    W: std::fmt::Write,
{
    type Err = std::fmt::Error;

    fn write_prologue(&mut self) -> Result<(), Self::Err> {
        writeln!(self.write, "digraph {{")?;
        Ok(())
    }

    fn write_subgraph_start(
        &mut self,
        sg_id: GraphSubgraphId,
        stratum: usize,
    ) -> Result<(), Self::Err> {
        writeln!(
            self.write,
            "{:t$}subgraph \"cluster n{:?}\" {{",
            "",
            sg_id.data(),
            t = 4,
        )?;
        writeln!(self.write, "{:t$}fillcolor=\"#dddddd\"", "", t = 8)?;
        writeln!(self.write, "{:t$}style=filled", "", t = 8)?;
        writeln!(
            self.write,
            "{:t$}label = \"sg_{:?}\\nstratum {}\"",
            "",
            sg_id.data(),
            stratum,
            t = 8,
        )?;
        Ok(())
    }

    fn write_node(
        &mut self,
        node_id: GraphNodeId,
        node: &str, //&Node,
        node_color: Color,
        in_subgraph: Option<GraphSubgraphId>,
    ) -> Result<(), Self::Err> {
        let nm = node.replace('"', "\\\"").replace('\n', "\\l");
        let label = format!("n{:?}", node_id.data());
        let shape_str = match node_color {
            Color::Push => "house",
            Color::Pull => "invhouse",
            Color::Hoff => "parallelogram",
            Color::Comp => "circle",
        };
        let color_str = match node_color {
            Color::Push => "style = filled, color = \"#ffff00\"",
            Color::Pull => "style = filled, color = \"#0022ff\", fontcolor = \"#ffffff\"",
            Color::Hoff => "style = filled, color = \"#ddddff\"",
            Color::Comp => "style = filled, color = white",
        };
        write!(
            self.write,
            "{:t$}{} [label=\"({}) {}{}\"",
            "",
            label,
            label,
            nm,
            // if contains linebreak left-justify by appending another "\\l"
            if nm.contains("\\l") { "\\l" } else { "" },
            t = if in_subgraph.is_some() { 8 } else { 4 },
        )?;
        write!(self.write, ", fontname=Monaco")?;
        write!(self.write, ", shape={}", shape_str)?;
        write!(self.write, ", {}", color_str)?;
        writeln!(self.write, "]")?;
        Ok(())
    }

    fn write_edge(
        &mut self,
        src_id: GraphNodeId,
        dst_id: GraphNodeId,
        delay_type: Option<DelayType>,
        label: Option<&str>,
        in_subgraph: Option<GraphSubgraphId>,
    ) -> Result<(), Self::Err> {
        let mut properties = Vec::new();
        if let Some(label) = label {
            properties.push(format!("label=\"{}\"", label));
        };
        if Some(DelayType::Stratum) == delay_type {
            properties.push("arrowhead=box, color=red".to_string());
        };
        writeln!(
            self.write,
            "{:t$}n{:?} -> n{:?}{}",
            "",
            src_id.data(),
            dst_id.data(),
            if !properties.is_empty() {
                format!(" [{}]", properties.join(", "))
            } else {
                "".to_string()
            },
            t = if in_subgraph.is_some() { 8 } else { 4 },
        )?;
        Ok(())
    }

    fn write_subgraph_varname(
        &mut self,
        sg_id: GraphSubgraphId,
        varname: &str,
        varname_nodes: impl Iterator<Item = GraphNodeId>,
    ) -> Result<(), Self::Err> {
        writeln!(
            self.write,
            "{:t$}subgraph \"cluster sg_{sg:?}_var_{var}\" {{",
            "",
            sg = sg_id.data(),
            var = varname,
            t = 8,
        )?;
        writeln!(
            self.write,
            "{:t$}label=\"var {var}\"",
            "",
            var = varname,
            t = 12,
        )?;
        for local_named_node in varname_nodes {
            writeln!(
                self.write,
                "{:t$}n{:?}",
                "",
                local_named_node.data(),
                t = 12
            )?;
        }
        writeln!(self.write, "{:t$}}}", "", t = 8)?;
        Ok(())
    }

    fn write_subgraph_end(&mut self) -> Result<(), Self::Err> {
        // subgraph footer
        writeln!(self.write, "{:t$}}}", "", t = 4)?;
        Ok(())
    }

    fn write_epilogue(&mut self) -> Result<(), Self::Err> {
        writeln!(self.write, "}}")?;
        Ok(())
    }
}
