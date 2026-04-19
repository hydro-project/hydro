//! File saving utilities for Hydro IR graph visualization.

use std::io::Result;

use super::render::{
    HydroWriteConfig, render_hydro_ir_dot, render_hydro_ir_json, render_hydro_ir_mermaid,
};
use crate::compile::ir::HydroRoot;

fn render_with_config<F>(
    roots: &[HydroRoot],
    config: Option<HydroWriteConfig>,
    renderer: F,
) -> String
where
    F: Fn(&[HydroRoot], HydroWriteConfig<'_>) -> String,
{
    renderer(roots, config.unwrap_or_default())
}

/// Saves Hydro IR roots as a Mermaid diagram file.
pub fn save_mermaid(
    roots: &[HydroRoot],
    filename: Option<&str>,
    config: Option<HydroWriteConfig>,
) -> Result<std::path::PathBuf> {
    let content = render_with_config(roots, config, render_hydro_ir_mermaid);
    save_to_file(content, filename, "hydro_graph.mmd")
}

/// Saves Hydro IR roots as a DOT/Graphviz file.
pub fn save_dot(
    roots: &[HydroRoot],
    filename: Option<&str>,
    config: Option<HydroWriteConfig>,
) -> Result<std::path::PathBuf> {
    let content = render_with_config(roots, config, render_hydro_ir_dot);
    save_to_file(content, filename, "hydro_graph.dot")
}

/// Saves Hydro IR roots as a JSON file.
pub fn save_json(
    roots: &[HydroRoot],
    filename: Option<&str>,
    config: Option<HydroWriteConfig>,
) -> Result<std::path::PathBuf> {
    let content = render_with_config(roots, config, render_hydro_ir_json);
    save_to_file(content, filename, "hydro_graph.json")
}

fn save_to_file(
    content: String,
    filename: Option<&str>,
    default_name: &str,
) -> Result<std::path::PathBuf> {
    let path = std::path::PathBuf::from(filename.unwrap_or(default_name));
    std::fs::write(&path, content)?;
    Ok(path)
}
