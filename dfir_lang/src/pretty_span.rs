//! Pretty, human-readable printing of [`proc_macro2::Span`]s.

use std::path::Path;

extern crate proc_macro;

/// Helper struct which displays the span as `path:row:col` for human reading/IDE linking.
/// Example: `dfir\tests\surface_syntax.rs:42:18`.
pub struct PrettySpan(pub proc_macro2::Span);
impl std::fmt::Display for PrettySpan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #[cfg(nightly)]
        if proc_macro::is_available() {
            let span = self.0.unwrap();
            write!(
                f,
                "{}:{}:{}",
                make_source_path_relative(&span.source_file().path()).display(),
                span.start().line(),
                span.start().column(),
            )?;
            return Ok(());
        }

        write!(
            f,
            "unknown:{}:{}",
            self.0.start().line,
            self.0.start().column
        )
    }
}

/// Helper struct which displays the span as `row:col` for human reading.
pub struct PrettyRowCol(pub proc_macro2::Span);
impl std::fmt::Display for PrettyRowCol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let span = self.0;
        write!(f, "{}:{}", span.start().line, span.start().column)
    }
}

/// Strip `DFIR_BASE_DIR` or `CARGO_MANIFEST_DIR` from the path prefix if possible.
pub fn make_source_path_relative(source_path: &Path) -> &Path {
    std::env::var_os("DFIR_BASE_DIR")
        .and_then(|base_dir| {
            let base_dir = std::fs::canonicalize(base_dir).ok()?;
            source_path.strip_prefix(base_dir).ok()
        })
        .or_else(|| {
            let manifest_dir = std::env::var_os("CARGO_MANIFEST_DIR")?;
            source_path.strip_prefix(manifest_dir).ok()
        })
        .unwrap_or(source_path)
}
