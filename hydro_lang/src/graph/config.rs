#[cfg(feature = "viz")]
use clap::{Parser, ValueEnum};

/// Enum for choosing between mermaid, dot, and reactflow graph writing.
#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum GraphType {
    /// Mermaid graphs.
    Mermaid,
    /// Dot (Graphviz) graphs.
    Dot,
    /// JSON format for interactive graphs.
    Json,
}

impl std::fmt::Display for GraphType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Configuration for graph generation in examples.
#[derive(Parser, Debug, Default)]
pub struct GraphConfig {
    /// Output graph format
    #[clap(long)]
    pub graph: Option<GraphType>,

    /// Force save to temporary file instead of opening in browser
    #[clap(long)]
    pub file: bool,

    /// Don't show metadata in generated graphs
    #[clap(long)]
    pub no_metadata: bool,

    /// Don't show location groups in generated graphs
    #[clap(long)]
    pub no_location_groups: bool,

    /// Use long labels in generated graphs
    #[clap(long)]
    pub long_labels: bool,
}
