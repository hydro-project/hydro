//! Graph visualization utilities for Hydro IR

pub mod api;
pub mod config;
pub mod debug;
pub mod graphviz;
pub mod json;
pub mod mermaid;
pub mod render;
mod template;

#[cfg(test)]
mod json_test;

#[cfg(test)]
#[cfg(feature = "viz")]
mod url_generation_test;

// Re-export only the necessary public API
pub use api::GraphApi;
pub use config::VisualizerConfig;
