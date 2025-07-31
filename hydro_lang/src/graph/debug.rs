//! Debugging utilities for Hydro IR graph visualization.
//!
//! Similar to the DFIR debugging utilities, this module provides convenient
//! methods for opening graphs in web browsers and VS Code.

use std::fmt::Write;
use std::io::Result;

use super::render::{HydroWriteConfig, render_hydro_ir_dot, render_hydro_ir_mermaid};
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

/// Opens Hydro IR leaves by passing JSON to a visualization in a browser.
/// Creates a complete HTML file with interactive graph visualization.
pub fn open_json_browser(
    leaves: &[HydroLeaf],
    filename: Option<&str>,
    config: Option<HydroWriteConfig>,
) -> Result<()> {
    let json_content = render_with_config(leaves, config, render_hydro_ir_json);
    let filename = filename.unwrap_or("hydro_graph.html");
    save_and_open_json_browser(&json_content, filename)
}

/// Saves Hydro IR leaves as a JSON file.
/// If no filename is provided, saves to temporary directory.
pub fn save_json(
    leaves: &[HydroLeaf],
    filename: Option<&str>,
    config: Option<HydroWriteConfig>,
) -> Result<std::path::PathBuf> {
    let content = render_with_config(leaves, config, render_hydro_ir_json);
    save_to_file(content, filename, "hydro_graph.json", "JSON")
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

/// Opens Hydro IR leaves in the new JSON visualizer in browser.
/// Generates JSON and opens it via URL encoding in the docs visualizer.
pub fn open_json_visualizer(leaves: &[HydroLeaf], config: Option<HydroWriteConfig>) -> Result<()> {
    let json_content = render_with_config(leaves, config, render_hydro_ir_json);

    // Debug: Print a snippet of the JSON to see if backtrace is included
    println!("=== JSON CONTENT SAMPLE (for backtrace verification) ===");
    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&json_content) {
        if let Some(nodes) = parsed["nodes"].as_array() {
            if let Some(first_node) = nodes.first() {
                if let Some(backtrace) = first_node["data"]["backtrace"].as_array() {
                    println!("First node has backtrace with {} elements", backtrace.len());
                    if !backtrace.is_empty() {
                        println!(
                            "Sample backtrace element: {}",
                            serde_json::to_string_pretty(&backtrace[0]).unwrap_or_default()
                        );
                    }
                } else {
                    println!("First node does not have backtrace data");
                }
            }
        }
    }
    println!("=== END JSON SAMPLE ===");

    open_json_browser_impl(&json_content)
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

fn open_json_browser_impl(json_content: &str) -> Result<()> {
    #[cfg(feature = "viz")]
    {
        use data_encoding::BASE64URL_NOPAD;

        // Encode the JSON data for URL
        let encoded_data = BASE64URL_NOPAD.encode(json_content.as_bytes());

        // URLs longer than ~2000 characters may fail in some browsers
        // Use a conservative limit of 1800 characters for the base URL + encoded data
        const MAX_SAFE_URL_LENGTH: usize = 1800;
        let base_url_length = "https://hydro.run/docs/visualizer#data=".len();
        
        if base_url_length + encoded_data.len() > MAX_SAFE_URL_LENGTH {
            // Large graph - save to temp file and give instructions
            let temp_file = save_json_to_temp(json_content)?;
            
            println!("📊 Graph is too large for URL encoding ({} chars)", encoded_data.len());
            println!("💾 Saved JSON to temporary file: {}", temp_file.display());
            println!();
            println!("🎯 To visualize this graph:");
            println!("   1. Open https://hydro.run/docs/visualizer");
            println!("   2. Drag and drop the JSON file onto the visualizer");
            println!("   3. Or use the file upload button in the visualizer");
            println!();
            println!("💡 Alternatively, you can copy the file path above and use it with your preferred method.");
            
            return Ok(());
        }

        // Small graph - use URL encoding as before
        // Try localhost first (for development), then fall back to docs site
        let localhost_url = format!("http://localhost:3000/visualizer#data={}", encoded_data);
        let docs_url = format!("https://hydro.run/docs/visualizer#data={}", encoded_data);

        // Try to open localhost first
        match webbrowser::open(&localhost_url) {
            Ok(_) => {
                println!("Opened new JSON visualizer (localhost): {}", localhost_url);
            }
            Err(_) => {
                // If localhost fails, try the main docs site
                webbrowser::open(&docs_url)?;
                println!("Opened new JSON visualizer: {}", docs_url);
            }
        }
    }

    #[cfg(not(feature = "viz"))]
    {
        println!("viz feature not enabled, cannot open browser");
    }

    Ok(())
}

/// Save JSON content to a temporary file with a descriptive name
fn save_json_to_temp(json_content: &str) -> Result<std::path::PathBuf> {
    use std::io::Write;
    
    // Create a descriptive filename with timestamp
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    let filename = format!("hydro_graph_{}.json", timestamp);
    let temp_file = std::env::temp_dir().join(filename);
    
    // Write the JSON content to the temp file
    let mut file = std::fs::File::create(&temp_file)?;
    file.write_all(json_content.as_bytes())?;
    file.flush()?;
    
    Ok(temp_file)
}

/// Helper function to create a complete HTML file with JSON visualization and open it in browser.
/// Creates files in temporary directory to avoid cluttering the workspace.
pub fn save_and_open_json_browser(json_content: &str, _filename: &str) -> Result<()> {
    // Use the new docs-based approach instead of creating temp HTML files
    #[cfg(feature = "viz")]
    {
        use data_encoding::BASE64URL_NOPAD;

        // Encode the JSON data for URL
        let encoded_data = BASE64URL_NOPAD.encode(json_content.as_bytes());

        // URLs longer than ~2000 characters may fail in some browsers
        // Use a conservative limit of 1800 characters for the base URL + encoded data
        const MAX_SAFE_URL_LENGTH: usize = 1800;
        let base_url_length = "https://hydro.run/docs/visualizer#data=".len();
        
        if base_url_length + encoded_data.len() > MAX_SAFE_URL_LENGTH {
            // Large graph - save to temp file and give instructions
            let temp_file = save_json_to_temp(json_content)?;
            
            println!("📊 Graph is too large for URL encoding ({} chars)", encoded_data.len());
            println!("💾 Saved JSON to temporary file: {}", temp_file.display());
            println!();
            println!("🎯 To visualize this graph:");
            println!("   1. Open https://hydro.run/docs/visualizer");
            println!("   2. Drag and drop the JSON file onto the visualizer");
            println!("   3. Or use the file upload button in the visualizer");
            println!();
            println!("💡 Alternatively, you can copy the file path above and use it with your preferred method.");
            
            return Ok(());
        }

        // Small graph - use URL encoding as before
        // Try localhost first (for development), then fall back to docs site
        let localhost_url = format!("http://localhost:3000/visualizer#data={}", encoded_data);
        let docs_url = format!("https://hydro.run/docs/visualizer#data={}", encoded_data);

        // Try to open localhost first
        match webbrowser::open(&localhost_url) {
            Ok(_) => {
                println!(
                    "Opened JSON visualizer in docs (localhost): {}",
                    localhost_url
                );
            }
            Err(_) => {
                // If localhost fails, try the main docs site
                webbrowser::open(&docs_url)?;
                println!("Opened JSON visualizer in docs: {}", docs_url);
            }
        }
    }

    #[cfg(not(feature = "viz"))]
    {
        println!("viz feature not enabled, cannot open browser");
    }

    Ok(())
}

/// Helper function to render multiple Hydro IR leaves as ReactFlow.js JSON.
fn render_hydro_ir_json(leaves: &[HydroLeaf], config: &HydroWriteConfig) -> String {
    super::render::render_hydro_ir_json(leaves, config)
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
