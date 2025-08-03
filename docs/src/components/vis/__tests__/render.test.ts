/**
 * Unit Tests for Render Directory Components
 * 
 * Comprehensive test coverage for ReactFlow rendering components
 */

import * as assert from 'assert';
import { VisualizationState, createVisualizationState } from '../core/VisState.js';
import { ReactFlowConverter } from '../render/ReactFlowConverter.js';
import { applyNodeStyling } from '../render/nodeStyler.js';
import { createNodeEventHandlers, createEdgeEventHandlers } from '../render/eventHandlers.js';
import { DEFAULT_RENDER_CONFIG } from '../render/config.js';
import { validateELKResult, validateReactFlowResult } from '../render/validation.js';
import { DEFAULT_LAYOUT_CONFIG } from '../layout/index.js';
import { NODE_STYLES, EDGE_STYLES } from '../shared/constants.js';

// Test utilities
function createTestState(): VisualizationState {
  return createVisualizationState();
}

function createMockLayoutResult() {
  return {
    nodes: [
      {
        id: 'node1',
        label: 'Test Node 1',
        style: 'default',
        x: 100,
        y: 100,
        width: 150,
        height: 50,
        hidden: false
      },
      {
        id: 'node2', 
        label: 'Test Node 2',
        style: 'default',
        x: 300,
        y: 200,
        width: 150,
        height: 50,
        hidden: false
      }
    ],
    edges: [
      {
        id: 'edge1',
        source: 'node1',
        target: 'node2',
        style: 'default'
      }
    ],
    containers: [
      {
        id: 'container1',
        collapsed: false,
        children: new Set(['node1']),
        x: 50,
        y: 50,
        width: 200,
        height: 150
      }
    ],
    hyperEdges: []
  };
}

// ============ ReactFlowConverter Tests ============

console.log('Running Render Component tests...');

function testReactFlowConverter(): void {
  console.log('Testing ReactFlowConverter...');
  
  const layoutResult = createMockLayoutResult();
  const reactFlowData = ReactFlowConverter.convert(layoutResult);
  
  // Test basic conversion
  assert(reactFlowData, 'Converter should return data');
  assert(Array.isArray(reactFlowData.nodes), 'Should have nodes array');
  assert(Array.isArray(reactFlowData.edges), 'Should have edges array');
  
  // Test node conversion
  assert.strictEqual(reactFlowData.nodes.length, 3, 'Should convert containers and nodes (1 container + 2 nodes)');
  
  const standardNode = reactFlowData.nodes.find(n => n.id === 'node1');
  assert(standardNode, 'Should find converted standard node');
  assert.strictEqual(standardNode.type, 'standard', 'Standard node should have correct type');
  assert.strictEqual(standardNode.position.x, 100, 'Node should have correct x position');
  assert.strictEqual(standardNode.position.y, 100, 'Node should have correct y position');
  assert(standardNode.data.label, 'Node should have label in data');
  
  const containerNode = reactFlowData.nodes.find(n => n.id === 'container1');
  assert(containerNode, 'Should find converted container node');
  assert.strictEqual(containerNode.type, 'container', 'Container should have correct type');
  assert.strictEqual(containerNode.position.x, 50, 'Container should have correct x position');
  assert.strictEqual(containerNode.style?.width, 200, 'Container should have correct width');
  assert.strictEqual(containerNode.style?.height, 150, 'Container should have correct height');
  
  // Test edge conversion
  assert.strictEqual(reactFlowData.edges.length, 1, 'Should convert all edges');
  const edge = reactFlowData.edges[0];
  assert.strictEqual(edge.source, 'node1', 'Edge should have correct source');
  assert.strictEqual(edge.target, 'node2', 'Edge should have correct target');
  assert.strictEqual(edge.type, 'standard', 'Edge should have standard type');
  
  console.log('✓ ReactFlowConverter tests passed');
}

// ============ Node Styling Tests ============

function testNodeStyling(): void {
  console.log('Testing node styling...');
  
  const mockNodes = [
    {
      id: 'node1',
      type: 'standard',
      position: { x: 0, y: 0 },
      data: { 
        label: 'Source Node',
        nodeType: 'Source'
      }
    },
    {
      id: 'node2',
      type: 'standard', 
      position: { x: 0, y: 0 },
      data: {
        label: 'Transform Node',
        nodeType: 'Transform'
      }
    },
    {
      id: 'container1',
      type: 'container',
      position: { x: 0, y: 0 },
      data: { collapsed: false }
    }
  ];
  
  const styledNodes = applyNodeStyling(mockNodes, 'Set2');
  
  assert.strictEqual(styledNodes.length, 3, 'Should return same number of nodes');
  
  // Check that container nodes are not styled (skipped)
  const containerNode = styledNodes.find(n => n.id === 'container1');
  assert(containerNode, 'Should find container node');
  assert(!containerNode.style?.backgroundColor, 'Container node should not have background color styling');
  
  // Check that standard nodes get styling
  const sourceNode = styledNodes.find(n => n.id === 'node1');
  assert(sourceNode, 'Should find source node');
  assert(sourceNode.style?.backgroundColor, 'Source node should have background color');
  
  const transformNode = styledNodes.find(n => n.id === 'node2');
  assert(transformNode, 'Should find transform node');
  assert(transformNode.style?.backgroundColor, 'Transform node should have background color');
  
  // Test that different node types get different colors
  assert.notStrictEqual(
    sourceNode.style?.backgroundColor,
    transformNode.style?.backgroundColor,
    'Different node types should have different colors'
  );
  
  console.log('✓ Node styling tests passed');
}

// ============ Event Handler Tests ============

function testEventHandlers(): void {
  console.log('Testing event handlers...');
  
  let nodeClickCalled = false;
  let edgeClickCalled = false;
  let nodeDoubleClickCalled = false;
  let contextMenuCalled = false;
  
  // Test node event handlers
  const nodeData = {
    onNodeClick: (id: string) => { nodeClickCalled = true; },
    onNodeDoubleClick: (id: string) => { nodeDoubleClickCalled = true; },
    onNodeContextMenu: (id: string, event: any) => { contextMenuCalled = true; }
  };
  
  const nodeHandlers = createNodeEventHandlers('test-node', nodeData);
  
  // Create mock events
  const mockEvent = {
    stopPropagation: () => {},
    preventDefault: () => {}
  };
  
  nodeHandlers.handleClick(mockEvent as any);
  assert(nodeClickCalled, 'Node click handler should be called');
  
  nodeHandlers.handleDoubleClick(mockEvent as any);
  assert(nodeDoubleClickCalled, 'Node double click handler should be called');
  
  nodeHandlers.handleContextMenu(mockEvent as any);
  assert(contextMenuCalled, 'Node context menu handler should be called');
  
  // Test edge event handlers
  const edgeData = {
    onEdgeClick: (id: string) => { edgeClickCalled = true; }
  };
  
  const edgeHandlers = createEdgeEventHandlers('test-edge', edgeData);
  edgeHandlers.handleClick(mockEvent as any);
  assert(edgeClickCalled, 'Edge click handler should be called');
  
  console.log('✓ Event handler tests passed');
}

// ============ Render Configuration Tests ============

function testRenderConfiguration(): void {
  console.log('Testing render configuration...');
  
  // Test default config
  assert(typeof DEFAULT_RENDER_CONFIG === 'object', 'Default config should be an object');
  assert(typeof DEFAULT_RENDER_CONFIG.enableMiniMap === 'boolean', 'enableMiniMap should be boolean');
  assert(typeof DEFAULT_RENDER_CONFIG.enableControls === 'boolean', 'enableControls should be boolean');
  assert(typeof DEFAULT_RENDER_CONFIG.fitView === 'boolean', 'fitView should be boolean');
  assert(typeof DEFAULT_RENDER_CONFIG.nodesDraggable === 'boolean', 'nodesDraggable should be boolean');
  assert(typeof DEFAULT_RENDER_CONFIG.snapToGrid === 'boolean', 'snapToGrid should be boolean');
  assert(typeof DEFAULT_RENDER_CONFIG.gridSize === 'number', 'gridSize should be number');
  
  // Test that defaults provide sensible values
  assert.strictEqual(DEFAULT_RENDER_CONFIG.enableMiniMap, true, 'MiniMap should be enabled by default');
  assert.strictEqual(DEFAULT_RENDER_CONFIG.enableControls, true, 'Controls should be enabled by default');
  assert.strictEqual(DEFAULT_RENDER_CONFIG.fitView, true, 'fitView should be enabled by default');
  assert(DEFAULT_RENDER_CONFIG.gridSize > 0, 'Grid size should be positive');
  
  console.log('✓ Render configuration tests passed');
}

// ============ Validation Tests ============

function testValidation(): void {
  console.log('Testing validation utilities...');
  
  // Test ELK result validation
  const validLayoutResult = createMockLayoutResult();
  const validReport = validateELKResult(validLayoutResult);
  assert(validReport.isValid, 'Valid layout result should pass validation');
  assert.strictEqual(validReport.errors.length, 0, 'Valid layout should have no errors');
  
  // Test invalid layout result
  const invalidLayoutResult = null;
  const invalidReport = validateELKResult(invalidLayoutResult);
  assert(!invalidReport.isValid, 'Invalid layout result should fail validation');
  assert(invalidReport.errors.length > 0, 'Invalid layout should have errors');
  
  // Test layout result missing required fields
  const incompleteLayoutResult = { nodes: null, edges: [], containers: [] };
  const incompleteReport = validateELKResult(incompleteLayoutResult);
  assert(!incompleteReport.isValid, 'Incomplete layout result should fail validation');
  
  console.log('✓ Validation tests passed');
}

// ============ Layout Integration Tests ============

function testLayoutIntegration(): void {
  console.log('Testing layout integration...');
  
  // Test that layout config integrates with rendering
  assert(typeof DEFAULT_LAYOUT_CONFIG === 'object', 'Layout config should exist');
  
  // Test VisualizationState integration with layout
  const state = createTestState();
  
  // Add some test data
  state.setGraphNode('node1', { label: 'Test Node', style: NODE_STYLES.DEFAULT });
  state.setGraphEdge('edge1', { source: 'node1', target: 'node2', style: EDGE_STYLES.DEFAULT });
  
  assert.strictEqual(state.visibleNodes.length, 1, 'State should track visible nodes');
  assert.strictEqual(state.visibleEdges.length, 1, 'State should track visible edges');
  
  // Test that state provides data in format suitable for layout engine
  const nodes = state.visibleNodes;
  const edges = state.visibleEdges;
  
  assert(Array.isArray(nodes), 'Visible nodes should be array');
  assert(Array.isArray(edges), 'Visible edges should be array');
  
  if (nodes.length > 0) {
    const node = nodes[0];
    assert(node.id, 'Node should have id');
    assert(node.label, 'Node should have label');
    assert(node.style, 'Node should have style');
  }
  
  if (edges.length > 0) {
    const edge = edges[0];
    assert(edge.id, 'Edge should have id');
    assert(edge.source, 'Edge should have source');
    assert(edge.target, 'Edge should have target');
  }
  
  console.log('✓ Layout integration tests passed');
}

// ============ GraphFlow Integration Tests ============

function testGraphFlowIntegration(): void {
  console.log('Testing GraphFlow integration with VisualizationState...');
  
  const state = createTestState();
  
  // Add test data to state
  state.setGraphNode('node1', { 
    label: 'Source Node',
    style: NODE_STYLES.DEFAULT 
  });
  state.setGraphNode('node2', { 
    label: 'Sink Node',
    style: NODE_STYLES.DEFAULT 
  });
  state.setGraphEdge('edge1', { 
    source: 'node1',
    target: 'node2',
    style: EDGE_STYLES.DEFAULT 
  });
  
  // Test that state provides data in correct format for GraphFlow
  const visibleNodes = state.visibleNodes;
  const visibleEdges = state.visibleEdges;
  const visibleContainers = state.visibleContainers;
  const hyperEdges = state.allHyperEdges;
  
  assert(Array.isArray(visibleNodes), 'Should provide nodes array');
  assert(Array.isArray(visibleEdges), 'Should provide edges array');
  assert(Array.isArray(visibleContainers), 'Should provide containers array');
  assert(Array.isArray(hyperEdges), 'Should provide hyperEdges array');
  
  // Test that data has required structure for layout engine
  assert.strictEqual(visibleNodes.length, 2, 'Should have correct number of nodes');
  assert.strictEqual(visibleEdges.length, 1, 'Should have correct number of edges');
  
  // Verify node structure
  const node = visibleNodes[0];
  assert(typeof node.id === 'string', 'Node should have string id');
  assert(typeof node.label === 'string', 'Node should have string label');
  assert(typeof node.style === 'string', 'Node should have string style');
  
  // Verify edge structure
  const edge = visibleEdges[0];
  assert(typeof edge.id === 'string', 'Edge should have string id');
  assert(typeof edge.source === 'string', 'Edge should have string source');
  assert(typeof edge.target === 'string', 'Edge should have string target');
  assert(typeof edge.style === 'string', 'Edge should have string style');
  
  console.log('✓ GraphFlow integration tests passed');
}

// ============ Run All Tests ============

export async function runAllTests(): Promise<void> {
  console.log('🎨 Running Render Component Tests\n');
  console.log('======================================\n');
  
  try {
    testReactFlowConverter();
    testNodeStyling();
    testEventHandlers();
    testRenderConfiguration();
    testValidation();
    testLayoutIntegration();
    testGraphFlowIntegration();
    
    console.log('\n======================================');
    console.log('🎉 All render component tests passed!');
    console.log('✅ ReactFlow components working correctly');
    console.log('✅ State integration verified');
    console.log('✅ Layout engine compatibility confirmed');
    
  } catch (error: unknown) {
    console.error('\n======================================');
    console.error('❌ Render component tests failed');
    console.error('Error:', error instanceof Error ? error.message : String(error));
    throw error;
  }
}

// Run tests if this file is executed directly  
if (typeof require !== 'undefined' && require.main === module) {
  runAllTests().catch(() => process.exit(1));
}