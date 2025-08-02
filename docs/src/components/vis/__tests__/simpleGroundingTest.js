import { createVisualizationState } from '../dist/core/VisState.js';
import assert from 'assert';

function testSimpleGrounding() {
  console.log('Testing simple grounding...');
  
  const state = createVisualizationState();
  
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
  console.log('  HyperEdge details:', JSON.stringify(hyperEdge, null, 2));
  
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

testSimpleGrounding();
