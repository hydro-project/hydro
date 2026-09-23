use clap::{Parser, ValueEnum};

/// Graph output format.
#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum GraphType {
    /// Mermaid graphs.
    Mermaid,
    /// Dot (Graphviz) graphs.
    Dot,
    /// JSON format for Hydroscope interactive viewer.
    Json,
}

impl GraphType {
    /// File extension for this format.
    pub fn file_extension(self) -> &'static str {
        match self {
            GraphType::Mermaid => "mmd",
            GraphType::Dot => "dot",
            GraphType::Json => "json",
        }
    }
}

/// Configuration for graph generation in examples.
#[derive(Parser, Debug, Default)]
pub struct GraphConfig {
    /// Graph format to generate (writes to file and exits)
    #[clap(long)]
    pub graph: Option<GraphType>,

    /// Output file path (default: `hydro_graph.{ext}`)
    #[clap(long, short = 'o')]
    pub output: Option<String>,

    /// Use full/long labels instead of short ones
    #[clap(long)]
    pub long_labels: bool,

    /// Don't show metadata in graph nodes
    #[clap(long)]
    pub no_metadata: bool,
}
