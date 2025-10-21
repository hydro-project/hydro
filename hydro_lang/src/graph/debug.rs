//! Debugging utilities for Hydro IR graph visualization.
//!
//! Similar to the DFIR debugging utilities, this module provides convenient
//! methods for opening graphs in web browsers and VS Code.

use std::fmt::Write;
use std::io::{Result, Write as IoWrite};
use std::time::{SystemTime, UNIX_EPOCH};

use super::config::VisualizerConfig;
use super::render::{HydroWriteConfig, render_hydro_ir_dot, render_hydro_ir_mermaid};
use crate::compile::ir::HydroRoot;

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

/// Compress JSON content using gzip compression.
/// Returns the compressed bytes or an error if compression fails.
#[cfg(feature = "viz")]
fn compress_json(json_content: &str) -> Result<Vec<u8>> {
    use flate2::write::GzEncoder;
    use flate2::Compression;
    
    let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
    encoder.write_all(json_content.as_bytes())?;
    encoder.finish()
}

/// Encode data to base64 URL-safe format without padding.
/// This format is safe for use in URLs and doesn't require additional escaping.
#[cfg(feature = "viz")]
fn encode_base64_url_safe(data: &[u8]) -> String {
    data_encoding::BASE64URL_NOPAD.encode(data)
}

/// Try to compress and encode JSON content for URL embedding.
/// Returns (encoded_data, is_compressed, compression_ratio).
/// 
/// Compression is skipped for small JSON (<min_compression_size bytes).
/// If compression fails or doesn't reduce size, falls back to uncompressed encoding.
#[cfg(feature = "viz")]
fn try_compress_and_encode(
    json_content: &str,
    config: &VisualizerConfig,
) -> (String, bool, f64) {
    let original_size = json_content.len();
    
    // Skip compression for small JSON
    if !config.enable_compression || original_size < config.min_compression_size {
        let encoded = encode_base64_url_safe(json_content.as_bytes());
        return (encoded, false, 1.0);
    }
    
    // Try compression
    match compress_json(json_content) {
        Ok(compressed) => {
            let compressed_size = compressed.len();
            let ratio = original_size as f64 / compressed_size as f64;
            
            // Only use compression if it actually reduces size
            if compressed_size < original_size {
                let encoded = encode_base64_url_safe(&compressed);
                println!("📦 Compressed JSON: {} bytes → {} bytes (ratio: {:.2}x)", 
                         original_size, compressed_size, ratio);
                (encoded, true, ratio)
            } else {
                // Compression didn't help, use uncompressed
                let encoded = encode_base64_url_safe(json_content.as_bytes());
                (encoded, false, 1.0)
            }
        }
        Err(e) => {
            // Compression failed, fall back to uncompressed
            println!("⚠️  Compression failed: {}, using uncompressed", e);
            let encoded = encode_base64_url_safe(json_content.as_bytes());
            (encoded, false, 1.0)
        }
    }
}

/// Calculate the total URL length for a given encoded data and parameter name.
/// Returns the total length including base URL, parameter name, and encoded data.
#[cfg(feature = "viz")]
fn calculate_url_length(base_url: &str, param_name: &str, encoded_data: &str) -> usize {
    // Format: base_url#param_name=encoded_data
    base_url.len() + 1 + param_name.len() + 1 + encoded_data.len()
}

/// Generate a URL for the visualizer with the given JSON content.
/// Automatically chooses between compressed and uncompressed encoding based on URL length.
/// Returns (url, is_compressed) or None if the URL would be too long.
#[cfg(feature = "viz")]
fn generate_visualizer_url(
    json_content: &str,
    config: &VisualizerConfig,
) -> Option<(String, bool)> {
    let (encoded_data, is_compressed, _ratio) = try_compress_and_encode(json_content, config);
    
    // Determine parameter name based on compression
    let param_name = if is_compressed { "compressed" } else { "data" };
    
    // Calculate total URL length
    let url_length = calculate_url_length(&config.base_url, param_name, &encoded_data);
    
    println!("🔗 URL length: {} characters (max: {})", url_length, config.max_url_length);
    
    // Check if URL is within length limit
    if url_length <= config.max_url_length {
        let url = format!("{}#{}={}", config.base_url, param_name, encoded_data);
        Some((url, is_compressed))
    } else {
        // If compressed URL is too long, try uncompressed as a fallback
        if is_compressed {
            println!("⚠️  Compressed URL too long, trying uncompressed...");
            let uncompressed_encoded = encode_base64_url_safe(json_content.as_bytes());
            let uncompressed_length = calculate_url_length(&config.base_url, "data", &uncompressed_encoded);
            
            if uncompressed_length <= config.max_url_length {
                println!("✓ Uncompressed URL fits: {} characters", uncompressed_length);
                let url = format!("{}#data={}", config.base_url, uncompressed_encoded);
                return Some((url, false));
            }
        }
        
        println!("❌ URL too long even with compression, will use file-based fallback");
        None
    }
}

/// Generate a timestamped filename for temporary graph files.
/// Format: hydro_graph_<timestamp>.json
#[cfg(feature = "viz")]
fn generate_timestamped_filename() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|e| {
            println!("⚠️  System time is before Unix epoch: {}. Using fallback timestamp 0.", e);
            std::time::Duration::from_secs(0)
        })
        .as_secs();
    format!("hydro_graph_{}.json", timestamp)
}

/// Save JSON content to a temporary file with a timestamped filename.
/// Returns the path to the created file.
/// 
/// Requirements: 9.1, 9.2, 9.3
#[cfg(feature = "viz")]
fn save_json_to_temp_file(json_content: &str) -> Result<std::path::PathBuf> {
    let filename = generate_timestamped_filename();
    let temp_file = std::env::temp_dir().join(filename);
    
    std::fs::write(&temp_file, json_content)?;
    
    println!("📁 Saved graph to temporary file: {}", temp_file.display());
    
    Ok(temp_file)
}

/// URL-encode a file path for safe transmission in query parameters.
/// Uses percent encoding to ensure special characters are properly escaped.
/// 
/// Requirements: 9.4
#[cfg(feature = "viz")]
fn url_encode_file_path(file_path: &std::path::Path) -> String {
    let path_str = file_path.to_string_lossy();
    urlencoding::encode(&path_str).to_string()
}

/// Generate a visualizer URL with a file query parameter.
/// Format: base_url?file=<encoded_path>
/// 
/// Requirements: 9.4, 9.5
#[cfg(feature = "viz")]
fn generate_file_based_url(file_path: &std::path::Path, config: &VisualizerConfig) -> String {
    let encoded_path = url_encode_file_path(file_path);
    format!("{}?file={}", config.base_url, encoded_path)
}

/// Print fallback instructions for manual loading of the graph file.
/// Provides clear guidance if automatic browser opening fails.
/// 
/// Requirements: 9.6, 9.7
#[cfg(feature = "viz")]
fn print_fallback_instructions(file_path: &std::path::Path, url: &str) {
    println!("\n📊 Graph Visualization Instructions");
    println!("═══════════════════════════════════════════════════════════");
    println!("The graph is too large to embed in a URL.");
    println!("It has been saved to a temporary file:");
    println!("  📁 {}", file_path.display());
    println!();
    println!("Opening visualizer in browser...");
    println!("  🌐 {}", url);
    println!();
    println!("If the browser doesn't open automatically, you can:");
    println!("  1. Manually open: {}", url);
    println!("  2. Or visit {} and drag-and-drop the file", 
             url.split('?').next().unwrap_or(url));
    println!("═══════════════════════════════════════════════════════════\n");
}

/// Handle large graph visualization using file-based fallback.
/// Saves the JSON to a temporary file and opens the visualizer with a file parameter.
/// Uses the configured base URL from VisualizerConfig.
/// 
/// Requirements: 9.1, 9.2, 9.3, 9.4, 9.5, 9.6, 9.7, 9.8, 9.9
#[cfg(feature = "viz")]
fn handle_large_graph_fallback(
    json_content: &str,
    config: &VisualizerConfig,
) -> Result<()> {
    // Save JSON to temporary file with timestamped filename
    let temp_file = save_json_to_temp_file(json_content)?;
    
    // Generate URL with file parameter using configured base URL
    let url = generate_file_based_url(&temp_file, config);
    
    // Print fallback instructions
    print_fallback_instructions(&temp_file, &url);
    
    // Try to open the visualizer in browser
    match webbrowser::open(&url) {
        Ok(_) => {
            println!("✓ Successfully opened visualizer in browser");
        }
        Err(e) => {
            println!("⚠️  Failed to open browser automatically: {}", e);
            println!("Please manually open the URL above or drag-and-drop the file.");
        }
    }
    
    Ok(())
}

/// Open JSON visualizer with automatic fallback to file-based approach for large graphs.
/// First attempts to embed the JSON in the URL using compression.
/// If the URL is too long, falls back to saving the file and using a file parameter.
/// 
/// This is the main entry point for opening JSON visualizations.
/// 
/// Requirements: 8.1-8.9, 9.1-9.9
#[cfg(feature = "viz")]
fn open_json_visualizer_with_fallback(
    json_content: &str,
    config: &VisualizerConfig,
) -> Result<()> {
    // Try to generate a URL with embedded data
    match generate_visualizer_url(json_content, config) {
        Some((url, is_compressed)) => {
            // URL fits within length limit, open it directly
            let compression_msg = if is_compressed { " (compressed)" } else { "" };
            println!("🌐 Opening visualizer with embedded data{}...", compression_msg);
            webbrowser::open(&url)?;
            println!("✓ Successfully opened visualizer in browser");
            Ok(())
        }
        None => {
            // URL too long, use file-based fallback
            println!("📦 Graph too large for URL embedding, using file-based approach...");
            handle_large_graph_fallback(json_content, config)
        }
    }
}

/// Opens Hydro IR roots as a JSON visualization in the browser.
/// Automatically handles compression and file-based fallback for large graphs.
/// 
/// This function generates JSON from the Hydro IR and opens it in the configured
/// visualizer (defaults to https://hydro.run/docs/hydroscope, can be overridden
/// with HYDRO_VISUALIZER_URL environment variable).
#[cfg(feature = "viz")]
pub fn open_json_visualizer(
    roots: &[HydroRoot],
    config: Option<HydroWriteConfig>,
) -> Result<()> {
    let json_content = render_with_config(roots, config, render_hydro_ir_json);
    let viz_config = VisualizerConfig::default();
    open_json_visualizer_with_fallback(&json_content, &viz_config)
}

/// Opens Hydro IR roots as a JSON visualization with custom visualizer configuration.
/// Allows specifying a custom base URL and compression settings.
#[cfg(feature = "viz")]
pub fn open_json_visualizer_with_config(
    roots: &[HydroRoot],
    config: Option<HydroWriteConfig>,
    viz_config: VisualizerConfig,
) -> Result<()> {
    let json_content = render_with_config(roots, config, render_hydro_ir_json);
    open_json_visualizer_with_fallback(&json_content, &viz_config)
}

/// Saves Hydro IR roots as a JSON file.
/// If no filename is provided, saves to temporary directory.
pub fn save_json(
    roots: &[HydroRoot],
    filename: Option<&str>,
    config: Option<HydroWriteConfig>,
) -> Result<std::path::PathBuf> {
    let content = render_with_config(roots, config, render_hydro_ir_json);
    save_to_file(content, filename, "hydro_graph.json", "JSON file")
}

/// Helper function to render multiple Hydro IR roots as JSON.
fn render_hydro_ir_json(roots: &[HydroRoot], config: &HydroWriteConfig) -> String {
    super::render::render_hydro_ir_json(roots, config)
}
