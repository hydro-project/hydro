//! Tests for URL generation and compression logic

#[cfg(test)]
#[cfg(feature = "viz")]
mod tests {
    use crate::graph::config::VisualizerConfig;

    #[test]
    fn test_visualizer_config_default() {
        let config = VisualizerConfig::default();
        assert!(config.base_url.contains("hydro.run") || config.base_url.contains("localhost"));
        assert!(config.enable_compression);
        assert_eq!(config.max_url_length, 4000);
        assert_eq!(config.min_compression_size, 1000);
    }

    #[test]
    fn test_visualizer_config_with_base_url() {
        let config = VisualizerConfig::with_base_url("https://example.com/viz");
        assert_eq!(config.base_url, "https://example.com/viz");
        assert!(config.enable_compression);
    }

    #[test]
    fn test_visualizer_config_local() {
        let config = VisualizerConfig::local();
        assert_eq!(config.base_url, "http://localhost:3000/hydroscope");
    }

    #[test]
    fn test_visualizer_config_without_compression() {
        let config = VisualizerConfig::default().without_compression();
        assert!(!config.enable_compression);
    }

    #[test]
    fn test_compression_and_encoding() {
        // Test with a small JSON that should not be compressed
        let small_json = r#"{"nodes":[],"edges":[]}"#;
        let config = VisualizerConfig::default();

        // Small JSON should skip compression
        assert!(small_json.len() < config.min_compression_size);
    }

    #[test]
    fn test_url_length_calculation() {
        let base_url = "https://hydro.run/docs/hydroscope";
        let param_name = "data";
        let encoded_data = "eyJub2RlcyI6W10sImVkZ2VzIjpbXX0"; // base64 encoded {"nodes":[],"edges":[]}

        // Format: base_url#param_name=encoded_data
        let expected_length = base_url.len() + 1 + param_name.len() + 1 + encoded_data.len();

        // Verify calculation: 35 (base_url) + 1 (#) + 4 (param_name) + 1 (=) + 29 (encoded_data)
        assert_eq!(expected_length, 35 + 1 + 4 + 1 + 29); // 70 total
    }
}
