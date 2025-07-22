use clap::{Parser, ValueEnum};
use hydro_lang::graph::render::HydroWriteConfig;

/// Enum for choosing between mermaid, dot, and reactflow graph writing.
#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum GraphType {
    /// Mermaid graphs.
    Mermaid,
    /// Dot (Graphviz) graphs.
    Dot,
    /// Reactflow.js interactive graphs.
    Reactflow,
}

/// Configuration for graph generation in examples.
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

impl GraphConfig {
    /// Convert to HydroWriteConfig with the built flow's names
    pub fn to_hydro_config(
        &self,
        built: &hydro_lang::builder::built::BuiltFlow,
    ) -> HydroWriteConfig {
        HydroWriteConfig {
            show_metadata: !self.no_metadata,
            show_location_groups: !self.no_location_groups,
            include_tee_ids: !self.no_tee_ids,
            use_short_labels: !self.long_labels, // Inverted because our flag is for long labels
            process_id_name: built.process_id_name().clone(),
            cluster_id_name: built.cluster_id_name().clone(),
            external_id_name: built.external_id_name().clone(),
        }
    }

    /// Generate graph based on the configuration
    pub fn generate_graph(
        &self,
        built: &hydro_lang::builder::built::BuiltFlow,
        message_handler: Option<&dyn Fn(&str)>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let default_handler = |msg: &str| println!("{}", msg);
        let handler = message_handler.unwrap_or(&default_handler);

        if let Some(graph_type) = self.graph {
            let config = self.to_hydro_config(built);

            match graph_type {
                GraphType::Mermaid => {
                    handler("Opening Mermaid graph in browser...");
                    hydro_lang::graph::debug::open_mermaid(built.ir(), Some(config))?;
                }
                GraphType::Dot => {
                    handler("Opening Graphviz/DOT graph in browser...");
                    hydro_lang::graph::debug::open_dot(built.ir(), Some(config))?;
                }
                GraphType::Reactflow => {
                    handler("Opening ReactFlow graph in browser...");
                    hydro_lang::graph::debug::open_reactflow_browser(
                        built.ir(),
                        None,
                        Some(config),
                    )?;
                }
            }
        }
        Ok(())
    }

    /// Generate all graph types and save to files with a given prefix
    pub fn generate_all_files(
        &self,
        built: &hydro_lang::builder::built::BuiltFlow,
        prefix: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let config = self.to_hydro_config(built);

        let label_suffix = if self.long_labels { "_long" } else { "_short" };

        // Generate Mermaid
        let mermaid_content =
            hydro_lang::graph::render::render_hydro_ir_mermaid(built.ir(), &config);
        let mermaid_file = format!("{}{}_labels.mmd", prefix, label_suffix);
        std::fs::write(&mermaid_file, mermaid_content)?;
        println!("Generated: {}", mermaid_file);

        // Generate Graphviz
        let dot_content = hydro_lang::graph::render::render_hydro_ir_dot(built.ir(), &config);
        let dot_file = format!("{}{}_labels.dot", prefix, label_suffix);
        std::fs::write(&dot_file, dot_content)?;
        println!("Generated: {}", dot_file);

        // Generate ReactFlow
        let reactflow_content =
            hydro_lang::graph::render::render_hydro_ir_reactflow(built.ir(), &config);
        let reactflow_file = format!("{}{}_labels.json", prefix, label_suffix);
        std::fs::write(&reactflow_file, reactflow_content)?;
        println!("Generated: {}", reactflow_file);

        Ok(())
    }
}
