//! Tests for JSON graph generation with semantic tags

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    #[cfg(test)]
    use hydro_build_utils::insta;

    use crate::viz::json::HydroJson;
    use crate::viz::render::{
        HydroEdgeProp, HydroGraphWrite, HydroNodeType, HydroWriteConfig, NodeLabel,
    };

    #[test]
    fn test_json_structure_with_semantic_tags() {
        let mut output = String::new();
        let config = HydroWriteConfig::default();
        let mut writer = HydroJson::new(&mut output, &config);

        // Write a simple graph
        writer.write_prologue().unwrap();

        // Add a source node
        writer
            .write_node_definition(
                0,
                &NodeLabel::Static("source".to_string()),
                HydroNodeType::Source,
                Some(0),
                Some("Process"),
                None,
            )
            .unwrap();

        // Add a transform node
        writer
            .write_node_definition(
                1,
                &NodeLabel::Static("map".to_string()),
                HydroNodeType::Transform,
                Some(0),
                Some("Process"),
                None,
            )
            .unwrap();

        // Add an edge with semantic properties
        let mut edge_props = HashSet::new();
        edge_props.insert(HydroEdgeProp::Stream);
        edge_props.insert(HydroEdgeProp::Unbounded);
        edge_props.insert(HydroEdgeProp::TotalOrder);

        writer.write_edge(0, 1, &edge_props, None).unwrap();

        writer.write_epilogue().unwrap();

        // Snapshot test the complete JSON output
        insta::assert_snapshot!(output);
    }

    #[test]
    fn test_empty_semantic_tags() {
        let mut output = String::new();
        let config = HydroWriteConfig::default();
        let mut writer = HydroJson::new(&mut output, &config);

        writer.write_prologue().unwrap();

        writer
            .write_node_definition(
                0,
                &NodeLabel::Static("node".to_string()),
                HydroNodeType::Transform,
                None,
                None,
                None,
            )
            .unwrap();

        // Edge with no properties
        let edge_props = HashSet::new();
        writer.write_edge(0, 0, &edge_props, None).unwrap();

        writer.write_epilogue().unwrap();

        // Snapshot test the complete JSON output
        insta::assert_snapshot!(output);
    }

    #[test]
    fn test_deterministic_output_and_network_tagging() {
        use crate::viz::render::HydroWriteConfig;
        let mut output1 = String::new();
        let mut output2 = String::new();
        let config = HydroWriteConfig::default();
        let mut w1 = HydroJson::new(&mut output1, &config);
        let mut w2 = HydroJson::new(&mut output2, &config);

        // Build same small graph with two locations to force Network tag
        let mut edge_props = HashSet::new();
        edge_props.insert(HydroEdgeProp::Stream);

        // Graph 1
        w1.write_prologue().unwrap();
        w1.write_node_definition(
            0,
            &NodeLabel::Static("a".into()),
            HydroNodeType::Source,
            Some(0),
            Some("Process"),
            None,
        )
        .unwrap();
        w1.write_node_definition(
            1,
            &NodeLabel::Static("b".into()),
            HydroNodeType::Transform,
            Some(1),
            Some("Process"),
            None,
        )
        .unwrap();
        w1.write_edge(0, 1, &edge_props, None).unwrap();
        w1.write_epilogue().unwrap();

        // Graph 2 (same operations, different insertion order to test determinism)
        w2.write_prologue().unwrap();
        w2.write_node_definition(
            1,
            &NodeLabel::Static("b".into()),
            HydroNodeType::Transform,
            Some(1),
            Some("Process"),
            None,
        )
        .unwrap();
        w2.write_node_definition(
            0,
            &NodeLabel::Static("a".into()),
            HydroNodeType::Source,
            Some(0),
            Some("Process"),
            None,
        )
        .unwrap();
        w2.write_edge(0, 1, &edge_props, None).unwrap();
        w2.write_epilogue().unwrap();

        // Verify deterministic output
        assert_eq!(
            output1, output2,
            "JSON output should be deterministic regardless of insertion order"
        );

        // Snapshot test the complete JSON output
        insta::assert_snapshot!(output1);
    }
}
