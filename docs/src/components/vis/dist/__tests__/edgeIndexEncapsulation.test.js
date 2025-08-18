import assert from 'assert';
import { createVisualizationState } from '../dist/core/VisState.js';
/**
 * Test to verify that edge index is automatically maintained through encapsulated APIs
 * This test ensures that container collapse/expand works correctly with automatic index maintenance
 */
function testEdgeIndexEncapsulation() {
    console.log('Testing automatic edge index maintenance through encapsulated APIs...');
    const state = createVisualizationState();
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
    assert.strictEqual(state.getContainer('container1').collapsed, false, 'Container should not be collapsed initially');
    // Verify all edges are visible initially
    assert.strictEqual(state.getGraphEdge('edge1-2').hidden, false, 'Edge1-2 should be visible initially');
    assert.strictEqual(state.getGraphEdge('edge2-3').hidden, false, 'Edge2-3 should be visible initially');
    assert.strictEqual(state.getGraphEdge('edge1-4').hidden, false, 'Edge1-4 should be visible initially');
    assert.strictEqual(state.getGraphEdge('edge4-3').hidden, false, 'Edge4-3 should be visible initially');
    // Test container collapse with automatic edge processing
    state.collapseContainer('container1');
    // Verify container is collapsed
    assert.strictEqual(state.getContainer('container1').collapsed, true, 'Container should be collapsed');
    // Verify all edges are hidden automatically due to proper edge index maintenance
    assert.strictEqual(state.getGraphEdge('edge1-2').hidden, true, 'Edge1-2 should be hidden after collapse');
    assert.strictEqual(state.getGraphEdge('edge2-3').hidden, true, 'Edge2-3 should be hidden after collapse');
    assert.strictEqual(state.getGraphEdge('edge1-4').hidden, true, 'Edge1-4 should be hidden after collapse');
    assert.strictEqual(state.getGraphEdge('edge4-3').hidden, true, 'Edge4-3 should be hidden after collapse');
    // Verify hyperEdges were created
    assert(state.allHyperEdges.length > 0, 'HyperEdges should be created for boundary connections');
    // Verify visibility state
    assert.strictEqual(state.visibleNodes.length, 1, 'Should have only container node visible');
    assert.strictEqual(state.visibleEdges.length, 0, 'Should have no visible edges');
    // Test expansion
    state.expandContainer('container1');
    // Verify container is expanded
    assert.strictEqual(state.getContainer('container1').collapsed, false, 'Container should be expanded');
    // Verify edges are visible again
    assert.strictEqual(state.getGraphEdge('edge1-2').hidden, false, 'Edge1-2 should be visible after expand');
    assert.strictEqual(state.getGraphEdge('edge2-3').hidden, false, 'Edge2-3 should be visible after expand');
    assert.strictEqual(state.getGraphEdge('edge1-4').hidden, false, 'Edge1-4 should be visible after expand');
    assert.strictEqual(state.getGraphEdge('edge4-3').hidden, false, 'Edge4-3 should be visible after expand');
    console.log('✅ Edge index encapsulation test passed');
}
/**
 * Run all tests
 */
function runAllTests() {
    console.log('🧪 Running Edge Index Encapsulation Tests');
    console.log('=====================================\n');
    try {
        testEdgeIndexEncapsulation();
        console.log('\n🎉 All edge index encapsulation tests passed!');
        console.log('✅ Automatic edge index maintenance is working correctly');
    }
    catch (error) {
        console.error('❌ Test failed:', error.message);
        process.exit(1);
    }
}
// Run tests if this file is executed directly
if (import.meta.url === `file://${process.argv[1]}`) {
    runAllTests();
}
export { testEdgeIndexEncapsulation, runAllTests };
//# sourceMappingURL=edgeIndexEncapsulation.test.js.map