/**
 * Unit Tests for VisualizationState
 * 
 * Comprehensive tests covering all functionality of the visualization state management
 */

import assert from 'assert';
import { VisualizationState, createVisualizationState } from '../dist/VisState.js';
import { NODE_STYLES, EDGE_STYLES } from '../dist/constants.js';

// Test utilities
function createTestState() {
  return createVisualizationState();
}

function assertSetsEqual(actual, expected, message) {
  const actualArray = Array.from(actual).sort();
  const expectedArray = Array.from(expected).sort();
  assert.deepStrictEqual(actualArray, expectedArray, message);
}

// ============ Basic State Creation Tests ============

console.log('Running VisualizationState tests...');

function testStateCreation() {
  console.log('Testing state creation...');
  
  const state1 = new VisualizationState();
  const state2 = createVisualizationState();
  
  assert(state1 instanceof VisualizationState, 'Direct constructor should work');
  assert(state2 instanceof VisualizationState, 'Factory function should work');
  
  // Check initial state
  assert.strictEqual(state1.graphNodes.size, 0, 'Should start with no nodes');
  assert.strictEqual(state1.graphEdges.size, 0, 'Should start with no edges');
  assert.strictEqual(state1.containers.size, 0, 'Should start with no containers');
  assert.strictEqual(state1.hyperEdges.size, 0, 'Should start with no hyperEdges');
  
  console.log('✓ State creation tests passed');
}

// ============ Node Management Tests ============

function testNodeManagement() {
  console.log('Testing node management...');
  
  const state = createTestState();
  
  // Test node creation
  const node1 = state.setGraphNode('node1', { 
    label: 'Test Node 1',
    style: NODE_STYLES.DEFAULT 
  });
  
  assert.strictEqual(node1.id, 'node1', 'Node should have correct id');
  assert.strictEqual(node1.label, 'Test Node 1', 'Node should have correct label');
  assert.strictEqual(node1.style, NODE_STYLES.DEFAULT, 'Node should have correct style');
  assert.strictEqual(node1.hidden, false, 'Node should not be hidden by default');
  
  // Test node retrieval
  const retrieved = state.getGraphNode('node1');
  assert.deepStrictEqual(retrieved, node1, 'Retrieved node should match created node');
  
  // Test visible nodes collection
  const visibleNodes = state.getVisibleNodes();
  assert.strictEqual(visibleNodes.length, 1, 'Should have one visible node');
  assert.strictEqual(visibleNodes[0].id, 'node1', 'Visible node should be node1');
  
  // Test hiding nodes
  state.setNodeHidden('node1', true);
  assert.strictEqual(state.getNodeHidden('node1'), true, 'Node should be hidden');
  assert.strictEqual(state.getVisibleNodes().length, 0, 'Should have no visible nodes when hidden');
  
  // Test showing nodes
  state.setNodeHidden('node1', false);
  assert.strictEqual(state.getNodeHidden('node1'), false, 'Node should not be hidden');
  assert.strictEqual(state.getVisibleNodes().length, 1, 'Should have one visible node when shown');
  
  // Test node removal
  state.removeGraphNode('node1');
  assert.strictEqual(state.getGraphNode('node1'), undefined, 'Removed node should not exist');
  assert.strictEqual(state.getVisibleNodes().length, 0, 'Should have no visible nodes after removal');
  
  console.log('✓ Node management tests passed');
}

// ============ Edge Management Tests ============

function testEdgeManagement() {
  console.log('Testing edge management...');
  
  const state = createTestState();
  
  // Create some nodes first
  state.setGraphNode('node1', { label: 'Node 1' });
  state.setGraphNode('node2', { label: 'Node 2' });
  
  // Test edge creation
  const edge1 = state.setGraphEdge('edge1', {
    source: 'node1',
    target: 'node2',
    style: EDGE_STYLES.DEFAULT
  });
  
  assert.strictEqual(edge1.id, 'edge1', 'Edge should have correct id');
  assert.strictEqual(edge1.source, 'node1', 'Edge should have correct source');
  assert.strictEqual(edge1.target, 'node2', 'Edge should have correct target');
  assert.strictEqual(edge1.hidden, false, 'Edge should not be hidden by default');
  
  // Test nodeToEdges mapping
  const node1Edges = state.nodeToEdges.get('node1');
  const node2Edges = state.nodeToEdges.get('node2');
  assert(node1Edges && node1Edges.has('edge1'), 'Node1 should be connected to edge1');
  assert(node2Edges && node2Edges.has('edge1'), 'Node2 should be connected to edge1');
  
  // Test edge visibility
  const visibleEdges = state.getVisibleEdges();
  assert.strictEqual(visibleEdges.length, 1, 'Should have one visible edge');
  
  // Test edge hiding
  state.setEdgeHidden('edge1', true);
  assert.strictEqual(state.getEdgeHidden('edge1'), true, 'Edge should be hidden');
  assert.strictEqual(state.getVisibleEdges().length, 0, 'Should have no visible edges when hidden');
  
  // Test edge removal
  state.removeGraphEdge('edge1');
  assert.strictEqual(state.getGraphEdge('edge1'), undefined, 'Removed edge should not exist');
  const node1EdgesAfter = state.nodeToEdges.get('node1');
  const node2EdgesAfter = state.nodeToEdges.get('node2');
  assert(!node1EdgesAfter || !node1EdgesAfter.has('edge1'), 'Node1 should not be connected to removed edge');
  assert(!node2EdgesAfter || !node2EdgesAfter.has('edge1'), 'Node2 should not be connected to removed edge');
  
  console.log('✓ Edge management tests passed');
}

// ============ Container Management Tests ============

function testContainerManagement() {
  console.log('Testing container management...');
  
  const state = createTestState();
  
  // Create some nodes
  state.setGraphNode('node1', { label: 'Node 1' });
  state.setGraphNode('node2', { label: 'Node 2' });
  
  // Test container creation
  const container1 = state.setContainer('container1', {
    expandedDimensions: { width: 200, height: 150 },
    children: ['node1', 'node2']
  });
  
  assert.strictEqual(container1.id, 'container1', 'Container should have correct id');
  assert.strictEqual(container1.collapsed, false, 'Container should not be collapsed by default');
  assert.strictEqual(container1.hidden, false, 'Container should not be hidden by default');
  assertSetsEqual(container1.children, new Set(['node1', 'node2']), 'Container should have correct children');
  
  // Test container hierarchy tracking
  assert.strictEqual(state.getNodeContainer('node1'), 'container1', 'Node1 should be in container1');
  assert.strictEqual(state.getNodeContainer('node2'), 'container1', 'Node2 should be in container1');
  assertSetsEqual(state.getContainerChildren('container1'), new Set(['node1', 'node2']), 'Container should have correct children');
  
  // Test container visibility management
  const visibleContainers = state.getVisibleContainers();
  const expandedContainers = state.getExpandedContainers();
  assert.strictEqual(visibleContainers.length, 1, 'Should have one visible container');
  assert.strictEqual(expandedContainers.length, 1, 'Should have one expanded container');
  
  // Test container collapse
  state.setContainerCollapsed('container1', true);
  assert.strictEqual(state.getContainerCollapsed('container1'), true, 'Container should be collapsed');
  assert.strictEqual(state.getExpandedContainers().length, 0, 'Should have no expanded containers when collapsed');
  
  // Test container hiding
  state.setContainerHidden('container1', true);
  assert.strictEqual(state.getContainerHidden('container1'), true, 'Container should be hidden');
  assert.strictEqual(state.getVisibleContainers().length, 0, 'Should have no visible containers when hidden');
  
  console.log('✓ Container management tests passed');
}

// ============ HyperEdge Management Tests ============

function testHyperEdgeManagement() {
  console.log('Testing hyperEdge management...');
  
  const state = createTestState();
  
  // Test hyperEdge creation
  const hyperEdge1 = state.setHyperEdge('hyper1', {
    source: 'node1',
    target: 'container1',
    style: EDGE_STYLES.THICK,
    originalEdges: []
  });
  
  assert.strictEqual(hyperEdge1.id, 'hyper1', 'HyperEdge should have correct id');
  assert.strictEqual(hyperEdge1.source, 'node1', 'HyperEdge should have correct source');
  assert.strictEqual(hyperEdge1.target, 'container1', 'HyperEdge should have correct target');
  assert.strictEqual(hyperEdge1.style, EDGE_STYLES.THICK, 'HyperEdge should have correct style');
  
  // Test hyperEdge retrieval
  const retrieved = state.getHyperEdge('hyper1');
  assert.deepStrictEqual(retrieved, hyperEdge1, 'Retrieved hyperEdge should match created hyperEdge');
  
  // Test hyperEdge collection
  const hyperEdges = state.getHyperEdges();
  assert.strictEqual(hyperEdges.length, 1, 'Should have one hyperEdge');
  assert.strictEqual(hyperEdges[0].id, 'hyper1', 'HyperEdge should be hyper1');
  
  // Test hyperEdge removal
  state.removeHyperEdge('hyper1');
  assert.strictEqual(state.getHyperEdge('hyper1'), undefined, 'Removed hyperEdge should not exist');
  assert.strictEqual(state.getHyperEdges().length, 0, 'Should have no hyperEdges after removal');
  
  console.log('✓ HyperEdge management tests passed');
}

// ============ Container Collapse/Expand Tests ============

function testContainerCollapseExpand() {
  console.log('Testing container collapse/expand...');
  
  const state = createTestState();
  
  // Create a test graph: container1 with nodes 1,2,3 and edges between them and to external node4
  state.setGraphNode('node1', { label: 'Node 1' });
  state.setGraphNode('node2', { label: 'Node 2' });
  state.setGraphNode('node3', { label: 'Node 3' });
  state.setGraphNode('node4', { label: 'External Node' });
  
  state.setGraphEdge('edge1-2', { source: 'node1', target: 'node2' }); // internal edge
  state.setGraphEdge('edge2-3', { source: 'node2', target: 'node3' }); // internal edge
  state.setGraphEdge('edge1-4', { source: 'node1', target: 'node4' }); // boundary edge
  state.setGraphEdge('edge4-3', { source: 'node4', target: 'node3' }); // boundary edge
  
  state.setContainer('container1', {
    children: ['node1', 'node2', 'node3']
  });
  
  // Verify initial state
  assert.strictEqual(state.getVisibleNodes().length, 4, 'Should have 4 visible nodes initially');
  assert.strictEqual(state.getVisibleEdges().length, 4, 'Should have 4 visible edges initially');
  assert.strictEqual(state.getHyperEdges().length, 0, 'Should have no hyperEdges initially');
  
  // Test collapse
  state.collapseContainer('container1');
  
  // Check collapsed state
  assert.strictEqual(state.getContainerCollapsed('container1'), true, 'Container should be collapsed');
  assert(state.collapsedContainers.has('container1'), 'Should have collapsed container representation');
  
  // Check node visibility (nodes 1,2,3 should be hidden, node4 should be visible)
  assert.strictEqual(state.getNodeHidden('node1'), true, 'Node1 should be hidden');
  assert.strictEqual(state.getNodeHidden('node2'), true, 'Node2 should be hidden');
  assert.strictEqual(state.getNodeHidden('node3'), true, 'Node3 should be hidden');
  assert.strictEqual(state.getNodeHidden('node4'), false, 'Node4 should be visible');
  assert.strictEqual(state.getVisibleNodes().length, 1, 'Should have 1 visible node after collapse');
  
  // Check edge visibility (all original edges should be hidden)
  assert.strictEqual(state.getEdgeHidden('edge1-2'), true, 'Internal edge should be hidden');
  assert.strictEqual(state.getEdgeHidden('edge2-3'), true, 'Internal edge should be hidden');
  assert.strictEqual(state.getEdgeHidden('edge1-4'), true, 'Boundary edge should be hidden');
  assert.strictEqual(state.getEdgeHidden('edge4-3'), true, 'Boundary edge should be hidden');
  assert.strictEqual(state.getVisibleEdges().length, 0, 'Should have no visible edges after collapse');
  
  // Check hyperEdges (should have hyperEdges connecting node4 to container1)
  const hyperEdges = state.getHyperEdges();
  assert.strictEqual(hyperEdges.length, 2, 'Should have 2 hyperEdges after collapse');
  
  const hyperEdgeIds = hyperEdges.map(he => he.id);
  assert(hyperEdgeIds.includes('hyper_node4_to_container1'), 'Should have incoming hyperEdge');
  assert(hyperEdgeIds.includes('hyper_container1_to_node4'), 'Should have outgoing hyperEdge');
  
  // Test expand
  state.expandContainer('container1');
  
  // Check expanded state
  assert.strictEqual(state.getContainerCollapsed('container1'), false, 'Container should be expanded');
  assert(!state.collapsedContainers.has('container1'), 'Should not have collapsed container representation');
  
  // Check node visibility (all nodes should be visible again)
  assert.strictEqual(state.getNodeHidden('node1'), false, 'Node1 should be visible');
  assert.strictEqual(state.getNodeHidden('node2'), false, 'Node2 should be visible');
  assert.strictEqual(state.getNodeHidden('node3'), false, 'Node3 should be visible');
  assert.strictEqual(state.getNodeHidden('node4'), false, 'Node4 should be visible');
  assert.strictEqual(state.getVisibleNodes().length, 4, 'Should have 4 visible nodes after expand');
  
  // Check edge visibility (all original edges should be visible again)
  assert.strictEqual(state.getEdgeHidden('edge1-2'), false, 'Internal edge should be visible');
  assert.strictEqual(state.getEdgeHidden('edge2-3'), false, 'Internal edge should be visible');
  assert.strictEqual(state.getEdgeHidden('edge1-4'), false, 'Boundary edge should be visible');
  assert.strictEqual(state.getEdgeHidden('edge4-3'), false, 'Boundary edge should be visible');
  assert.strictEqual(state.getVisibleEdges().length, 4, 'Should have 4 visible edges after expand');
  
  // Check hyperEdges (should be removed)
  assert.strictEqual(state.getHyperEdges().length, 0, 'Should have no hyperEdges after expand');
  
  console.log('✓ Container collapse/expand tests passed');
}

// ============ Nested Container Tests ============

function testNestedContainers() {
  console.log('Testing nested containers...');
  
  const state = createTestState();
  
  // Create nested structure: outer container contains inner container which contains nodes
  state.setGraphNode('node1', { label: 'Node 1' });
  state.setGraphNode('node2', { label: 'Node 2' });
  state.setGraphNode('node3', { label: 'External Node' });
  
  state.setContainer('innerContainer', {
    children: ['node1', 'node2']
  });
  
  state.setContainer('outerContainer', {
    children: ['innerContainer']
  });
  
  state.setGraphEdge('edge1-3', { source: 'node1', target: 'node3' });
  
  // Test collapsing outer container (should collapse inner container first)
  state.collapseContainer('outerContainer');
  
  // Both containers should be collapsed
  assert.strictEqual(state.getContainerCollapsed('innerContainer'), true, 'Inner container should be collapsed');
  assert.strictEqual(state.getContainerCollapsed('outerContainer'), true, 'Outer container should be collapsed');
  
  // Nodes should be hidden
  assert.strictEqual(state.getNodeHidden('node1'), true, 'Node1 should be hidden');
  assert.strictEqual(state.getNodeHidden('node2'), true, 'Node2 should be hidden');
  assert.strictEqual(state.getNodeHidden('node3'), false, 'External node should be visible');
  
  // Should have hyperEdge from outerContainer to node3
  const hyperEdges = state.getHyperEdges();
  assert.strictEqual(hyperEdges.length, 1, 'Should have 1 hyperEdge');
  assert.strictEqual(hyperEdges[0].source, 'outerContainer', 'HyperEdge should originate from outer container');
  assert.strictEqual(hyperEdges[0].target, 'node3', 'HyperEdge should connect to external node');
  
  console.log('✓ Nested container tests passed');
}

// ============ Edge Style Aggregation Tests ============

function testEdgeStyleAggregation() {
  console.log('Testing edge style aggregation...');
  
  const state = createTestState();
  
  // Test style aggregation priority
  const defaultEdge = { style: EDGE_STYLES.DEFAULT };
  const highlightedEdge = { style: EDGE_STYLES.HIGHLIGHTED };
  const warningEdge = { style: EDGE_STYLES.WARNING };
  const thickEdge = { style: EDGE_STYLES.THICK };
  
  // Test with single edge
  assert.strictEqual(
    state._aggregateEdgeStyles([defaultEdge]),
    EDGE_STYLES.DEFAULT,
    'Single default edge should remain default'
  );
  
  // Test priority: warning > highlighted > default
  assert.strictEqual(
    state._aggregateEdgeStyles([defaultEdge, highlightedEdge, warningEdge]),
    EDGE_STYLES.WARNING,
    'Warning should have highest priority'
  );
  
  // Test priority: thick > highlighted > default
  assert.strictEqual(
    state._aggregateEdgeStyles([defaultEdge, highlightedEdge, thickEdge]),
    EDGE_STYLES.THICK,
    'Thick should have higher priority than highlighted'
  );
  
  console.log('✓ Edge style aggregation tests passed');
}

// ============ Clear State Tests ============

function testClearState() {
  console.log('Testing state clearing...');
  
  const state = createTestState();
  
  // Add various elements
  state.setGraphNode('node1', { label: 'Node 1' });
  state.setGraphEdge('edge1', { source: 'node1', target: 'node1' });
  state.setContainer('container1', { children: ['node1'] });
  state.setHyperEdge('hyper1', { source: 'node1', target: 'container1' });
  
  // Verify elements exist
  assert.strictEqual(state.graphNodes.size, 1, 'Should have nodes before clear');
  assert.strictEqual(state.graphEdges.size, 1, 'Should have edges before clear');
  assert.strictEqual(state.containers.size, 1, 'Should have containers before clear');
  assert.strictEqual(state.hyperEdges.size, 1, 'Should have hyperEdges before clear');
  
  // Clear state
  state.clear();
  
  // Verify everything is cleared
  assert.strictEqual(state.graphNodes.size, 0, 'Should have no nodes after clear');
  assert.strictEqual(state.graphEdges.size, 0, 'Should have no edges after clear');
  assert.strictEqual(state.containers.size, 0, 'Should have no containers after clear');
  assert.strictEqual(state.hyperEdges.size, 0, 'Should have no hyperEdges after clear');
  assert.strictEqual(state.visibleNodes.size, 0, 'Should have no visible nodes after clear');
  assert.strictEqual(state.visibleEdges.size, 0, 'Should have no visible edges after clear');
  assert.strictEqual(state.nodeToEdges.size, 0, 'Should have no edge mappings after clear');
  
  console.log('✓ State clearing tests passed');
}

// ============ Simple Grounding Test ============

function testSimpleGrounding() {
  console.log('Testing simple grounding...');
  
  const state = createTestState();
  
  // Create a very simple scenario: one container with one node, connected to one external node
  state.setGraphNode('internal', { label: 'Internal Node' });
  state.setGraphNode('external', { label: 'External Node' });
  
  state.setContainer('container1', {
    children: ['internal']
  });
  
  state.setGraphEdge('edge1', { source: 'internal', target: 'external' });
  
  // Verify initial state
  assert.strictEqual(state.getNodeHidden('internal'), false, 'Internal node should initially be visible');
  assert.strictEqual(state.getNodeHidden('external'), false, 'External node should initially be visible');
  assert.strictEqual(state.getEdgeHidden('edge1'), false, 'Edge should initially be visible');
  assert.strictEqual(state.getHyperEdges().length, 0, 'Should have no hyperEdges initially');
  
  console.log('  Initial state verified');
  
  // Collapse the container
  state.collapseContainer('container1');
  
  // Verify collapsed state
  assert.strictEqual(state.getNodeHidden('internal'), true, 'Internal node should be hidden after collapse');
  assert.strictEqual(state.getNodeHidden('external'), false, 'External node should still be visible after collapse');
  assert.strictEqual(state.getEdgeHidden('edge1'), true, 'Edge should be hidden after collapse');
  
  const hyperEdges = state.getHyperEdges();
  assert.strictEqual(hyperEdges.length, 1, 'Should have exactly 1 hyperEdge after collapse');
  
  const hyperEdge = hyperEdges[0];
  assert.strictEqual(hyperEdge.source, 'container1', 'HyperEdge should originate from container');
  assert.strictEqual(hyperEdge.target, 'external', 'HyperEdge should connect to external node');
  assert(hyperEdge.originalEdges && hyperEdge.originalEdges.length === 1, 'HyperEdge should have 1 original edge');
  assert.strictEqual(hyperEdge.originalInternalEndpoint, 'internal', 'HyperEdge should cache the internal endpoint');
  
  console.log('  Collapsed state verified');
  
  // Expand the container
  state.expandContainer('container1');
  
  // Verify expanded state (should be exactly like initial state)
  assert.strictEqual(state.getNodeHidden('internal'), false, 'Internal node should be visible after expand');
  assert.strictEqual(state.getNodeHidden('external'), false, 'External node should still be visible after expand');
  assert.strictEqual(state.getEdgeHidden('edge1'), false, 'Edge should be visible after expand');
  assert.strictEqual(state.getHyperEdges().length, 0, 'Should have no hyperEdges after expand');
  
  console.log('  Expanded state verified');
  
  console.log('✓ Simple grounding test passed');
}

// ============ Inverse Operation Tests ============

function testInverseOperations() {
  console.log('Testing inverse operations...');
  
  testSimpleCollapseExpandInverse();
  testNestedCollapseExpandInverse();
  testMultiLevelInverse();
  testPartialCollapseInverse();
  
  console.log('✓ All inverse operation tests passed');
}

/**
 * Test that collapse followed by expand returns to original state
 */
function testSimpleCollapseExpandInverse() {
  console.log('  Testing simple collapse/expand inverse...');
  
  const state = createTestState();
  
  // Create a simple hierarchy
  state.setGraphNode('node1', { label: 'Node 1' });
  state.setGraphNode('node2', { label: 'Node 2' });
  state.setGraphNode('node3', { label: 'External Node' });
  
  state.setContainer('container1', {
    children: ['node1', 'node2']
  });
  
  state.setGraphEdge('edge1-2', { source: 'node1', target: 'node2' }); // Internal
  state.setGraphEdge('edge1-3', { source: 'node1', target: 'node3' }); // External
  state.setGraphEdge('edge2-3', { source: 'node2', target: 'node3' }); // External
  
  // Capture initial state
  const initialState = captureStateSnapshot(state);
  
  // Collapse and expand
  state.collapseContainer('container1');
  state.expandContainer('container1');
  
  // Check that we're back to the initial state
  const finalState = captureStateSnapshot(state);
  assertStateEqual(initialState, finalState, 'Simple collapse/expand should be inverse');
  
  console.log('    ✓ Simple collapse/expand inverse test passed');
}

/**
 * Test nested container collapse/expand inverse
 */
function testNestedCollapseExpandInverse() {
  console.log('  Testing nested collapse/expand inverse...');
  
  const state = createTestState();
  
  // Create nested hierarchy: outer -> inner -> nodes
  state.setGraphNode('node1', { label: 'Node 1' });
  state.setGraphNode('node2', { label: 'Node 2' });
  state.setGraphNode('node3', { label: 'External Node' });
  
  state.setContainer('innerContainer', {
    children: ['node1', 'node2']
  });
  
  state.setContainer('outerContainer', {
    children: ['innerContainer']
  });
  
  state.setGraphEdge('edge1-2', { source: 'node1', target: 'node2' }); // Internal to inner
  state.setGraphEdge('edge1-3', { source: 'node1', target: 'node3' }); // External
  
  // Capture initial state
  const initialState = captureStateSnapshot(state);
  
  // Test outer container collapse/expand
  state.collapseContainer('outerContainer');
  state.expandContainer('outerContainer');
  
  const afterOuterState = captureStateSnapshot(state);
  assertStateEqual(initialState, afterOuterState, 'Outer collapse/expand should be inverse');
  
  // Test inner container collapse/expand
  state.collapseContainer('innerContainer');
  state.expandContainer('innerContainer');
  
  const afterInnerState = captureStateSnapshot(state);
  assertStateEqual(initialState, afterInnerState, 'Inner collapse/expand should be inverse');
  
  console.log('    ✓ Nested collapse/expand inverse test passed');
}

/**
 * Test multi-level collapse and expand operations
 */
function testMultiLevelInverse() {
  console.log('  Testing multi-level inverse operations...');
  
  const state = createTestState();
  
  // Create a 3-level hierarchy
  state.setGraphNode('node1', { label: 'Node 1' });
  state.setGraphNode('node2', { label: 'Node 2' });
  state.setGraphNode('node3', { label: 'Node 3' });
  state.setGraphNode('external', { label: 'External Node' });
  
  state.setContainer('level1', { children: ['node1'] });
  state.setContainer('level2', { children: ['level1', 'node2'] });
  state.setContainer('level3', { children: ['level2', 'node3'] });
  
  state.setGraphEdge('edge1-2', { source: 'node1', target: 'node2' });
  state.setGraphEdge('edge2-3', { source: 'node2', target: 'node3' });
  state.setGraphEdge('edge1-ext', { source: 'node1', target: 'external' });
  
  const initialState = captureStateSnapshot(state);
  
  // Test different collapse/expand sequences
  
  // Sequence 1: Bottom-up collapse, top-down expand
  state.collapseContainer('level1');
  state.collapseContainer('level2');
  state.collapseContainer('level3');
  
  state.expandContainer('level3');
  state.expandContainer('level2');
  state.expandContainer('level1');
  
  const afterSequence1 = captureStateSnapshot(state);
  assertStateEqual(initialState, afterSequence1, 'Bottom-up collapse + top-down expand should be inverse');
  
  // Sequence 2: Top-down collapse, bottom-up expand
  state.collapseContainer('level3');
  state.expandContainer('level3');
  
  const afterSequence2 = captureStateSnapshot(state);
  assertStateEqual(initialState, afterSequence2, 'Top-down collapse + expand should be inverse');
  
  // Sequence 3: Mixed order
  state.collapseContainer('level2');
  state.collapseContainer('level1');
  state.expandContainer('level2');
  state.expandContainer('level1');
  
  const afterSequence3 = captureStateSnapshot(state);
  assertStateEqual(initialState, afterSequence3, 'Mixed order collapse/expand should be inverse');
  
  console.log('    ✓ Multi-level inverse test passed');
}

/**
 * Test partial collapse scenarios where only some containers are collapsed
 */
function testPartialCollapseInverse() {
  console.log('  Testing partial collapse inverse operations...');
  
  const state = createTestState();
  
  // Create multiple independent containers
  state.setGraphNode('node1', { label: 'Node 1' });
  state.setGraphNode('node2', { label: 'Node 2' });
  state.setGraphNode('node3', { label: 'Node 3' });
  state.setGraphNode('node4', { label: 'Node 4' });
  state.setGraphNode('external', { label: 'External Node' });
  
  state.setContainer('containerA', { children: ['node1', 'node2'] });
  state.setContainer('containerB', { children: ['node3', 'node4'] });
  
  state.setGraphEdge('edgeA1-A2', { source: 'node1', target: 'node2' });
  state.setGraphEdge('edgeB3-B4', { source: 'node3', target: 'node4' });
  state.setGraphEdge('edgeA-B', { source: 'node1', target: 'node3' });
  state.setGraphEdge('edgeA-ext', { source: 'node2', target: 'external' });
  state.setGraphEdge('edgeB-ext', { source: 'node4', target: 'external' });
  
  const initialState = captureStateSnapshot(state);
  
  // Test partial collapse: only containerA
  state.collapseContainer('containerA');
  const afterCollapseA = captureStateSnapshot(state);
  
  state.expandContainer('containerA');
  const afterExpandA = captureStateSnapshot(state);
  assertStateEqual(initialState, afterExpandA, 'Partial collapse A should be inverse');
  
  // Test partial collapse: only containerB
  state.collapseContainer('containerB');
  const afterCollapseB = captureStateSnapshot(state);
  
  state.expandContainer('containerB');
  const afterExpandB = captureStateSnapshot(state);
  assertStateEqual(initialState, afterExpandB, 'Partial collapse B should be inverse');
  
  // Test both collapsed
  state.collapseContainer('containerA');
  state.collapseContainer('containerB');
  const afterBothCollapsed = captureStateSnapshot(state);
  
  state.expandContainer('containerA');
  state.expandContainer('containerB');
  const afterBothExpanded = captureStateSnapshot(state);
  assertStateEqual(initialState, afterBothExpanded, 'Both collapsed should be inverse');
  
  // Test different expand order
  state.collapseContainer('containerA');
  state.collapseContainer('containerB');
  
  state.expandContainer('containerB');
  state.expandContainer('containerA');
  const afterDifferentOrder = captureStateSnapshot(state);
  assertStateEqual(initialState, afterDifferentOrder, 'Different expand order should be inverse');
  
  console.log('    ✓ Partial collapse inverse test passed');
}

/**
 * Capture a complete snapshot of the state for comparison
 */
function captureStateSnapshot(state) {
  return {
    // Node states
    nodes: Array.from(state.graphNodes.entries()).map(([id, node]) => ({
      id,
      hidden: node.hidden,
      label: node.label
    })).sort((a, b) => a.id.localeCompare(b.id)),
    
    // Edge states
    edges: Array.from(state.graphEdges.entries()).map(([id, edge]) => ({
      id,
      source: edge.source,
      target: edge.target,
      hidden: edge.hidden
    })).sort((a, b) => a.id.localeCompare(b.id)),
    
    // Container states
    containers: Array.from(state.containers.entries()).map(([id, container]) => ({
      id,
      collapsed: container.collapsed,
      hidden: container.hidden,
      children: Array.from(container.children).sort()
    })).sort((a, b) => a.id.localeCompare(b.id)),
    
    // HyperEdges
    hyperEdges: Array.from(state.hyperEdges.entries()).map(([id, hyperEdge]) => ({
      id,
      source: hyperEdge.source,
      target: hyperEdge.target,
      originalEdgeCount: hyperEdge.originalEdges?.length || 0
    })).sort((a, b) => a.id.localeCompare(b.id)),
    
    // Derived collections
    visibleNodeCount: state.getVisibleNodes().length,
    visibleEdgeCount: state.getVisibleEdges().length,
    visibleContainerCount: state.getVisibleContainers().length,
    expandedContainerCount: state.getExpandedContainers().length,
    hyperEdgeCount: state.getHyperEdges().length
  };
}

/**
 * Assert that two state snapshots are equal
 */
function assertStateEqual(expected, actual, message) {
  // Compare nodes
  assert.deepStrictEqual(actual.nodes, expected.nodes, `${message}: Node states should match`);
  
  // Compare edges
  assert.deepStrictEqual(actual.edges, expected.edges, `${message}: Edge states should match`);
  
  // Compare containers
  assert.deepStrictEqual(actual.containers, expected.containers, `${message}: Container states should match`);
  
  // Compare hyperEdges
  assert.deepStrictEqual(actual.hyperEdges, expected.hyperEdges, `${message}: HyperEdge states should match`);
  
  // Compare derived counts
  assert.strictEqual(actual.visibleNodeCount, expected.visibleNodeCount, `${message}: Visible node count should match`);
  assert.strictEqual(actual.visibleEdgeCount, expected.visibleEdgeCount, `${message}: Visible edge count should match`);
  assert.strictEqual(actual.visibleContainerCount, expected.visibleContainerCount, `${message}: Visible container count should match`);
  assert.strictEqual(actual.expandedContainerCount, expected.expandedContainerCount, `${message}: Expanded container count should match`);
  assert.strictEqual(actual.hyperEdgeCount, expected.hyperEdgeCount, `${message}: HyperEdge count should match`);
}

// ============ Run All Tests ============

function runAllTests() {
  try {
    testStateCreation();
    testNodeManagement();
    testEdgeManagement();
    testContainerManagement();
    testHyperEdgeManagement();
    testContainerCollapseExpand();
    testNestedContainers();
    testEdgeStyleAggregation();
    testClearState();
    testSimpleGrounding();
    testInverseOperations();
    
    console.log('\n🎉 All tests passed! VisualizationState is working correctly.');
  } catch (error) {
    console.error('\n❌ Test failed:', error.message);
    console.error(error.stack);
    process.exit(1);
  }
}

// Export for potential use in other test files
export {
  testStateCreation,
  testNodeManagement,
  testEdgeManagement,
  testContainerManagement,
  testHyperEdgeManagement,
  testContainerCollapseExpand,
  testNestedContainers,
  testEdgeStyleAggregation,
  testClearState,
  testSimpleGrounding,
  testInverseOperations,
  runAllTests
};

// Run tests if this file is executed directly
if (import.meta.url === `file://${process.argv[1]}`) {
  runAllTests();
}
