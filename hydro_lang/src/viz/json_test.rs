//! Tests for JSON graph generation with semantic tags

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    #[cfg(test)]
    use hydro_build_utils::insta;

    use crate::location::dynamic::LocationId;
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

        let loc0 = LocationId::Process(0);

        // Register location
        writer.write_location_start(0, "Process").unwrap();

        // Add a source node
        writer
            .write_node_definition(
                0,
                &NodeLabel::Static("source".to_string()),
                HydroNodeType::Source,
                Some(0),
                Some(&loc0),
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
                Some(&loc0),
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

        let loc0 = LocationId::Process(0);
        let loc1 = LocationId::Process(1);

        // Graph 1
        w1.write_prologue().unwrap();
        w1.write_location_start(0, "Process").unwrap();
        w1.write_location_start(1, "Process").unwrap();
        w1.write_node_definition(
            0,
            &NodeLabel::Static("a".into()),
            HydroNodeType::Source,
            Some(0),
            Some(&loc0),
            None,
        )
        .unwrap();
        w1.write_node_definition(
            1,
            &NodeLabel::Static("b".into()),
            HydroNodeType::Transform,
            Some(1),
            Some(&loc1),
            None,
        )
        .unwrap();
        w1.write_edge(0, 1, &edge_props, None).unwrap();
        w1.write_epilogue().unwrap();

        // Graph 2 (same operations, different insertion order to test determinism)
        w2.write_prologue().unwrap();
        w2.write_location_start(0, "Process").unwrap();
        w2.write_location_start(1, "Process").unwrap();
        w2.write_node_definition(
            1,
            &NodeLabel::Static("b".into()),
            HydroNodeType::Transform,
            Some(1),
            Some(&loc1),
            None,
        )
        .unwrap();
        w2.write_node_definition(
            0,
            &NodeLabel::Static("a".into()),
            HydroNodeType::Source,
            Some(0),
            Some(&loc0),
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

    #[test]
    fn test_tick_hierarchy_simple() {
        let mut output = String::new();
        let config = HydroWriteConfig::default();
        let mut writer = HydroJson::new(&mut output, &config);

        writer.write_prologue().unwrap();

        let loc0 = LocationId::Process(0);
        let tick1_loc0 = LocationId::Tick(1, Box::new(loc0.clone()));

        // Register location
        writer.write_location_start(0, "Process").unwrap();

        // two nodes at tick=1 in process 0
        writer
            .write_node_definition(
                0,
                &NodeLabel::Static("a".into()),
                HydroNodeType::Source,
                Some(0),
                Some(&tick1_loc0),
                None,
            )
            .unwrap();
        writer
            .write_node_definition(
                1,
                &NodeLabel::Static("b".into()),
                HydroNodeType::Transform,
                Some(0),
                Some(&tick1_loc0),
                None,
            )
            .unwrap();

        let mut edge_props = HashSet::new();
        edge_props.insert(HydroEdgeProp::Stream);
        writer.write_edge(0, 1, &edge_props, None).unwrap();

        writer.write_epilogue().unwrap();

        // Parse and assert key structure
        let v: serde_json::Value = serde_json::from_str(&output).unwrap();
        // Ensure nodes carry tickId=1
        let nodes = v["nodes"].as_array().unwrap();
        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes[0]["data"]["tickId"], 1);
        assert_eq!(nodes[1]["data"]["tickId"], 1);

        // Check location hierarchy has a Tick(1) child container
        let choices = v["hierarchyChoices"].as_array().unwrap();
        let loc_choice = choices
            .iter()
            .find(|c| c["id"].as_str() == Some("location"))
            .unwrap();
        let children = loc_choice["children"].as_array().unwrap();
        // One location (Process 0)
        assert_eq!(children.len(), 1);
        let loc0_node = &children[0];
        assert_eq!(loc0_node["id"].as_str().unwrap(), "loc_0");
        let tick_children = loc0_node["children"].as_array().unwrap();
        assert_eq!(tick_children.len(), 1);
        assert_eq!(tick_children[0]["id"].as_str().unwrap(), "loc_0_tick_1");

        // And nodeAssignments map nodes to loc_0_tick_1
        let assignments = v["nodeAssignments"]["location"].as_object().unwrap();
        assert_eq!(assignments.get("0").unwrap(), "loc_0_tick_1");
        assert_eq!(assignments.get("1").unwrap(), "loc_0_tick_1");
    }

    #[test]
    fn test_mixed_tick_and_no_tick() {
        let mut output = String::new();
        let config = HydroWriteConfig::default();
        let mut writer = HydroJson::new(&mut output, &config);

        writer.write_prologue().unwrap();

        let loc0 = LocationId::Process(0);
        let tick2_loc0 = LocationId::Tick(2, Box::new(loc0.clone()));

        // Register location
        writer.write_location_start(0, "Process").unwrap();

        // node at tick=2
        writer
            .write_node_definition(
                0,
                &NodeLabel::Static("t2".into()),
                HydroNodeType::Transform,
                Some(0),
                Some(&tick2_loc0),
                None,
            )
            .unwrap();
        // node without tick (still in same location)
        writer
            .write_node_definition(
                1,
                &NodeLabel::Static("plain".into()),
                HydroNodeType::Transform,
                Some(0),
                Some(&loc0),
                None,
            )
            .unwrap();

        writer.write_epilogue().unwrap();

        let v: serde_json::Value = serde_json::from_str(&output).unwrap();
        // nodes: tickId present on node 0 only
        let nodes = v["nodes"].as_array().unwrap();
        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes[0]["data"]["tickId"], 2);
        assert!(nodes[1]["data"]["tickId"].is_null());

        let choices = v["hierarchyChoices"].as_array().unwrap();
        let loc_choice = choices
            .iter()
            .find(|c| c["id"].as_str() == Some("location"))
            .unwrap();
        let children = loc_choice["children"].as_array().unwrap();
        let loc0_node = &children[0];
        assert_eq!(loc0_node["id"], "loc_0");
        let tick_children = loc0_node["children"].as_array().unwrap();
        // Only one tick child present
        assert_eq!(tick_children.len(), 1);
        assert_eq!(tick_children[0]["id"], "loc_0_tick_2");

        // nodeAssignments: node 0 under tick container, node 1 directly under loc_0
        let assignments = v["nodeAssignments"]["location"].as_object().unwrap();
        assert_eq!(assignments.get("0").unwrap(), "loc_0_tick_2");
        assert_eq!(assignments.get("1").unwrap(), "loc_0");
    }

    #[test]
    fn test_disconnected_tick_components_split() {
        let mut output = String::new();
        let config = HydroWriteConfig::default();
        let mut writer = HydroJson::new(&mut output, &config);

        writer.write_prologue().unwrap();

        let loc0 = LocationId::Process(0);
        let tick1_loc0 = LocationId::Tick(1, Box::new(loc0.clone()));

        // Register location
        writer.write_location_start(0, "Process").unwrap();

        // Component 1: nodes 0-1 connected
        writer
            .write_node_definition(
                0,
                &NodeLabel::Static("a".into()),
                HydroNodeType::Transform,
                Some(0),
                Some(&tick1_loc0),
                None,
            )
            .unwrap();
        writer
            .write_node_definition(
                1,
                &NodeLabel::Static("b".into()),
                HydroNodeType::Transform,
                Some(0),
                Some(&tick1_loc0),
                None,
            )
            .unwrap();

        // Component 2: nodes 2-3 connected, but disconnected from 0-1
        writer
            .write_node_definition(
                2,
                &NodeLabel::Static("c".into()),
                HydroNodeType::Transform,
                Some(0),
                Some(&tick1_loc0),
                None,
            )
            .unwrap();
        writer
            .write_node_definition(
                3,
                &NodeLabel::Static("d".into()),
                HydroNodeType::Transform,
                Some(0),
                Some(&tick1_loc0),
                None,
            )
            .unwrap();

        let mut edge_props = HashSet::new();
        edge_props.insert(HydroEdgeProp::Stream);
        // edges within components only
        writer.write_edge(0, 1, &edge_props, None).unwrap();
        writer.write_edge(2, 3, &edge_props, None).unwrap();

        writer.write_epilogue().unwrap();

        let v: serde_json::Value = serde_json::from_str(&output).unwrap();
        let choices = v["hierarchyChoices"].as_array().unwrap();
        let loc_choice = choices
            .iter()
            .find(|c| c["id"].as_str() == Some("location"))
            .unwrap();
        let children = loc_choice["children"].as_array().unwrap();
        let loc0_node = &children[0];
        let tick_children = loc0_node["children"].as_array().unwrap();
        // Expect split into two tick containers c1 and c2
        assert_eq!(tick_children.len(), 2);
        assert_eq!(tick_children[0]["id"], "loc_0_tick_1_c1");
        assert_eq!(tick_children[1]["id"], "loc_0_tick_1_c2");

        // Node assignments reflect split
        let assignments = v["nodeAssignments"]["location"].as_object().unwrap();
        assert_eq!(assignments.get("0").unwrap(), "loc_0_tick_1_c1");
        assert_eq!(assignments.get("1").unwrap(), "loc_0_tick_1_c1");
        assert_eq!(assignments.get("2").unwrap(), "loc_0_tick_1_c2");
        assert_eq!(assignments.get("3").unwrap(), "loc_0_tick_1_c2");
    }

    #[test]
    fn test_nested_ticks_outermost_wins() {
        let mut output = String::new();
        let config = HydroWriteConfig::default();
        let mut writer = HydroJson::new(&mut output, &config);

        writer.write_prologue().unwrap();

        let loc0 = LocationId::Process(0);
        let tick1_loc0 = LocationId::Tick(1, Box::new(loc0.clone()));
        let tick2_over_tick1 = LocationId::Tick(2, Box::new(tick1_loc0.clone()));

        // Register location
        writer.write_location_start(0, "Process").unwrap();

        // Node under outer tick=2 (wrapped around inner tick=1)
        writer
            .write_node_definition(
                0,
                &NodeLabel::Static("outer2".into()),
                HydroNodeType::Transform,
                Some(0),
                Some(&tick2_over_tick1),
                None,
            )
            .unwrap();

        // Node under only inner tick=1
        writer
            .write_node_definition(
                1,
                &NodeLabel::Static("inner1".into()),
                HydroNodeType::Transform,
                Some(0),
                Some(&tick1_loc0),
                None,
            )
            .unwrap();

        writer.write_epilogue().unwrap();

        let v: serde_json::Value = serde_json::from_str(&output).unwrap();
        // tick ids
        let nodes = v["nodes"].as_array().unwrap();
        assert_eq!(nodes[0]["data"]["tickId"], 2);
        assert_eq!(nodes[1]["data"]["tickId"], 1);

        let choices = v["hierarchyChoices"].as_array().unwrap();
        let loc_choice = choices
            .iter()
            .find(|c| c["id"].as_str() == Some("location"))
            .unwrap();
        let children = loc_choice["children"].as_array().unwrap();
        let loc0_node = &children[0];
        let tick_children = loc0_node["children"].as_array().unwrap();

        // Expect two tick containers: tick 1 and tick 2
        assert_eq!(tick_children.len(), 2);
        assert_eq!(tick_children[0]["id"], "loc_0_tick_1");
        assert_eq!(tick_children[1]["id"], "loc_0_tick_2");

        // Node assignments accordingly
        let assignments = v["nodeAssignments"]["location"].as_object().unwrap();
        assert_eq!(assignments.get("0").unwrap(), "loc_0_tick_2");
        assert_eq!(assignments.get("1").unwrap(), "loc_0_tick_1");
    }
}
