use clap::{Parser, ValueEnum};

/// Enum for choosing between mermaid, dot, and json graph writing.
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
    /// Graph format to generate and display
    #[clap(long)]
    pub graph: Option<GraphType>,

    /// Write graph to file instead of opening in browser
    #[clap(long)]
    pub file: bool,

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

impl GraphConfig {
    /// Returns true if the program should exit after generating a graph file.
    /// This happens when both --file and --graph flags are provided.
    pub fn should_exit_after_graph_generation(&self) -> bool {
        self.file && self.graph.is_some()
    }
}

/// Configuration for visualizer URL generation and compression.
/// Controls how graphs are encoded and opened in web browsers.
#[derive(Debug, Clone)]
pub struct VisualizerConfig {
    /// Base URL for the visualizer (default: <https://hydro.run/hydroscope>)
    pub base_url: String,
    /// Whether to enable compression for small graphs
    pub enable_compression: bool,
    /// Maximum URL length before falling back to file-based approach
    pub max_url_length: usize,
    /// Minimum JSON size to attempt compression
    pub min_compression_size: usize,
}

impl Default for VisualizerConfig {
    fn default() -> Self {
        // Check for environment variable override for local development
        let base_url = std::env::var("HYDRO_VISUALIZER_URL")
            .unwrap_or_else(|_| "https://hydro.run/hydroscope".to_string());

        Self {
            base_url,
            enable_compression: true,
            max_url_length: 4000,
            min_compression_size: 1000,
        }
    }
}

impl VisualizerConfig {
    /// Create a new configuration with custom base URL
    pub fn with_base_url(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            ..Default::default()
        }
    }

    /// Create a configuration for local development
    pub fn local() -> Self {
        Self::with_base_url("http://localhost:3000/hydroscope")
    }

    /// Disable compression (useful for debugging)
    pub fn without_compression(mut self) -> Self {
        self.enable_compression = false;
        self
    }
}
