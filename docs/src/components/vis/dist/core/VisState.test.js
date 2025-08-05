/**
 * Unit Tests for VisualizationState (TypeScript Version)
 *
 * Simplified tests focusing on public API only
 */
import assert from 'assert';
import { VisualizationState, createVisualizationState } from '../core/VisState.js';
import { NODE_STYLES, EDGE_STYLES } from '../shared/constants.js';
// Test utilities
function createTestState() {
    return createVisualizationState();
}
// ============ Basic State Creation Tests ============
console.log('Running VisualizationState tests...');
function testStateCreation() {
    console.log('Testing state creation...');
    const state1 = new VisualizationState();
    const state2 = createVisualizationState();
    assert(state1 instanceof VisualizationState, 'Direct constructor should work');
    assert(state2 instanceof VisualizationState, 'Factory function should work');
    // Check initial state using public API
    assert.strictEqual(state1.visibleNodes.length, 0, 'Should start with no visible nodes');
    assert.strictEqual(state1.visibleEdges.length, 0, 'Should start with no visible edges');
    assert.strictEqual(state1.visibleContainers.length, 0, 'Should start with no visible containers');
    assert.strictEqual(state1.allHyperEdges.length, 0, 'Should start with no hyperEdges');
    console.log('✓ State creation tests passed');
}
// ============ Node Management Tests ============
function testNodeManagement() {
    console.log('Testing node management...');
    const state = createTestState();
    // Test node creation
    state.setGraphNode('node1', {
        label: 'Test Node 1',
        style: NODE_STYLES.DEFAULT
    });
    // Test node retrieval
    const node1 = state.getGraphNode('node1');
    assert(node1, 'Node should exist');
    assert.strictEqual(node1.id, 'node1', 'Node should have correct id');
    assert.strictEqual(node1.label, 'Test Node 1', 'Node should have correct label');
    assert.strictEqual(node1.style, NODE_STYLES.DEFAULT, 'Node should have correct style');
    assert.strictEqual(node1.hidden, false, 'Node should not be hidden by default');
    // Test visible nodes collection
    const visibleNodes = state.visibleNodes;
    assert.strictEqual(visibleNodes.length, 1, 'Should have one visible node');
    assert.strictEqual(visibleNodes[0].id, 'node1', 'Visible node should be node1');
    // Test hiding nodes
    state.updateNode('node1', { hidden: true });
    assert.strictEqual(state.getGraphNode('node1')?.hidden, true, 'Node should be hidden');
    assert.strictEqual(state.visibleNodes.length, 0, 'Should have no visible nodes when hidden');
    // Test showing nodes
    state.updateNode('node1', { hidden: false });
    assert.strictEqual(state.getGraphNode('node1')?.hidden, false, 'Node should not be hidden');
    assert.strictEqual(state.visibleNodes.length, 1, 'Should have one visible node when shown');
    // Test node removal
    state.removeGraphNode('node1');
    assert.strictEqual(state.getGraphNode('node1'), undefined, 'Removed node should not exist');
    assert.strictEqual(state.visibleNodes.length, 0, 'Should have no visible nodes after removal');
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
    state.setGraphEdge('edge1', {
        source: 'node1',
        target: 'node2',
        style: EDGE_STYLES.DEFAULT
    });
    // Test edge retrieval
    const edge1 = state.getGraphEdge('edge1');
    assert(edge1, 'Edge should exist');
    assert.strictEqual(edge1.id, 'edge1', 'Edge should have correct id');
    assert.strictEqual(edge1.source, 'node1', 'Edge should have correct source');
    assert.strictEqual(edge1.target, 'node2', 'Edge should have correct target');
    assert.strictEqual(edge1.hidden, false, 'Edge should not be hidden by default');
    // Test edge visibility
    const visibleEdges = state.visibleEdges;
    assert.strictEqual(visibleEdges.length, 1, 'Should have one visible edge');
    // Test edge hiding
    state.updateEdge('edge1', { hidden: true });
    assert.strictEqual(state.getGraphEdge('edge1')?.hidden, true, 'Edge should be hidden');
    assert.strictEqual(state.visibleEdges.length, 0, 'Should have no visible edges when hidden');
    // Test edge removal
    state.removeGraphEdge('edge1');
    assert.strictEqual(state.getGraphEdge('edge1'), undefined, 'Removed edge should not exist');
    assert.strictEqual(state.visibleEdges.length, 0, 'Should have no visible edges after removal');
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
    state.setContainer('container1', {
        expandedDimensions: { width: 200, height: 150 },
        children: ['node1', 'node2']
    });
    // Test container retrieval
    const container1 = state.getContainer('container1');
    assert(container1, 'Container should exist');
    assert.strictEqual(container1.id, 'container1', 'Container should have correct id');
    assert.strictEqual(container1.collapsed, false, 'Container should not be collapsed by default');
    assert.strictEqual(container1.hidden, false, 'Container should not be hidden by default');
    // Test container hierarchy tracking
    assert.strictEqual(state.getNodeContainer('node1'), 'container1', 'Node1 should be in container1');
    assert.strictEqual(state.getNodeContainer('node2'), 'container1', 'Node2 should be in container1');
    // Test container visibility management
    const visibleContainers = state.visibleContainers;
    const expandedContainers = state.expandedContainers;
    assert.strictEqual(visibleContainers.length, 1, 'Should have one visible container');
    assert.strictEqual(expandedContainers.length, 1, 'Should have one expanded container');
    // Test container collapse
    state.updateContainer('container1', { collapsed: true });
    assert.strictEqual(state.getContainer('container1')?.collapsed, true, 'Container should be collapsed');
    assert.strictEqual(state.expandedContainers.length, 0, 'Should have no expanded containers when collapsed');
    // Test container hiding
    state.updateContainer('container1', { hidden: true });
    assert.strictEqual(state.getContainer('container1')?.hidden, true, 'Container should be hidden');
    assert.strictEqual(state.visibleContainers.length, 0, 'Should have no visible containers when hidden');
    console.log('✓ Container management tests passed');
}
// ============ HyperEdge Management Tests ============
function testHyperEdgeManagement() {
    console.log('Testing hyperEdge management...');
    const state = createTestState();
    // Test hyperEdge creation
    state.setHyperEdge('hyper1', {
        source: 'node1',
        target: 'container1',
        style: EDGE_STYLES.THICK,
        originalEdges: []
    });
    // Test hyperEdge retrieval
    const hyperEdge1 = state.getHyperEdge('hyper1');
    assert(hyperEdge1, 'HyperEdge should exist');
    assert.strictEqual(hyperEdge1.id, 'hyper1', 'HyperEdge should have correct id');
    assert.strictEqual(hyperEdge1.source, 'node1', 'HyperEdge should have correct source');
    assert.strictEqual(hyperEdge1.target, 'container1', 'HyperEdge should have correct target');
    assert.strictEqual(hyperEdge1.style, EDGE_STYLES.THICK, 'HyperEdge should have correct style');
    // Test hyperEdge collection
    const hyperEdges = state.allHyperEdges;
    assert.strictEqual(hyperEdges.length, 1, 'Should have one hyperEdge');
    assert.strictEqual(hyperEdges[0].id, 'hyper1', 'HyperEdge should be hyper1');
    // Test hyperEdge removal
    state.removeHyperEdge('hyper1');
    assert.strictEqual(state.getHyperEdge('hyper1'), undefined, 'Removed hyperEdge should not exist');
    assert.strictEqual(state.allHyperEdges.length, 0, 'Should have no hyperEdges after removal');
    console.log('✓ HyperEdge management tests passed');
}
// ============ Container Collapse/Expand Tests ============
function testContainerCollapseExpand() {
    console.log('Testing container collapse/expand...');
    const state = createTestState();
    // Create a test graph
    state.setGraphNode('node1', { label: 'Node 1' });
    state.setGraphNode('node2', { label: 'Node 2' });
    state.setGraphNode('node3', { label: 'External Node' });
    state.setGraphEdge('edge1-2', { source: 'node1', target: 'node2' });
    state.setGraphEdge('edge1-3', { source: 'node1', target: 'node3' });
    state.setContainer('container1', {
        children: ['node1', 'node2']
    });
    // Verify initial state
    assert.strictEqual(state.visibleNodes.length, 3, 'Should have 3 visible nodes initially');
    assert.strictEqual(state.visibleEdges.length, 2, 'Should have 2 visible edges initially');
    assert.strictEqual(state.allHyperEdges.length, 0, 'Should have no hyperEdges initially');
    // Test collapse
    state.collapseContainer('container1');
    // Check collapsed state
    assert.strictEqual(state.getContainer('container1')?.collapsed, true, 'Container should be collapsed');
    // Test expand
    state.expandContainer('container1');
    // Check expanded state
    assert.strictEqual(state.getContainer('container1')?.collapsed, false, 'Container should be expanded');
    console.log('✓ Container collapse/expand tests passed');
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
    // Verify elements exist using public API
    assert(state.visibleNodes.length > 0, 'Should have visible nodes before clear');
    assert(state.visibleEdges.length > 0, 'Should have visible edges before clear');
    assert(state.visibleContainers.length > 0, 'Should have visible containers before clear');
    assert(state.allHyperEdges.length > 0, 'Should have hyperEdges before clear');
    // Clear state
    state.clear();
    // Verify everything is cleared using public API
    assert.strictEqual(state.visibleNodes.length, 0, 'Should have no visible nodes after clear');
    assert.strictEqual(state.visibleEdges.length, 0, 'Should have no visible edges after clear');
    assert.strictEqual(state.visibleContainers.length, 0, 'Should have no visible containers after clear');
    assert.strictEqual(state.expandedContainers.length, 0, 'Should have no expanded containers after clear');
    assert.strictEqual(state.allHyperEdges.length, 0, 'Should have no hyperEdges after clear');
    // Test that we can add new entities after clear
    state.setGraphNode('newNode', { label: 'New Node' });
    assert.strictEqual(state.visibleNodes.length, 1, 'Should be able to add nodes after clear');
    console.log('✓ State clearing tests passed');
}
// ============ Run All Tests ============
function runAllTests() {
    return new Promise((resolve, reject) => {
        try {
            testStateCreation();
            testNodeManagement();
            testEdgeManagement();
            testContainerManagement();
            testHyperEdgeManagement();
            testContainerCollapseExpand();
            testClearState();
            console.log('\n🎉 All tests passed! VisualizationState is working correctly.');
            resolve();
        }
        catch (error) {
            console.error('\n❌ Test failed:', error instanceof Error ? error.message : String(error));
            if (error instanceof Error) {
                console.error(error.stack);
            }
            reject(error);
        }
    });
}
// Export for potential use in other test files
export { testStateCreation, testNodeManagement, testEdgeManagement, testContainerManagement, testHyperEdgeManagement, testContainerCollapseExpand, testClearState, runAllTests };
// Run tests if this file is executed directly
if (import.meta.url === `file://${process.argv[1]}`) {
    runAllTests().catch(() => process.exit(1));
}
//# sourceMappingURL=VisState.test.js.map