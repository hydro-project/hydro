#![cfg(feature = "debugging")]

use std::fmt::Write;
use std::io::Result;

use super::hydroflow_graph::WriteConfig;
use super::{DfirGraph, WriteGraphType};

impl DfirGraph {
    /// Opens this as a mermaid graph in the [mermaid.live](https://mermaid.live) browser editor.
    pub fn open_mermaid(&self, write_config: &WriteConfig) -> Result<()> {
        let mermaid_src = self.to_mermaid(write_config);
        let state = serde_json::json!({
            "code": mermaid_src,
            "mermaid": serde_json::json!({
                "theme": "default"
            }),
            "autoSync": true,
            "updateDiagram": true
        });
        let state_json = serde_json::to_vec(&state)?;
        let state_base64 = data_encoding::BASE64URL.encode(&state_json);
        webbrowser::open(&format!(
            "https://mermaid.live/edit#base64:{}",
            state_base64
        ))
    }

    /// Opens this as dot/graphviz graph in the [Graphviz Online](https://dreampuf.github.io/GraphvizOnline/#)
    /// browser editor.
    pub fn open_dot(&self, write_config: &WriteConfig) -> Result<()> {
        let dot_src = self.to_dot(write_config);
        let mut url = "https://dreampuf.github.io/GraphvizOnline/#".to_owned();
        for byte in dot_src.bytes() {
            // Lazy percent encoding: https://en.wikipedia.org/wiki/Percent-encoding
            write!(url, "%{:02x}", byte).unwrap();
        }
        webbrowser::open(&url)
    }

    /// Opens the graph based on `graph_type`, which can be parsed by clap.
    pub fn open_graph(
        &self,
        graph_type: WriteGraphType,
        write_config: Option<WriteConfig>,
    ) -> Result<()> {
        let write_config = &write_config.unwrap_or_default();
        match graph_type {
            WriteGraphType::Mermaid => self.open_mermaid(write_config),
            WriteGraphType::Dot => self.open_dot(write_config),
        }
    }
}
