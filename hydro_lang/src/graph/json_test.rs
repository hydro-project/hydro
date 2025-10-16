//! Tests for JSON graph generation with semantic tags

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use crate::graph::render::{HydroEdgeType, HydroNodeType, HydroGraphWrite, NodeLabel};
    use crate::graph::json::HydroJson;
    use crate::graph::render::HydroWriteConfig;

    #[test]
    fn test_json_structure_with_semantic_tags() {
        let mut output = String::new();
        let config = HydroWriteConfig::default();
        let mut writer = HydroJson::new(&mut output, &config);

        // Write a simple graph
        writer.write_prologue().unwrap();

        // Add a source node
        writer.write_node_definition(
            0,
            &NodeLabel::Static("source".to_string()),
            HydroNodeType::Source,
            Some(0),
            Some("Process"),
            None,
        ).unwrap();

        // Add a transform node
        writer.write_node_definition(
            1,
            &NodeLabel::Static("map".to_string()),
            HydroNodeType::Transform,
            Some(0),
            Some("Process"),
            None,
        ).unwrap();

        // Add an edge with semantic properties
        let mut edge_props = HashSet::new();
        edge_props.insert(HydroEdgeType::Stream);
        edge_props.insert(HydroEdgeType::Unbounded);
        edge_props.insert(HydroEdgeType::TotalOrder);

        writer.write_edge(0, 1, &edge_props, None).unwrap();

        writer.write_epilogue().unwrap();

        // Parse the JSON to validate structure
        let json: serde_json::Value = serde_json::from_str(&output)
            .expect("Generated JSON should be valid");

        // Validate top-level structure
        assert!(json.get("nodes").is_some(), "JSON should have 'nodes' field");
        assert!(json.get("edges").is_some(), "JSON should have 'edges' field");
        assert!(json.get("edgeStyleConfig").is_some(), "JSON should have 'edgeStyleConfig' field");
        assert!(json.get("nodeTypeConfig").is_some(), "JSON should have 'nodeTypeConfig' field");

        // Validate nodes have semantic tags
        let nodes = json["nodes"].as_array().expect("nodes should be an array");
        assert_eq!(nodes.len(), 2, "Should have 2 nodes");
        
        for node in nodes {
            assert!(node.get("semanticTags").is_some(), "Node should have semanticTags field");
            let tags = node["semanticTags"].as_array().expect("semanticTags should be an array");
            assert!(!tags.is_empty(), "Node should have at least one semantic tag");
        }

        // Validate edges have semantic tags
        let edges = json["edges"].as_array().expect("edges should be an array");
        assert_eq!(edges.len(), 1, "Should have 1 edge");
        
        let edge = &edges[0];
        assert!(edge.get("semanticTags").is_some(), "Edge should have semanticTags field");
        let tags = edge["semanticTags"].as_array().expect("semanticTags should be an array");
        assert_eq!(tags.len(), 3, "Edge should have 3 semantic tags");
        
        // Verify the semantic tags are strings
        let tag_strings: Vec<String> = tags.iter()
            .map(|t| t.as_str().unwrap().to_string())
            .collect();
        assert!(tag_strings.contains(&"Stream".to_string()));
        assert!(tag_strings.contains(&"Unbounded".to_string()));
        assert!(tag_strings.contains(&"TotalOrder".to_string()));

        // Validate edge style config has semantic mappings
        let edge_style_config = &json["edgeStyleConfig"];
        assert!(edge_style_config.get("semanticMappings").is_some(), 
                "edgeStyleConfig should have semanticMappings");
        
        let semantic_mappings = &edge_style_config["semanticMappings"];
        assert!(semantic_mappings.get("NetworkGroup").is_some());
        assert!(semantic_mappings.get("BoundednessGroup").is_some());
        assert!(semantic_mappings.get("CollectionGroup").is_some());
        assert!(semantic_mappings.get("FlowGroup").is_some());
        assert!(semantic_mappings.get("OrderingGroup").is_some());
    }

    #[test]
    fn test_empty_semantic_tags() {
        let mut output = String::new();
        let config = HydroWriteConfig::default();
        let mut writer = HydroJson::new(&mut output, &config);

        writer.write_prologue().unwrap();

        writer.write_node_definition(
            0,
            &NodeLabel::Static("node".to_string()),
            HydroNodeType::Transform,
            None,
            None,
            None,
        ).unwrap();

        // Edge with no properties
        let edge_props = HashSet::new();
        writer.write_edge(0, 0, &edge_props, None).unwrap();

        writer.write_epilogue().unwrap();

        let json: serde_json::Value = serde_json::from_str(&output)
            .expect("Generated JSON should be valid");

        // Validate edge has empty semantic tags array
        let edges = json["edges"].as_array().unwrap();
        let edge = &edges[0];
        let tags = edge["semanticTags"].as_array().unwrap();
        assert_eq!(tags.len(), 0, "Edge with no properties should have empty semanticTags array");
    }
}
