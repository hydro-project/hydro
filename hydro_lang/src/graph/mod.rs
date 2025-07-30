//! Graph visualization utilities for Hydro IR

pub mod api;
pub mod debug;
pub mod dot;
pub mod json;
pub mod mermaid;
pub mod render;
pub mod template;

// Re-export only the necessary public API
pub use api::GraphApi;
