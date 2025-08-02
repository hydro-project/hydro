/**
 * Edge Index Encapsulation Tests (TypeScript Version)
 * 
 * Test to verify that edge index is automatically maintained through encapsulated APIs
 * This test ensures that container collapse/expand works correctly with automatic index maintenance
 */

import assert from 'assert';
import { createVisualizationState, VisualizationState } from '../core/VisState.js';

/**
 * Test to verify that edge index is automatically maintained through encapsulated APIs
 * This test ensures that container collapse/expand works correctly with automatic index maintenance
 */
function testEdgeIndexEncapsulation(): void {
  console.log('Testing automatic edge index maintenance through encapsulated APIs...');

  const state: VisualizationState = createVisualizationState();

  // Create the test graph exactly like in the test
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
  const containerChildren = state.getContainerChildren('container1');
  assert.strictEqual(containerChildren.size, 3, 'Container should have 3 children');
  assert.strictEqual(state.getContainer('container1')?.collapsed, false, 'Container should not be collapsed initially');

  // Verify all edges are visible initially
  assert.strictEqual(state.getGraphEdge('edge1-2')?.hidden, false, 'Edge1-2 should be visible initially');
  assert.strictEqual(state.getGraphEdge('edge2-3')?.hidden, false, 'Edge2-3 should be visible initially');
  assert.strictEqual(state.getGraphEdge('edge1-4')?.hidden, false, 'Edge1-4 should be visible initially');
  assert.strictEqual(state.getGraphEdge('edge4-3')?.hidden, false, 'Edge4-3 should be visible initially');

  // Test container collapse with automatic edge processing
  state.collapseContainer('container1');

  // Verify container is collapsed
  assert.strictEqual(state.getContainer('container1')?.collapsed, true, 'Container should be collapsed');

  // Verify all edges are hidden automatically due to proper edge index maintenance
  assert.strictEqual(state.getGraphEdge('edge1-2')?.hidden, true, 'Edge1-2 should be hidden after collapse');
  assert.strictEqual(state.getGraphEdge('edge2-3')?.hidden, true, 'Edge2-3 should be hidden after collapse');
  assert.strictEqual(state.getGraphEdge('edge1-4')?.hidden, true, 'Edge1-4 should be hidden after collapse');
  assert.strictEqual(state.getGraphEdge('edge4-3')?.hidden, true, 'Edge4-3 should be hidden after collapse');

  // Verify hyperEdges were created
  assert(state.allHyperEdges.length > 0, 'HyperEdges should be created for boundary connections');

  // Verify visibility state
  assert.strictEqual(state.visibleNodes.length, 1, 'Should have only container node visible');
  assert.strictEqual(state.visibleEdges.length, 0, 'Should have no visible edges');

  // Test expansion
  state.expandContainer('container1');

  // Verify container is expanded
  assert.strictEqual(state.getContainer('container1')?.collapsed, false, 'Container should be expanded');

  // Verify edges are visible again
  assert.strictEqual(state.getGraphEdge('edge1-2')?.hidden, false, 'Edge1-2 should be visible after expand');
  assert.strictEqual(state.getGraphEdge('edge2-3')?.hidden, false, 'Edge2-3 should be visible after expand');
  assert.strictEqual(state.getGraphEdge('edge1-4')?.hidden, false, 'Edge1-4 should be visible after expand');
  assert.strictEqual(state.getGraphEdge('edge4-3')?.hidden, false, 'Edge4-3 should be visible after expand');

  console.log('✅ Edge index encapsulation test passed');
}

/**
 * Test that multiple containers work correctly with edge index maintenance
 */
function testMultipleContainerEdgeIndex(): void {
  console.log('Testing edge index with multiple containers...');

  const state: VisualizationState = createVisualizationState();

  // Create multiple containers with interconnected nodes
  state.setGraphNode('node1', { label: 'Node 1' });
  state.setGraphNode('node2', { label: 'Node 2' });
  state.setGraphNode('node3', { label: 'Node 3' });
  state.setGraphNode('node4', { label: 'Node 4' });
  state.setGraphNode('external', { label: 'External Node' });

  state.setContainer('containerA', {
    children: ['node1', 'node2'],
    label: 'Container A'
  });

  state.setContainer('containerB', {
    children: ['node3', 'node4'],
    label: 'Container B'
  });

  // Create edges between containers and to external nodes
  state.setGraphEdge('edge1-2', { source: 'node1', target: 'node2' }); // internal to A
  state.setGraphEdge('edge3-4', { source: 'node3', target: 'node4' }); // internal to B
  state.setGraphEdge('edge1-3', { source: 'node1', target: 'node3' }); // between containers
  state.setGraphEdge('edge2-ext', { source: 'node2', target: 'external' }); // A to external
  state.setGraphEdge('edge4-ext', { source: 'node4', target: 'external' }); // B to external

  // Verify initial state
  assert.strictEqual(state.visibleNodes.length, 5, 'Should have 5 visible nodes initially');
  assert.strictEqual(state.visibleEdges.length, 5, 'Should have 5 visible edges initially');

  // Collapse containerA
  state.collapseContainer('containerA');

  // Verify edge states after containerA collapse
  assert.strictEqual(state.getGraphEdge('edge1-2')?.hidden, true, 'Internal edge in A should be hidden');
  assert.strictEqual(state.getGraphEdge('edge3-4')?.hidden, false, 'Internal edge in B should remain visible');
  assert.strictEqual(state.getGraphEdge('edge1-3')?.hidden, true, 'Cross-container edge should be hidden');
  assert.strictEqual(state.getGraphEdge('edge2-ext')?.hidden, true, 'A-to-external edge should be hidden');
  assert.strictEqual(state.getGraphEdge('edge4-ext')?.hidden, false, 'B-to-external edge should remain visible');

  // Should have hyperEdges for containerA connections
  const hyperEdges = state.allHyperEdges;
  assert(hyperEdges.length > 0, 'Should have hyperEdges for containerA');

  // Collapse containerB as well
  state.collapseContainer('containerB');

  // All original edges should be hidden
  assert.strictEqual(state.getGraphEdge('edge1-2')?.hidden, true, 'All edges should be hidden');
  assert.strictEqual(state.getGraphEdge('edge3-4')?.hidden, true, 'All edges should be hidden');
  assert.strictEqual(state.getGraphEdge('edge1-3')?.hidden, true, 'All edges should be hidden');
  assert.strictEqual(state.getGraphEdge('edge2-ext')?.hidden, true, 'All edges should be hidden');
  assert.strictEqual(state.getGraphEdge('edge4-ext')?.hidden, true, 'All edges should be hidden');

  // Expand both containers
  state.expandContainer('containerA');
  state.expandContainer('containerB');

  // All edges should be visible again
  assert.strictEqual(state.getGraphEdge('edge1-2')?.hidden, false, 'All edges should be visible again');
  assert.strictEqual(state.getGraphEdge('edge3-4')?.hidden, false, 'All edges should be visible again');
  assert.strictEqual(state.getGraphEdge('edge1-3')?.hidden, false, 'All edges should be visible again');
  assert.strictEqual(state.getGraphEdge('edge2-ext')?.hidden, false, 'All edges should be visible again');
  assert.strictEqual(state.getGraphEdge('edge4-ext')?.hidden, false, 'All edges should be visible again');

  // HyperEdges should be cleaned up
  assert.strictEqual(state.allHyperEdges.length, 0, 'HyperEdges should be cleaned up');

  console.log('✅ Multiple container edge index test passed');
}

/**
 * Test edge index maintenance with nested containers
 */
function testNestedContainerEdgeIndex(): void {
  console.log('Testing edge index with nested containers...');

  const state: VisualizationState = createVisualizationState();

  // Create nested container structure
  state.setGraphNode('node1', { label: 'Node 1' });
  state.setGraphNode('node2', { label: 'Node 2' });
  state.setGraphNode('external', { label: 'External Node' });

  state.setContainer('innerContainer', {
    children: ['node1', 'node2'],
    label: 'Inner Container'
  });

  state.setContainer('outerContainer', {
    children: ['innerContainer'],
    label: 'Outer Container'
  });

  state.setGraphEdge('edge1-2', { source: 'node1', target: 'node2' }); // internal
  state.setGraphEdge('edge1-ext', { source: 'node1', target: 'external' }); // to external

  // Test collapsing outer container (should also collapse inner)
  state.collapseContainer('outerContainer');

  assert.strictEqual(state.getGraphEdge('edge1-2')?.hidden, true, 'Internal edge should be hidden');
  assert.strictEqual(state.getGraphEdge('edge1-ext')?.hidden, true, 'External edge should be hidden');

  // Should have hyperEdge from outer container to external
  const hyperEdges = state.allHyperEdges;
  assert(hyperEdges.length > 0, 'Should have hyperEdge for outer container');

  // Expand outer container
  state.expandContainer('outerContainer');

  assert.strictEqual(state.getGraphEdge('edge1-2')?.hidden, false, 'Edges should be visible again');
  assert.strictEqual(state.getGraphEdge('edge1-ext')?.hidden, false, 'Edges should be visible again');
  assert.strictEqual(state.allHyperEdges.length, 0, 'HyperEdges should be cleaned up');

  console.log('✅ Nested container edge index test passed');
}

/**
 * Run all tests
 */
function runAllTests(): Promise<void> {
  console.log('🧪 Running Edge Index Encapsulation Tests');
  console.log('=====================================\n');
  
  return new Promise((resolve, reject) => {
    try {
      testEdgeIndexEncapsulation();
      testMultipleContainerEdgeIndex();
      testNestedContainerEdgeIndex();
      
      console.log('\n🎉 All edge index encapsulation tests passed!');
      console.log('✅ Edge index maintenance is working correctly through encapsulated APIs!');
      resolve();
    } catch (error: unknown) {
      console.error('\n❌ Edge index encapsulation test failed:', error instanceof Error ? error.message : String(error));
      if (error instanceof Error) {
        console.error(error.stack);
      }
      reject(error);
    }
  });
}

// Export for potential use in other test files
export {
  testEdgeIndexEncapsulation,
  testMultipleContainerEdgeIndex,
  testNestedContainerEdgeIndex,
  runAllTests
};

// Run tests if this file is executed directly
if (import.meta.url === `file://${process.argv[1]}`) {
  runAllTests().catch(() => process.exit(1));
}
