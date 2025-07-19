//! Debugging utilities for Hydro IR graph visualization.
//!
//! Similar to the DFIR debugging utilities, this module provides convenient
//! methods for opening graphs in web browsers and VS Code.

use std::fmt::Write;
use std::io::Result;

use super::render::{HydroWriteConfig, render_hydro_ir_dot, render_hydro_ir_mermaid};
use super::template::get_template;
use crate::ir::HydroLeaf;

/// Debugging extensions for Hydro IR.
impl HydroLeaf {
    /// Opens this Hydro IR graph as a mermaid diagram in the [mermaid.live](https://mermaid.live) browser editor.
    pub fn open_mermaid(&self, config: Option<HydroWriteConfig>) -> Result<()> {
        let config = config.unwrap_or_default();
        let mermaid_src = self.to_mermaid(&config);
        open_mermaid_browser(&mermaid_src)
    }

    /// Opens this Hydro IR graph as a DOT/Graphviz diagram in the browser.
    pub fn open_dot(&self, config: Option<HydroWriteConfig>) -> Result<()> {
        let config = config.unwrap_or_default();
        let dot_src = self.to_dot(&config);
        open_dot_browser(&dot_src)
    }

    /// Saves this Hydro IR graph as a ReactFlow.js JSON file and opens it in a browser.
    /// Creates a complete HTML file with ReactFlow.js visualization.
    pub fn open_reactflow_browser(
        &self,
        filename: Option<&str>,
        config: Option<HydroWriteConfig>,
    ) -> Result<()> {
        let config = config.unwrap_or_default();
        let reactflow_json = self.to_reactflow(&config);
        let filename = filename.unwrap_or("hydro_graph.html");
        save_and_open_reactflow_browser(&reactflow_json, filename)
    }

    /// Saves this Hydro IR graph as a ReactFlow.js JSON file.
    pub fn save_reactflow_json(
        &self,
        filename: Option<&str>,
        config: Option<HydroWriteConfig>,
    ) -> Result<()> {
        let config = config.unwrap_or_default();
        let reactflow_json = self.to_reactflow(&config);
        let filename = filename.unwrap_or("hydro_graph.json");
        std::fs::write(filename, reactflow_json)?;
        Ok(())
    }
}

/// Opens multiple Hydro IR leaves as a single mermaid diagram.
pub fn open_hydro_ir_mermaid(leaves: &[HydroLeaf], config: Option<HydroWriteConfig>) -> Result<()> {
    let config = config.unwrap_or_default();
    let mermaid_src = render_hydro_ir_mermaid(leaves, &config);
    open_mermaid_browser(&mermaid_src)
}

/// Opens multiple Hydro IR leaves as a single DOT diagram.
pub fn open_hydro_ir_dot(leaves: &[HydroLeaf], config: Option<HydroWriteConfig>) -> Result<()> {
    let config = config.unwrap_or_default();
    let dot_src = render_hydro_ir_dot(leaves, &config);
    open_dot_browser(&dot_src)
}

/// Opens multiple Hydro IR leaves as a ReactFlow.js visualization in a browser.
/// Creates a complete HTML file with ReactFlow.js interactive graph visualization.
pub fn open_hydro_ir_reactflow_browser(
    leaves: &[HydroLeaf],
    filename: Option<&str>,
    config: Option<HydroWriteConfig>,
) -> Result<()> {
    let config = config.unwrap_or_default();
    let reactflow_json = render_hydro_ir_reactflow(leaves, &config);
    let filename = filename.unwrap_or("hydro_graph.html");
    save_and_open_reactflow_browser(&reactflow_json, filename)
}

/// Saves multiple Hydro IR leaves as a ReactFlow.js JSON file.
pub fn save_hydro_ir_reactflow_json(
    leaves: &[HydroLeaf],
    filename: Option<&str>,
    config: Option<HydroWriteConfig>,
) -> Result<()> {
    let config = config.unwrap_or_default();
    let reactflow_json = render_hydro_ir_reactflow(leaves, &config);
    let filename = filename.unwrap_or("hydro_graph.json");
    std::fs::write(filename, reactflow_json)?;
    println!("Saved ReactFlow.js JSON to {}", filename);
    Ok(())
}

fn open_mermaid_browser(mermaid_src: &str) -> Result<()> {
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

fn open_dot_browser(dot_src: &str) -> Result<()> {
    let mut url = "https://dreampuf.github.io/GraphvizOnline/#".to_owned();
    for byte in dot_src.bytes() {
        // Lazy percent encoding: https://en.wikipedia.org/wiki/Percent-encoding
        write!(url, "%{:02x}", byte).unwrap();
    }
    webbrowser::open(&url)
}

/// Helper function to create a complete HTML file with ReactFlow.js visualization and open it in browser.
pub fn save_and_open_reactflow_browser(reactflow_json: &str, filename: &str) -> Result<()> {
    let template = get_template();
    let html_content = template.replace("{{GRAPH_DATA}}", reactflow_json);

    std::fs::write(filename, html_content)?;
    println!("Saved Enhanced ReactFlow.js visualization to {}", filename);

    // Open the HTML file in VS Code editor and in browser
    use std::process::Command;

    let file_path = std::env::current_dir()?.join(filename);
    let file_url = format!("file://{}", file_path.display());

    // Open the file in VS Code editor
    let _ = Command::new("code").arg(&file_path).status();

    // Also open in default browser for immediate viewing
    webbrowser::open(&file_url)?;

    println!("Opened Enhanced ReactFlow.js visualization in browser.");
    Ok(())
}

/// Helper function to render multiple Hydro IR leaves as ReactFlow.js JSON.
fn render_hydro_ir_reactflow(leaves: &[HydroLeaf], config: &HydroWriteConfig) -> String {
    super::render::render_hydro_ir_reactflow(leaves, config)
}
