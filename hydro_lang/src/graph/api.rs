use std::error::Error;

use crate::graph::render::HydroWriteConfig;
use crate::ir::HydroRoot;

/// Graph generation API for built flows
pub struct GraphApi<'a> {
    ir: &'a [HydroRoot],
    process_id_name: &'a [(usize, String)],
    cluster_id_name: &'a [(usize, String)],
    external_id_name: &'a [(usize, String)],
}

/// Graph output format
#[derive(Debug, Clone, Copy)]
pub enum GraphFormat {
    Mermaid,
    Dot,
    ReactFlow,
}

impl GraphFormat {
    fn file_extension(self) -> &'static str {
        match self {
            GraphFormat::Mermaid => "mmd",
            GraphFormat::Dot => "dot",
            GraphFormat::ReactFlow => "json",
        }
    }

    fn browser_message(self) -> &'static str {
        match self {
            GraphFormat::Mermaid => "Opening Mermaid graph in browser...",
            GraphFormat::Dot => "Opening Graphviz/DOT graph in browser...",
            GraphFormat::ReactFlow => "Opening ReactFlow graph in browser...",
        }
    }
}

impl<'a> GraphApi<'a> {
    pub fn new(
        ir: &'a [HydroRoot],
        process_id_name: &'a [(usize, String)],
        cluster_id_name: &'a [(usize, String)],
        external_id_name: &'a [(usize, String)],
    ) -> Self {
        Self {
            ir,
            process_id_name,
            cluster_id_name,
            external_id_name,
        }
    }

    /// Convert configuration options to HydroWriteConfig
    fn to_hydro_config(
        &self,
        show_metadata: bool,
        show_location_groups: bool,
        use_short_labels: bool,
    ) -> HydroWriteConfig {
        HydroWriteConfig {
            show_metadata,
            show_location_groups,
            use_short_labels,
            process_id_name: self.process_id_name.to_vec(),
            cluster_id_name: self.cluster_id_name.to_vec(),
            external_id_name: self.external_id_name.to_vec(),
        }
    }

    /// Generate graph content as string
    fn render_graph_to_string(&self, format: GraphFormat, config: &HydroWriteConfig) -> String {
        match format {
            GraphFormat::Mermaid => crate::graph::render::render_hydro_ir_mermaid(self.ir, config),
            GraphFormat::Dot => crate::graph::render::render_hydro_ir_dot(self.ir, config),
            GraphFormat::ReactFlow => {
                crate::graph::render::render_hydro_ir_reactflow(self.ir, config)
            }
        }
    }

    /// Open graph in browser
    fn open_graph_in_browser(
        &self,
        format: GraphFormat,
        config: HydroWriteConfig,
    ) -> Result<(), Box<dyn Error>> {
        match format {
            GraphFormat::Mermaid => Ok(crate::graph::debug::open_mermaid(self.ir, Some(config))?),
            GraphFormat::Dot => Ok(crate::graph::debug::open_dot(self.ir, Some(config))?),
            GraphFormat::ReactFlow => Ok(crate::graph::debug::open_reactflow_browser(
                self.ir,
                None,
                Some(config),
            )?),
        }
    }

    /// Generic method to open graph in browser
    fn open_browser(
        &self,
        format: GraphFormat,
        show_metadata: bool,
        show_location_groups: bool,
        use_short_labels: bool,
        message_handler: Option<&dyn Fn(&str)>,
    ) -> Result<(), Box<dyn Error>> {
        let default_handler = |msg: &str| println!("{}", msg);
        let handler = message_handler.unwrap_or(&default_handler);

        let config = self.to_hydro_config(show_metadata, show_location_groups, use_short_labels);

        handler(format.browser_message());
        self.open_graph_in_browser(format, config)?;
        Ok(())
    }

    /// Generate and save graph to file
    fn write_graph_to_file(
        &self,
        format: GraphFormat,
        filename: &str,
        show_metadata: bool,
        show_location_groups: bool,
        use_short_labels: bool,
    ) -> Result<(), Box<dyn Error>> {
        let config = self.to_hydro_config(show_metadata, show_location_groups, use_short_labels);
        let content = self.render_graph_to_string(format, &config);
        std::fs::write(filename, content)?;
        println!("Generated: {}", filename);
        Ok(())
    }

    /// Generate mermaid graph as string
    pub fn mermaid_to_string(
        &self,
        show_metadata: bool,
        show_location_groups: bool,
        use_short_labels: bool,
    ) -> String {
        let config = self.to_hydro_config(show_metadata, show_location_groups, use_short_labels);
        self.render_graph_to_string(GraphFormat::Mermaid, &config)
    }

    /// Generate DOT graph as string
    pub fn dot_to_string(
        &self,
        show_metadata: bool,
        show_location_groups: bool,
        use_short_labels: bool,
    ) -> String {
        let config = self.to_hydro_config(show_metadata, show_location_groups, use_short_labels);
        self.render_graph_to_string(GraphFormat::Dot, &config)
    }

    /// Generate ReactFlow graph as string
    pub fn reactflow_to_string(
        &self,
        show_metadata: bool,
        show_location_groups: bool,
        use_short_labels: bool,
    ) -> String {
        let config = self.to_hydro_config(show_metadata, show_location_groups, use_short_labels);
        self.render_graph_to_string(GraphFormat::ReactFlow, &config)
    }

    /// Write mermaid graph to file
    pub fn mermaid_to_file(
        &self,
        filename: &str,
        show_metadata: bool,
        show_location_groups: bool,
        use_short_labels: bool,
    ) -> Result<(), Box<dyn Error>> {
        self.write_graph_to_file(
            GraphFormat::Mermaid,
            filename,
            show_metadata,
            show_location_groups,
            use_short_labels,
        )
    }

    /// Write DOT graph to file
    pub fn dot_to_file(
        &self,
        filename: &str,
        show_metadata: bool,
        show_location_groups: bool,
        use_short_labels: bool,
    ) -> Result<(), Box<dyn Error>> {
        self.write_graph_to_file(
            GraphFormat::Dot,
            filename,
            show_metadata,
            show_location_groups,
            use_short_labels,
        )
    }

    /// Write ReactFlow graph to file
    pub fn reactflow_to_file(
        &self,
        filename: &str,
        show_metadata: bool,
        show_location_groups: bool,
        use_short_labels: bool,
    ) -> Result<(), Box<dyn Error>> {
        self.write_graph_to_file(
            GraphFormat::ReactFlow,
            filename,
            show_metadata,
            show_location_groups,
            use_short_labels,
        )
    }

    /// Open mermaid graph in browser
    pub fn mermaid_to_browser(
        &self,
        show_metadata: bool,
        show_location_groups: bool,
        use_short_labels: bool,
        message_handler: Option<&dyn Fn(&str)>,
    ) -> Result<(), Box<dyn Error>> {
        self.open_browser(
            GraphFormat::Mermaid,
            show_metadata,
            show_location_groups,
            use_short_labels,
            message_handler,
        )
    }

    /// Open DOT graph in browser
    pub fn dot_to_browser(
        &self,
        show_metadata: bool,
        show_location_groups: bool,
        use_short_labels: bool,
        message_handler: Option<&dyn Fn(&str)>,
    ) -> Result<(), Box<dyn Error>> {
        self.open_browser(
            GraphFormat::Dot,
            show_metadata,
            show_location_groups,
            use_short_labels,
            message_handler,
        )
    }

    /// Open ReactFlow graph in browser
    pub fn reactflow_to_browser(
        &self,
        show_metadata: bool,
        show_location_groups: bool,
        use_short_labels: bool,
        message_handler: Option<&dyn Fn(&str)>,
    ) -> Result<(), Box<dyn Error>> {
        self.open_browser(
            GraphFormat::ReactFlow,
            show_metadata,
            show_location_groups,
            use_short_labels,
            message_handler,
        )
    }

    /// Generate all graph types and save to files with a given prefix
    pub fn generate_all_files(
        &self,
        prefix: &str,
        show_metadata: bool,
        show_location_groups: bool,
        use_short_labels: bool,
    ) -> Result<(), Box<dyn Error>> {
        let label_suffix = if use_short_labels { "_short" } else { "_long" };

        let formats = [
            GraphFormat::Mermaid,
            GraphFormat::Dot,
            GraphFormat::ReactFlow,
        ];

        for format in formats {
            let filename = format!(
                "{}{}_labels.{}",
                prefix,
                label_suffix,
                format.file_extension()
            );
            self.write_graph_to_file(
                format,
                &filename,
                show_metadata,
                show_location_groups,
                use_short_labels,
            )?;
        }

        Ok(())
    }

    /// Generate graph based on GraphConfig, delegating to the appropriate method
    #[cfg(feature = "build")]
    pub fn generate_graph_with_config(
        &self,
        config: &crate::graph::config::GraphConfig,
        message_handler: Option<&dyn Fn(&str)>,
    ) -> Result<(), Box<dyn Error>> {
        if let Some(graph_type) = config.graph {
            let format = match graph_type {
                crate::graph::config::GraphType::Mermaid => GraphFormat::Mermaid,
                crate::graph::config::GraphType::Dot => GraphFormat::Dot,
                crate::graph::config::GraphType::Reactflow => GraphFormat::ReactFlow,
            };

            self.open_browser(
                format,
                !config.no_metadata,
                !config.no_location_groups,
                !config.long_labels, // use_short_labels is the inverse of long_labels
                message_handler,
            )
        } else {
            Ok(())
        }
    }

    /// Generate all graph files based on GraphConfig
    #[cfg(feature = "build")]
    pub fn generate_all_files_with_config(
        &self,
        config: &crate::graph::config::GraphConfig,
        prefix: &str,
    ) -> Result<(), Box<dyn Error>> {
        self.generate_all_files(
            prefix,
            !config.no_metadata,
            !config.no_location_groups,
            !config.long_labels,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graph_format() {
        assert_eq!(GraphFormat::Mermaid.file_extension(), "mmd");
        assert_eq!(GraphFormat::Dot.file_extension(), "dot");
        assert_eq!(GraphFormat::ReactFlow.file_extension(), "json");

        assert_eq!(
            GraphFormat::Mermaid.browser_message(),
            "Opening Mermaid graph in browser..."
        );
        assert_eq!(
            GraphFormat::Dot.browser_message(),
            "Opening Graphviz/DOT graph in browser..."
        );
        assert_eq!(
            GraphFormat::ReactFlow.browser_message(),
            "Opening ReactFlow graph in browser..."
        );
    }

    #[test]
    fn test_graph_api_creation() {
        let ir = vec![];
        let process_id_name = vec![(0, "test_process".to_string())];
        let cluster_id_name = vec![];
        let external_id_name = vec![];

        let api = GraphApi::new(&ir, &process_id_name, &cluster_id_name, &external_id_name);

        // Test config creation
        let config = api.to_hydro_config(true, true, false);
        assert!(config.show_metadata);
        assert!(config.show_location_groups);
        assert!(!config.use_short_labels);
        assert_eq!(config.process_id_name.len(), 1);
        assert_eq!(config.process_id_name[0].1, "test_process");
    }

    #[test]
    fn test_string_generation() {
        let ir = vec![];
        let process_id_name = vec![(0, "test_process".to_string())];
        let cluster_id_name = vec![];
        let external_id_name = vec![];

        let api = GraphApi::new(&ir, &process_id_name, &cluster_id_name, &external_id_name);

        // Test that string generation methods don't panic and return some content
        let mermaid = api.mermaid_to_string(true, true, false);
        let dot = api.dot_to_string(true, true, false);
        let reactflow = api.reactflow_to_string(true, true, false);

        // These should all return non-empty strings
        assert!(!mermaid.is_empty());
        assert!(!dot.is_empty());
        assert!(!reactflow.is_empty());
    }
}
