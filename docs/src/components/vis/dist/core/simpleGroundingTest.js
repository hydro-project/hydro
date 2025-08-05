/**
 * Simple Grounding Test (TypeScript Version)
 *
 * Tests the basic container collapse/expand functionality with a minimal scenario.
 * This serves as a simple smoke test for the core visualization state operations.
 */
import { createVisualizationState } from '../core/VisState.js';
import assert from 'assert';
/**
 * Test simple grounding with minimal container scenario
 */
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
    // Verify initial state using public API
    const internalNode = state.getGraphNode('internal');
    const externalNode = state.getGraphNode('external');
    const edge = state.getGraphEdge('edge1');
    assert.strictEqual(internalNode?.hidden, false, 'Internal node should initially be visible');
    assert.strictEqual(externalNode?.hidden, false, 'External node should initially be visible');
    assert.strictEqual(edge?.hidden, false, 'Edge should initially be visible');
    assert.strictEqual(state.allHyperEdges.length, 0, 'Should have no hyperEdges initially');
    console.log('  Initial state verified');
    // Collapse the container
    state.collapseContainer('container1');
    // Verify collapsed state using public API
    const internalNodeAfterCollapse = state.getGraphNode('internal');
    const externalNodeAfterCollapse = state.getGraphNode('external');
    const edgeAfterCollapse = state.getGraphEdge('edge1');
    assert.strictEqual(internalNodeAfterCollapse?.hidden, true, 'Internal node should be hidden after collapse');
    assert.strictEqual(externalNodeAfterCollapse?.hidden, false, 'External node should still be visible after collapse');
    assert.strictEqual(edgeAfterCollapse?.hidden, true, 'Edge should be hidden after collapse');
    const hyperEdges = state.allHyperEdges;
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
    const internalNodeAfterExpand = state.getGraphNode('internal');
    const externalNodeAfterExpand = state.getGraphNode('external');
    const edgeAfterExpand = state.getGraphEdge('edge1');
    assert.strictEqual(internalNodeAfterExpand?.hidden, false, 'Internal node should be visible after expand');
    assert.strictEqual(externalNodeAfterExpand?.hidden, false, 'External node should still be visible after expand');
    assert.strictEqual(edgeAfterExpand?.hidden, false, 'Edge should be visible after expand');
    assert.strictEqual(state.allHyperEdges.length, 0, 'Should have no hyperEdges after expand');
    console.log('  Expanded state verified');
    console.log('✓ Simple grounding test passed');
}
/**
 * Test multiple containers with interconnected nodes
 */
function testMultipleContainersGrounding() {
    console.log('Testing multiple containers grounding...');
    const state = createVisualizationState();
    // Create more complex scenario: two containers with internal connections
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
    // Create various edge types
    state.setGraphEdge('edge1-2', { source: 'node1', target: 'node2' }); // internal to A
    state.setGraphEdge('edge3-4', { source: 'node3', target: 'node4' }); // internal to B
    state.setGraphEdge('edge1-3', { source: 'node1', target: 'node3' }); // between containers
    state.setGraphEdge('edge2-ext', { source: 'node2', target: 'external' }); // A to external
    state.setGraphEdge('edge4-ext', { source: 'node4', target: 'external' }); // B to external
    // Verify initial state
    assert.strictEqual(state.visibleNodes.length, 5, 'Should have 5 visible nodes initially');
    assert.strictEqual(state.visibleEdges.length, 5, 'Should have 5 visible edges initially');
    assert.strictEqual(state.allHyperEdges.length, 0, 'Should have no hyperEdges initially');
    // Collapse containerA
    state.collapseContainer('containerA');
    // Verify partial collapse state
    assert.strictEqual(state.visibleNodes.length, 4, 'Should have 4 visible nodes after A collapse'); // containerA + node3 + node4 + external
    assert.strictEqual(state.visibleEdges.length, 1, 'Should have 1 visible edge after A collapse'); // edge3-4
    assert(state.allHyperEdges.length >= 2, 'Should have hyperEdges for A connections');
    // Collapse containerB as well
    state.collapseContainer('containerB');
    // Verify full collapse state
    assert.strictEqual(state.visibleNodes.length, 3, 'Should have 3 visible nodes after both collapsed'); // containerA + containerB + external
    assert.strictEqual(state.visibleEdges.length, 0, 'Should have no visible edges after both collapsed');
    assert(state.allHyperEdges.length >= 3, 'Should have hyperEdges for all connections');
    // Expand both containers
    state.expandContainer('containerA');
    state.expandContainer('containerB');
    // Verify full expansion
    assert.strictEqual(state.visibleNodes.length, 5, 'Should restore 5 visible nodes');
    assert.strictEqual(state.visibleEdges.length, 5, 'Should restore 5 visible edges');
    assert.strictEqual(state.allHyperEdges.length, 0, 'Should have no hyperEdges after full expansion');
    console.log('✓ Multiple containers grounding test passed');
}
/**
 * Test nested container grounding
 */
function testNestedContainerGrounding() {
    console.log('Testing nested container grounding...');
    const state = createVisualizationState();
    // Create nested structure
    state.setGraphNode('innerNode1', { label: 'Inner Node 1' });
    state.setGraphNode('innerNode2', { label: 'Inner Node 2' });
    state.setGraphNode('external', { label: 'External Node' });
    state.setContainer('innerContainer', {
        children: ['innerNode1', 'innerNode2'],
        label: 'Inner Container'
    });
    state.setContainer('outerContainer', {
        children: ['innerContainer'],
        label: 'Outer Container'
    });
    state.setGraphEdge('inner-edge', { source: 'innerNode1', target: 'innerNode2' });
    state.setGraphEdge('external-edge', { source: 'innerNode1', target: 'external' });
    // Test collapsing outer container
    state.collapseContainer('outerContainer');
    assert.strictEqual(state.visibleNodes.length, 2, 'Should have 2 visible nodes (outer container + external)');
    assert.strictEqual(state.visibleEdges.length, 0, 'Should have no visible edges');
    assert.strictEqual(state.allHyperEdges.length, 1, 'Should have 1 hyperEdge from outer to external');
    // Expand and verify restoration
    state.expandContainer('outerContainer');
    assert.strictEqual(state.visibleNodes.length, 3, 'Should restore nodes (inner container + 2 inner nodes + external)');
    assert.strictEqual(state.visibleEdges.length, 2, 'Should restore edges');
    assert.strictEqual(state.allHyperEdges.length, 0, 'Should have no hyperEdges');
    console.log('✓ Nested container grounding test passed');
}
/**
 * Run all simple grounding tests
 */
function runAllTests() {
    console.log('🧪 Running Simple Grounding Tests');
    console.log('=================================\n');
    return new Promise((resolve, reject) => {
        try {
            testSimpleGrounding();
            testMultipleContainersGrounding();
            testNestedContainerGrounding();
            console.log('\n🎉 All simple grounding tests passed!');
            console.log('✅ Basic container operations are working correctly!');
            resolve();
        }
        catch (error) {
            console.error('\n❌ Simple grounding test failed:', error instanceof Error ? error.message : String(error));
            if (error instanceof Error) {
                console.error(error.stack);
            }
            reject(error);
        }
    });
}
// Export for potential use in other test files
export { testSimpleGrounding, testMultipleContainersGrounding, testNestedContainerGrounding, runAllTests };
// Run tests if this file is executed directly
if (import.meta.url === `file://${process.argv[1]}`) {
    runAllTests().catch(() => process.exit(1));
}
//# sourceMappingURL=simpleGroundingTest.js.map