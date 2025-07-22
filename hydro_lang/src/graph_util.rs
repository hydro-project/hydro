#[cfg(feature = "build")]
use clap::{Parser, ValueEnum};

/// Enum for choosing between mermaid, dot, and reactflow graph writing.
#[cfg(feature = "build")]
#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum GraphType {
    /// Mermaid graphs.
    Mermaid,
    /// Dot (Graphviz) graphs.
    Dot,
    /// Reactflow.js interactive graphs.
    Reactflow,
}

#[cfg(feature = "build")]
impl std::fmt::Display for GraphType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Configuration for graph generation in examples.
#[cfg(feature = "build")]
#[derive(Parser, Debug)]
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

#[cfg(feature = "build")]
impl Default for GraphConfig {
    fn default() -> Self {
        Self {
            graph: None,
            no_metadata: false,
            no_location_groups: false,
            no_tee_ids: false,
            long_labels: false,
        }
    }
}

#[cfg(feature = "build")]
impl GraphConfig {
    /// Convert to HydroWriteConfig with the built flow's names
    pub fn to_hydro_config(
        &self,
        built: &crate::builder::built::BuiltFlow,
    ) -> crate::graph::render::HydroWriteConfig {
        crate::graph::render::HydroWriteConfig {
            show_metadata: !self.no_metadata,
            show_location_groups: !self.no_location_groups,
            include_tee_ids: !self.no_tee_ids,
            use_short_labels: !self.long_labels, // Inverted because our flag is for long labels
            process_id_name: built.process_id_name().clone(),
            cluster_id_name: built.cluster_id_name().clone(),
            external_id_name: built.external_id_name().clone(),
        }
    }

    /// Generate graph based on the configuration by delegating to BuiltFlow methods
    pub fn generate_graph(
        &self,
        built: &crate::builder::built::BuiltFlow,
        message_handler: Option<&dyn Fn(&str)>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        built.generate_graph_with_config(self, message_handler)
    }

    /// Generate all graph types by delegating to BuiltFlow's generate_all_files method
    pub fn generate_all_files(
        &self,
        built: &crate::builder::built::BuiltFlow,
        prefix: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        built.generate_all_files_with_config(self, prefix)
    }
}
