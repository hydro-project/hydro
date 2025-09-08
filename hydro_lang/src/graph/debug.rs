//! Debugging utilities for Hydro IR graph visualization.
//!
//! Similar to the DFIR debugging utilities, this module provides convenient
//! methods for opening graphs in web browsers and VS Code.

use std::fmt::Write;
use std::io::Result;

use super::render::{HydroWriteConfig, render_hydro_ir_dot, render_hydro_ir_mermaid};
use crate::ir::HydroRoot;

/// URLs longer than ~8000 characters may fail in some browsers.
/// With modern JSON compression, we can afford a higher limit.
/// Use a limit of 4000 characters for the base URL + encoded data.
const MAX_SAFE_URL_LENGTH: usize = 4000;

/// Opens Hydro IR roots as a single mermaid diagram.
pub fn open_mermaid(roots: &[HydroRoot], config: Option<HydroWriteConfig>) -> Result<()> {
    let mermaid_src = render_with_config(roots, config, render_hydro_ir_mermaid);
    open_mermaid_browser(&mermaid_src)
}

/// Opens Hydro IR roots as a single DOT diagram.
pub fn open_dot(roots: &[HydroRoot], config: Option<HydroWriteConfig>) -> Result<()> {
    let dot_src = render_with_config(roots, config, render_hydro_ir_dot);
    open_dot_browser(&dot_src)
}

/// Opens Hydro IR roots by passing JSON to a visualization in a browser.
/// Creates a complete HTML file with interactive graph visualization.
pub fn open_json_browser(
    roots: &[HydroRoot],
    filename: Option<&str>,
    config: Option<HydroWriteConfig>,
) -> Result<()> {
    let json_content = render_with_config(roots, config, render_hydro_ir_json);
    let filename = filename.unwrap_or("hydro_graph.html");
    save_and_open_json_browser(&json_content, filename)
}

/// Saves Hydro IR roots as a JSON file.
/// If no filename is provided, saves to temporary directory.
pub fn save_json(
    roots: &[HydroRoot],
    filename: Option<&str>,
    config: Option<HydroWriteConfig>,
) -> Result<std::path::PathBuf> {
    let content = render_with_config(roots, config, render_hydro_ir_json);
    save_to_file(content, filename, "hydro_graph.json", "JSON")
}

/// Saves Hydro IR roots as a Mermaid diagram file.
/// If no filename is provided, saves to temporary directory.
pub fn save_mermaid(
    roots: &[HydroRoot],
    filename: Option<&str>,
    config: Option<HydroWriteConfig>,
) -> Result<std::path::PathBuf> {
    let content = render_with_config(roots, config, render_hydro_ir_mermaid);
    save_to_file(content, filename, "hydro_graph.mermaid", "Mermaid diagram")
}

/// Saves Hydro IR roots as a DOT/Graphviz file.
/// If no filename is provided, saves to temporary directory.
pub fn save_dot(
    roots: &[HydroRoot],
    filename: Option<&str>,
    config: Option<HydroWriteConfig>,
) -> Result<std::path::PathBuf> {
    let content = render_with_config(roots, config, render_hydro_ir_dot);
    save_to_file(content, filename, "hydro_graph.dot", "DOT/Graphviz file")
}

/// Opens Hydro IR roots by passing JSON to a visualization in a browser.
/// Uses URL compression when possible, falls back to file approach for large graphs.
pub fn open_json_visualizer(roots: &[HydroRoot], config: Option<HydroWriteConfig>) -> Result<()> {
    let json_content = render_with_config(roots, config, render_hydro_ir_json);

    // Use the centralized compression and URL logic
    open_json_visualizer_with_fallback(&json_content, "JSON visualizer")
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

/// Helper function to create a complete HTML file with JSON visualization and open it in browser.
/// Creates files in temporary directory to avoid cluttering the workspace.
pub fn save_and_open_json_browser(json_content: &str, _filename: &str) -> Result<()> {
    open_json_visualizer_with_fallback(json_content, "JSON visualizer in docs")
}

/// Centralized function to handle JSON visualization with URL length checks and fallbacks.
fn open_json_visualizer_with_fallback(json_content: &str, context_name: &str) -> Result<()> {
    #[cfg(feature = "viz")]
    {
        use data_encoding::BASE64URL_NOPAD;

        // Try compression first for large JSON
        let (encoded_data, is_compressed) = if json_content.len() > 1000 {
            match compress_json(json_content) {
                Ok(compressed) => {
                    let encoded = BASE64URL_NOPAD.encode(&compressed);
                    println!(
                        "📦 Compressed JSON from {} to {} bytes",
                        json_content.len(),
                        compressed.len()
                    );
                    (encoded, true)
                }
                Err(_) => {
                    // Fallback to uncompressed
                    (BASE64URL_NOPAD.encode(json_content.as_bytes()), false)
                }
            }
        } else {
            (BASE64URL_NOPAD.encode(json_content.as_bytes()), false)
        };

        let base_url_length = if is_compressed {
            "https://hydro.run/docs/hydroscope#compressed=".len()
        } else {
            "https://hydro.run/docs/hydroscope#data=".len()
        };

        if base_url_length + encoded_data.len() > MAX_SAFE_URL_LENGTH {
            // Still too large even with compression - save to temp file
            handle_large_graph_fallback(json_content, encoded_data.len())?;
            return Ok(());
        }

        // Small enough - use URL encoding
        try_open_visualizer_urls(&encoded_data, context_name, is_compressed)?;
    }

    #[cfg(not(feature = "viz"))]
    {
        println!("viz feature not enabled, cannot open browser");
    }

    Ok(())
}

/// Handle the case where the graph is too large for URL encoding.
fn handle_large_graph_fallback(json_content: &str, encoded_size: usize) -> Result<()> {
    let temp_file = save_json_to_temp(json_content)?;

    println!(
        "📊 Graph is too large for URL encoding ({} chars)",
        encoded_size
    );
    println!("� Saved graph to: {}", temp_file.display());
    println!("🌐 Opening visualizer...");

    // URL encode the file path
    let file_path_str = temp_file.to_string_lossy();
    let encoded_path = urlencoding::encode(&file_path_str);

    // Try to open localhost first with file parameter, fall back to docs site
    let localhost_url = format!("http://localhost:3000/hydroscope?file={}", encoded_path);
    let docs_url = format!("https://hydro.run/docs/hydroscope?file={}", encoded_path);

    if webbrowser::open(&localhost_url).is_ok() {
        println!("✅ Opened visualizer: {}", localhost_url);
    } else if webbrowser::open(&docs_url).is_ok() {
        println!("✅ Opened visualizer: {}", docs_url);
    } else {
        println!("❌ Failed to open browser");
        println!("🎯 Please manually open the visualizer and load the file:");
        println!("   1. Open https://hydro.run/docs/hydroscope");
        println!("   2. Drag and drop the JSON file onto the visualizer");
        println!("   3. Or use the file upload button in the visualizer");
    }

    println!("💡 The generated file path is shown above for easy loading");

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

/// Try to open the visualizer URLs, with localhost fallback to docs site.
fn try_open_visualizer_urls(
    encoded_data: &str,
    context_name: &str,
    is_compressed: bool,
) -> Result<()> {
    let url_param = if is_compressed { "compressed" } else { "data" };
    let localhost_url = format!(
        "http://localhost:3000/hydroscope#{}={}",
        url_param, encoded_data
    );
    let docs_url = format!(
        "https://hydro.run/docs/hydroscope#{}={}",
        url_param, encoded_data
    );

    // Show the compressed URL for debugging
    if is_compressed {
        println!("📦 Compressed URL length: {} characters", docs_url.len());
        println!(
            "🔗 Compressed URL: {}",
            if docs_url.len() > 200 {
                format!(
                    "{}...{}",
                    &docs_url[..100],
                    &docs_url[docs_url.len() - 50..]
                )
            } else {
                docs_url.clone()
            }
        );
    }

    // Try to open localhost first
    match webbrowser::open(&localhost_url) {
        Ok(_) => {
            println!("Opened {} (localhost): {}", context_name, localhost_url);
        }
        Err(_) => {
            // If localhost fails, try the main docs site
            webbrowser::open(&docs_url)?;
            println!("Opened {}: {}", context_name, docs_url);
        }
    }
    Ok(())
}

/// Helper function to render multiple Hydro IR roots as ReactFlow.js JSON.
fn render_hydro_ir_json(roots: &[HydroRoot], config: &HydroWriteConfig) -> String {
    super::render::render_hydro_ir_json(roots, config)
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
    roots: &[HydroRoot],
    config: Option<HydroWriteConfig>,
    renderer: F,
) -> String
where
    F: Fn(&[HydroRoot], &HydroWriteConfig) -> String,
{
    let config = config.unwrap_or_default();
    renderer(roots, &config)
}

/// Compress JSON data using gzip compression
#[cfg(feature = "viz")]
fn compress_json(json_content: &str) -> Result<Vec<u8>> {
    use std::io::Write;

    use flate2::Compression;
    use flate2::write::GzEncoder;

    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(json_content.as_bytes())?;
    encoder.finish()
}
