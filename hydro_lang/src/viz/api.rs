use std::error::Error;

use slotmap::SecondaryMap;

use crate::compile::ir::HydroRoot;
use crate::location::LocationKey;
use crate::viz::render::{
    HydroWriteConfig, render_hydro_ir_dot, render_hydro_ir_json, render_hydro_ir_mermaid,
};

/// Graph generation API for built flows.
pub struct GraphApi<'a> {
    ir: &'a [HydroRoot],
    location_names: &'a SecondaryMap<LocationKey, String>,
}

impl<'a> GraphApi<'a> {
    pub fn new(ir: &'a [HydroRoot], location_names: &'a SecondaryMap<LocationKey, String>) -> Self {
        Self { ir, location_names }
    }

    fn config(&self, use_short_labels: bool, show_metadata: bool) -> HydroWriteConfig<'a> {
        HydroWriteConfig {
            show_metadata,
            show_location_groups: true,
            use_short_labels,
            location_names: self.location_names,
        }
    }

    /// Render graph to string in the given format.
    pub fn render(
        &self,
        format: crate::viz::config::GraphType,
        use_short_labels: bool,
        show_metadata: bool,
    ) -> String {
        let config = self.config(use_short_labels, show_metadata);
        match format {
            crate::viz::config::GraphType::Mermaid => render_hydro_ir_mermaid(self.ir, config),
            crate::viz::config::GraphType::Dot => render_hydro_ir_dot(self.ir, config),
            crate::viz::config::GraphType::Json => render_hydro_ir_json(self.ir, config),
        }
    }

    /// Write graph to file.
    pub fn write_to_file(
        &self,
        format: crate::viz::config::GraphType,
        filename: &str,
        use_short_labels: bool,
        show_metadata: bool,
    ) -> Result<(), Box<dyn Error>> {
        let content = self.render(format, use_short_labels, show_metadata);
        std::fs::write(filename, content)?;
        Ok(())
    }

    /// Generate graph based on CLI GraphConfig. Returns Some(path) if a file was written.
    #[cfg(feature = "build")]
    pub fn generate_graph(
        &self,
        config: &crate::viz::config::GraphConfig,
    ) -> Result<Option<String>, Box<dyn Error>> {
        let Some(graph_type) = config.graph else {
            return Ok(None);
        };
        let filename = config
            .output
            .clone()
            .unwrap_or_else(|| format!("hydro_graph.{}", graph_type.file_extension()));
        self.write_to_file(
            graph_type,
            &filename,
            !config.long_labels,
            !config.no_metadata,
        )?;
        Ok(Some(filename))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_generation() {
        let ir = vec![];
        let mut location_names = SecondaryMap::new();
        let loc_key_1 = LocationKey::TEST_KEY_1;
        location_names.insert(loc_key_1, "test_process".to_owned());

        let api = GraphApi::new(&ir, &location_names);

        let mermaid = api.render(crate::viz::config::GraphType::Mermaid, true, true);
        let dot = api.render(crate::viz::config::GraphType::Dot, true, true);
        let json = api.render(crate::viz::config::GraphType::Json, true, true);

        assert!(!mermaid.is_empty());
        assert!(!dot.is_empty());
        assert!(!json.is_empty());
    }
}
