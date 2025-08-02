/**
 * Unit Tests for JSONParser
 * 
 * Tests for parsing Hydro graph JSON data into VisualizationState
 */

import assert from 'assert';
import { 
  parseGraphJSON, 
  createGraphParser, 
  getAvailableGroupings,
  validateGraphJSON 
} from '../dist/core/JSONParser.js';
import { NODE_STYLES, EDGE_STYLES } from '../dist/shared/constants.js';

console.log('Running JSONParser tests...');

// Sample test data based on the chat.json structure
const sampleGraphData = {
  nodes: [
    {
      id: "0",
      data: {
        backtrace: [
          {
            fn_name: "hydro_lang::stream::Stream<T,L,B,O,R>::broadcast_bincode",
            filename: "/Users/test/stream.rs"
          }
        ]
      },
      position: { x: 100, y: 50 }
    },
    {
      id: "1", 
      data: {
        label: "Test Node 1"
      }
    },
    {
      id: "2",
      data: {
        name: "Custom Name Node"
      }
    }
  ],
  edges: [
    {
      id: "e0",
      source: "0",
      target: "1",
      animated: false,
      style: {
        stroke: "#008800",
        strokeWidth: 3
      }
    },
    {
      id: "e1", 
      source: "1",
      target: "2",
      animated: true,
      style: {
        strokeDasharray: "5,5"
      }
    }
  ],
  hierarchyChoices: [
    {
      id: "location",
      name: "Location",
      hierarchy: [
        {
          id: "loc_0",
          name: "Client",
          children: []
        },
        {
          id: "loc_1", 
          name: "Server",
          children: []
        }
      ]
    },
    {
      id: "backtrace",
      name: "Backtrace", 
      hierarchy: [
        {
          id: "bt_1",
          name: "examples/chat.rs",
          children: [
            {
              id: "bt_2",
              name: "main",
              children: []
            }
          ]
        }
      ]
    }
  ],
  nodeAssignments: {
    location: {
      "0": "loc_0",
      "1": "loc_0", 
      "2": "loc_1"
    },
    backtrace: {
      "0": "bt_2",
      "1": "bt_2",
      "2": "bt_2"
    }
  }
};

function testBasicJSONParsing() {
  console.log('Testing basic JSON parsing...');
  
  // Test parsing with object input
  const result1 = parseGraphJSON(sampleGraphData);
  assert(result1.state, 'Should return a state object');
  assert(result1.metadata, 'Should return metadata');
  
  // Test parsing with JSON string input
  const jsonString = JSON.stringify(sampleGraphData);
  const result2 = parseGraphJSON(jsonString);
  assert(result2.state, 'Should parse JSON string');
  
  // Verify basic structure
  assert.strictEqual(result1.state.visibleNodes.length, 3, 'Should have 3 nodes');
  assert.strictEqual(result1.state.visibleEdges.length, 2, 'Should have 2 edges');
  
  console.log('✓ Basic JSON parsing tests passed');
}

function testNodeParsing() {
  console.log('Testing node parsing...');
  
  const result = parseGraphJSON(sampleGraphData);
  const state = result.state;
  
  // Test node 0 (has backtrace data)
  const node0 = state.getGraphNode('0');
  assert(node0, 'Node 0 should exist');
  assert.strictEqual(node0.id, '0', 'Node should have correct ID');
  assert.strictEqual(node0.label, 'broadcast_bincode', 'Should extract label from backtrace');
  assert.strictEqual(node0.style, NODE_STYLES.DEFAULT, 'Should have default style');
  assert.strictEqual(node0.hidden, false, 'Should not be hidden initially');
  
  // Test node 1 (has explicit label)
  const node1 = state.getGraphNode('1');
  assert.strictEqual(node1.label, 'Test Node 1', 'Should use explicit label');
  
  // Test node 2 (has name)
  const node2 = state.getGraphNode('2');
  assert.strictEqual(node2.label, 'Custom Name Node', 'Should use name as label');
  
  console.log('✓ Node parsing tests passed');
}

function testEdgeParsing() {
  console.log('Testing edge parsing...');
  
  const result = parseGraphJSON(sampleGraphData);
  const state = result.state;
  
  // Test edge 0 (thick stroke)
  const edge0 = state.getGraphEdge('e0');
  assert(edge0, 'Edge e0 should exist');
  assert.strictEqual(edge0.source, '0', 'Should have correct source');
  assert.strictEqual(edge0.target, '1', 'Should have correct target');
  assert.strictEqual(edge0.style, EDGE_STYLES.THICK, 'Should detect thick style');
  assert.strictEqual(edge0.animated, false, 'Should preserve animated property');
  
  // Test edge 1 (animated and dashed)
  const edge1 = state.getGraphEdge('e1');
  assert.strictEqual(edge1.style, EDGE_STYLES.HIGHLIGHTED, 'Animated edges should be highlighted');
  assert.strictEqual(edge1.animated, true, 'Should preserve animated property');
  
  console.log('✓ Edge parsing tests passed');
}

function testHierarchyParsing() {
  console.log('Testing hierarchy parsing...');
  
  const result = parseGraphJSON(sampleGraphData, 'location');
  const state = result.state;
  
  // Check that containers were created
  const containers = state.visibleContainers;
  assert.strictEqual(containers.length, 2, 'Should have 2 containers');
  
  // Check container properties
  const loc0 = state.getContainer('loc_0');
  const loc1 = state.getContainer('loc_1');
  assert(loc0, 'Container loc_0 should exist');
  assert(loc1, 'Container loc_1 should exist');
  assert.strictEqual(loc0.label, 'Client', 'Should have correct label');
  assert.strictEqual(loc1.label, 'Server', 'Should have correct label');
  
  // Check node assignments
  assert.strictEqual(state.getNodeContainer('0'), 'loc_0', 'Node 0 should be in loc_0');
  assert.strictEqual(state.getNodeContainer('1'), 'loc_0', 'Node 1 should be in loc_0');
  assert.strictEqual(state.getNodeContainer('2'), 'loc_1', 'Node 2 should be in loc_1');
  
  // Check metadata
  assert.strictEqual(result.metadata.selectedGrouping, 'location', 'Should track selected grouping');
  assert.strictEqual(result.metadata.availableGroupings.length, 2, 'Should list available groupings');
  
  console.log('✓ Hierarchy parsing tests passed');
}

function testGroupingSelection() {
  console.log('Testing grouping selection...');
  
  // Test default grouping (first available)
  const result1 = parseGraphJSON(sampleGraphData);
  assert.strictEqual(result1.metadata.selectedGrouping, 'location', 'Should default to first grouping');
  
  // Test specific grouping selection
  const result2 = parseGraphJSON(sampleGraphData, 'backtrace');
  assert.strictEqual(result2.metadata.selectedGrouping, 'backtrace', 'Should use specified grouping');
  
  // Test invalid grouping (should fall back to first)
  const result3 = parseGraphJSON(sampleGraphData, 'nonexistent');
  assert.strictEqual(result3.metadata.selectedGrouping, 'location', 'Should fall back to first grouping');
  
  console.log('✓ Grouping selection tests passed');
}

function testNestedHierarchy() {
  console.log('Testing nested hierarchy...');
  
  const result = parseGraphJSON(sampleGraphData, 'backtrace');
  const state = result.state;
  
  // Check that nested containers were created
  const bt1 = state.getContainer('bt_1');
  const bt2 = state.getContainer('bt_2');
  assert(bt1, 'Parent container should exist');
  assert(bt2, 'Child container should exist');
  
  // Check hierarchy relationships
  const bt1Children = state.getContainerChildren('bt_1');
  assert(bt1Children.has('bt_2'), 'Parent should contain child container');
  
  // Check node assignments to leaf containers
  assert.strictEqual(state.getNodeContainer('0'), 'bt_2', 'Nodes should be in leaf container');
  
  console.log('✓ Nested hierarchy tests passed');
}

function testValidation() {
  console.log('Testing JSON validation...');
  
  // Test valid data
  const validation1 = validateGraphJSON(sampleGraphData);
  assert.strictEqual(validation1.isValid, true, 'Valid data should pass validation');
  assert.strictEqual(validation1.errors.length, 0, 'Should have no errors');
  assert.strictEqual(validation1.nodeCount, 3, 'Should count nodes correctly');
  assert.strictEqual(validation1.edgeCount, 2, 'Should count edges correctly');
  
  // Test invalid data
  const invalidData = { nodes: [] }; // Missing edges
  const validation2 = validateGraphJSON(invalidData);
  assert.strictEqual(validation2.isValid, false, 'Invalid data should fail validation');
  assert(validation2.errors.length > 0, 'Should have errors');
  
  // Test JSON parse error
  const validation3 = validateGraphJSON('invalid json');
  assert.strictEqual(validation3.isValid, false, 'Invalid JSON should fail validation');
  assert(validation3.errors.some(e => e.includes('JSON parsing error')), 'Should detect JSON error');
  
  console.log('✓ JSON validation tests passed');
}

function testUtilityFunctions() {
  console.log('Testing utility functions...');
  
  // Test getAvailableGroupings
  const groupings = getAvailableGroupings(sampleGraphData);
  assert.strictEqual(groupings.length, 2, 'Should return all groupings');
  assert.strictEqual(groupings[0].id, 'location', 'Should return correct grouping data');
  
  // Test with JSON string
  const groupings2 = getAvailableGroupings(JSON.stringify(sampleGraphData));
  assert.strictEqual(groupings2.length, 2, 'Should work with JSON string');
  
  // Test createGraphParser
  const parser = createGraphParser();
  assert(typeof parser.parse === 'function', 'Should return parser with parse method');
  
  const result = parser.parse(sampleGraphData);
  assert(result.state, 'Custom parser should work');
  
  console.log('✓ Utility function tests passed');
}

function testEdgeStyleDetection() {
  console.log('Testing edge style detection...');
  
  const testData = {
    nodes: [
      { id: "1" },
      { id: "2" }
    ],
    edges: [
      {
        id: "thick",
        source: "1", 
        target: "2",
        style: { strokeWidth: 5 }
      },
      {
        id: "dashed",
        source: "1",
        target: "2", 
        style: { strokeDasharray: "5,5" }
      },
      {
        id: "warning",
        source: "1",
        target: "2",
        style: { stroke: "red" }
      },
      {
        id: "animated",
        source: "1", 
        target: "2",
        animated: true
      }
    ]
  };
  
  const result = parseGraphJSON(testData);
  const state = result.state;
  
  assert.strictEqual(state.getGraphEdge('thick').style, EDGE_STYLES.THICK, 'Should detect thick edges');
  assert.strictEqual(state.getGraphEdge('dashed').style, EDGE_STYLES.DASHED, 'Should detect dashed edges');
  assert.strictEqual(state.getGraphEdge('warning').style, EDGE_STYLES.WARNING, 'Should detect warning edges');
  assert.strictEqual(state.getGraphEdge('animated').style, EDGE_STYLES.HIGHLIGHTED, 'Should detect animated edges');
  
  console.log('✓ Edge style detection tests passed');
}

function testErrorHandling() {
  console.log('Testing error handling...');
  
  // Test empty data
  try {
    parseGraphJSON({});
    assert.fail('Should throw error for empty data');
  } catch (error) {
    assert(error.message.includes('Invalid graph data'), 'Should throw descriptive error');
  }
  
  // Test missing nodes
  try {
    parseGraphJSON({ edges: [] });
    assert.fail('Should throw error for missing nodes');
  } catch (error) {
    assert(error.message.includes('Invalid graph data'), 'Should throw descriptive error');
  }
  
  // Test invalid JSON string
  try {
    parseGraphJSON('invalid json');
    assert.fail('Should throw error for invalid JSON');
  } catch (error) {
    assert(error instanceof SyntaxError, 'Should throw JSON syntax error');
  }
  
  console.log('✓ Error handling tests passed');
}

// ============ Run All Tests ============

function runAllTests() {
  try {
    testBasicJSONParsing();
    testNodeParsing();
    testEdgeParsing();
    testHierarchyParsing();
    testGroupingSelection();
    testNestedHierarchy();
    testValidation();
    testUtilityFunctions();
    testEdgeStyleDetection();
    testErrorHandling();
    
    console.log('\n🎉 All JSONParser tests passed! Parser is working correctly.');
  } catch (error) {
    console.error('\n❌ JSONParser test failed:', error.message);
    console.error(error.stack);
    process.exit(1);
  }
}

// Export for potential use in other test files
export {
  testBasicJSONParsing,
  testNodeParsing,
  testEdgeParsing,
  testHierarchyParsing,
  testGroupingSelection,
  testNestedHierarchy,
  testValidation,
  testUtilityFunctions,
  testEdgeStyleDetection,
  testErrorHandling,
  runAllTests
};

// Run tests if this file is executed directly
if (import.meta.url === `file://${process.argv[1]}`) {
  runAllTests();
}
