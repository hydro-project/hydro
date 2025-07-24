//! Debugging utilities for Hydro IR graph visualization.
//!
//! Similar to the DFIR debugging utilities, this module provides convenient
//! methods for opening graphs in web browsers and VS Code.

use std::fmt::Write;
use std::io::Result;

use super::render::{HydroWriteConfig, render_hydro_ir_dot, render_hydro_ir_mermaid};
use super::template::get_template;
use crate::ir::HydroLeaf;

/// Opens Hydro IR leaves as a single mermaid diagram.
pub fn open_mermaid(leaves: &[HydroLeaf], config: Option<HydroWriteConfig>) -> Result<()> {
    let mermaid_src = render_with_config(leaves, config, render_hydro_ir_mermaid);
    open_mermaid_browser(&mermaid_src)
}

/// Opens Hydro IR leaves as a single DOT diagram.
pub fn open_dot(leaves: &[HydroLeaf], config: Option<HydroWriteConfig>) -> Result<()> {
    let dot_src = render_with_config(leaves, config, render_hydro_ir_dot);
    open_dot_browser(&dot_src)
}

/// Opens Hydro IR leaves as a ReactFlow.js visualization in a browser.
/// Creates a complete HTML file with ReactFlow.js interactive graph visualization.
pub fn open_reactflow_browser(
    leaves: &[HydroLeaf],
    filename: Option<&str>,
    config: Option<HydroWriteConfig>,
) -> Result<()> {
    let reactflow_json = render_with_config(leaves, config, render_hydro_ir_reactflow);
    let filename = filename.unwrap_or("hydro_graph.html");
    save_and_open_reactflow_browser(&reactflow_json, filename)
}

/// Saves Hydro IR leaves as a ReactFlow.js JSON file.
/// If no filename is provided, saves to temporary directory.
pub fn save_reactflow_json(
    leaves: &[HydroLeaf],
    filename: Option<&str>,
    config: Option<HydroWriteConfig>,
) -> Result<std::path::PathBuf> {
    let content = render_with_config(leaves, config, render_hydro_ir_reactflow);
    save_to_file(content, filename, "hydro_graph.json", "ReactFlow.js JSON")
}

/// Saves Hydro IR leaves as a Mermaid diagram file.
/// If no filename is provided, saves to temporary directory.
pub fn save_mermaid(
    leaves: &[HydroLeaf],
    filename: Option<&str>,
    config: Option<HydroWriteConfig>,
) -> Result<std::path::PathBuf> {
    let content = render_with_config(leaves, config, render_hydro_ir_mermaid);
    save_to_file(content, filename, "hydro_graph.mermaid", "Mermaid diagram")
}

/// Saves Hydro IR leaves as a DOT/Graphviz file.
/// If no filename is provided, saves to temporary directory.
pub fn save_dot(
    leaves: &[HydroLeaf],
    filename: Option<&str>,
    config: Option<HydroWriteConfig>,
) -> Result<std::path::PathBuf> {
    let content = render_with_config(leaves, config, render_hydro_ir_dot);
    save_to_file(content, filename, "hydro_graph.dot", "DOT/Graphviz file")
}

fn open_mermaid_browser(mermaid_src: &str) -> Result<()> {
    // Debug: Print the mermaid source being sent to browser
    println!("=== MERMAID SOURCE BEING SENT TO BROWSER ===");
    println!("{}", mermaid_src);
    println!("=== END MERMAID SOURCE ===");

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
/// Creates files in temporary directory to avoid cluttering the workspace.
pub fn save_and_open_reactflow_browser(reactflow_json: &str, filename: &str) -> Result<()> {
    let template = get_template();
    let html_content = template.replace("{{GRAPH_DATA}}", reactflow_json);

    // Create file in temporary directory
    let temp_file = save_to_file(html_content, None, filename, "HTML/Reactflow JS file").unwrap();
    println!("Got path {}", temp_file.display());

    // Open the HTML file in browser
    let file_url = format!("file://{}", temp_file.display());
    webbrowser::open(&file_url)?;

    println!("Opened Enhanced ReactFlow.js visualization in browser.");
    Ok(())
}

/// Helper function to render multiple Hydro IR leaves as ReactFlow.js JSON.
fn render_hydro_ir_reactflow(leaves: &[HydroLeaf], config: &HydroWriteConfig) -> String {
    super::render::render_hydro_ir_reactflow(leaves, config)
}

/// Helper function to save content to a file with consistent path handling.
/// If no filename is provided, saves to temporary directory with the default name.
fn save_to_file(
    content: String,
    filename: Option<&str>,
    default_name: &str,
    content_type: &str,
) -> Result<std::path::PathBuf> {
    let file_path = if let Some(filename) = filename {
        std::path::PathBuf::from(filename)
    } else {
        std::env::temp_dir().join(default_name)
    };

    std::fs::write(&file_path, content)?;
    println!("Saved {} to {}", content_type, file_path.display());
    Ok(file_path)
}

/// Helper function to handle config unwrapping and rendering.
fn render_with_config<F>(
    leaves: &[HydroLeaf],
    config: Option<HydroWriteConfig>,
    renderer: F,
) -> String
where
    F: Fn(&[HydroLeaf], &HydroWriteConfig) -> String,
{
    let config = config.unwrap_or_default();
    renderer(leaves, &config)
}
