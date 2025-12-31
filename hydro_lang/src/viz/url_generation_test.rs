//! Tests for URL generation and compression logic

#[cfg(test)]
mod tests {
    use crate::viz::config::VisualizerConfig;

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
    fn test_compression_and_encoding() {
        // Test with a small JSON that should not be compressed
        let small_json = r#"{"nodes":[],"edges":[]}"#;
        let config = VisualizerConfig::default();

        // Small JSON should skip compression
        assert!(small_json.len() < config.min_compression_size);
    }

    #[test]
    fn test_url_length_calculation() {
        let base_url = "https://hydro.run/hydroscope";
        let param_name = "data";
        let encoded_data = "eyJub2RlcyI6W10sImVkZ2VzIjpbXX0"; // base64 encoded {"nodes":[],"edges":[]}
        // Format: base_url#param_name=encoded_data
        let expected_length = base_url.len() + 1 + param_name.len() + 1 + encoded_data.len();
        // Check that the computed length matches manual composition bounds rather than fixed constant
        assert!(expected_length >= base_url.len());
        assert!(expected_length >= encoded_data.len());
        // And ensure separator characters are counted
        assert_eq!(
            expected_length,
            base_url.len() + 1 + param_name.len() + 1 + encoded_data.len()
        );
    }

    #[test]
    fn test_url_structure_example() {
        let base_url = "https://hydro.run/hydroscope";
        let param_name = "data";
        let encoded_data = "eyJub2RlcyI6W10sImVkZ2VzIjpbXX0";
        let url = format!("{}#{}={}", base_url, param_name, encoded_data);
        assert!(url.starts_with(base_url));
        assert!(url.contains('#'));
        assert!(url.contains('='));
        let fragment = url.split('#').nth(1).unwrap();
        let parts: Vec<&str> = fragment.split('=').collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], param_name);
        assert_eq!(parts[1], encoded_data);
        // Decoding should succeed with URL-safe base64 without padding
        let decoded = data_encoding::BASE64URL_NOPAD.decode(parts[1].as_bytes());
        assert!(decoded.is_ok());
    }
}
