#[cfg(feature = "viz")]
use clap::{Parser, ValueEnum};

/// Enum for choosing between mermaid, dot, and reactflow graph writing.
#[cfg(feature = "viz")]
#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum GraphType {
    /// Mermaid graphs.
    Mermaid,
    /// Dot (Graphviz) graphs.
    Dot,
    /// Reactflow.js interactive graphs.
    Reactflow,
}

#[cfg(feature = "viz")]
impl std::fmt::Display for GraphType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Configuration for graph generation in examples.
#[cfg(feature = "viz")]
#[derive(Parser, Debug, Default)]
pub struct GraphConfig {
    /// Graph format to generate and display
    #[clap(long)]
    pub graph: Option<GraphType>,

    /// Don't show metadata in graph nodes
    #[clap(long)]
    pub no_metadata: bool,

    /// Don't show location groups
    #[clap(long)]
    pub no_location_groups: bool,

    /// Don't include tee IDs in nodes
    #[clap(long)]
    pub no_tee_ids: bool,

    /// Use full/long labels instead of short ones
    #[clap(long)]
    pub long_labels: bool,
}
