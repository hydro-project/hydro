/**
 * Symmetric Inverse Tests (TypeScript Version)
 * 
 * Tests that verify all symmetric function pairs are true inverses of each other.
 * These tests ensure that applying a function followed by its inverse returns
 * the system to exactly the original state.
 */

import { createVisualizationState, VisualizationState } from '../core/VisState';

/**
 * Create a deep copy of a VisualizationState for comparison
 * Note: Using public API only, so this is a functional copy rather than deep copy
 */
function copyState(state: VisualizationState): VisualizationState {
  const copy = createVisualizationState();
  
  // Copy visible nodes
  const nodes = state.visibleNodes;
  for (const node of nodes) {
    copy.setGraphNode(node.id, { 
      label: node.label,
      style: node.style,
      hidden: node.hidden 
    });
  }
  
  // Copy visible edges
  const edges = state.visibleEdges;
  for (const edge of edges) {
    copy.setGraphEdge(edge.id, { 
      source: edge.source,
      target: edge.target,
      style: edge.style,
      hidden: edge.hidden
    });
  }
  
  // Copy visible containers
  const containers = state.visibleContainers;
  for (const container of containers) {
    const children = Array.from(state.getContainerChildren(container.id));
    copy.setContainer(container.id, { 
      children,
      collapsed: container.collapsed,
      hidden: container.hidden,
      label: container.label
    });
  }
  
  // Copy hyperEdges
  const hyperEdges = state.allHyperEdges;
  for (const hyperEdge of hyperEdges) {
    copy.setHyperEdge(hyperEdge.id, { 
      source: hyperEdge.source,
      target: hyperEdge.target,
      style: hyperEdge.style,
      originalEdges: hyperEdge.originalEdges || []
    });
  }
  
  return copy;
}

/**
 * Compare two VisualizationState instances for functional equality
 */
function statesEqual(state1: VisualizationState, state2: VisualizationState, testName: string = ""): boolean {
  const errors: string[] = [];
  
  // Compare visible nodes
  const nodes1 = state1.visibleNodes;
  const nodes2 = state2.visibleNodes;
  
  if (nodes1.length !== nodes2.length) {
    errors.push(`Visible node count mismatch: ${nodes1.length} vs ${nodes2.length}`);
  }
  
  for (const node1 of nodes1) {
    const node2 = nodes2.find(n => n.id === node1.id);
    if (!node2) {
      errors.push(`Missing node ${node1.id} in state2`);
      continue;
    }
    
    if (node1.hidden !== node2.hidden) {
      errors.push(`Node ${node1.id} hidden mismatch: ${node1.hidden} vs ${node2.hidden}`);
    }
    
    if (node1.label !== node2.label) {
      errors.push(`Node ${node1.id} label mismatch: ${node1.label} vs ${node2.label}`);
    }
  }
  
  // Compare visible edges
  const edges1 = state1.visibleEdges;
  const edges2 = state2.visibleEdges;
  
  if (edges1.length !== edges2.length) {
    errors.push(`Visible edge count mismatch: ${edges1.length} vs ${edges2.length}`);
  }
  
  for (const edge1 of edges1) {
    const edge2 = edges2.find(e => e.id === edge1.id);
    if (!edge2) {
      errors.push(`Missing edge ${edge1.id} in state2`);
      continue;
    }
    
    if (edge1.hidden !== edge2.hidden) {
      errors.push(`Edge ${edge1.id} hidden mismatch: ${edge1.hidden} vs ${edge2.hidden}`);
    }
  }
  
  // Compare visible containers
  const containers1 = state1.visibleContainers;
  const containers2 = state2.visibleContainers;
  
  if (containers1.length !== containers2.length) {
    errors.push(`Visible container count mismatch: ${containers1.length} vs ${containers2.length}`);
  }
  
  for (const container1 of containers1) {
    const container2 = containers2.find(c => c.id === container1.id);
    if (!container2) {
      errors.push(`Missing container ${container1.id} in state2`);
      continue;
    }
    
    if (container1.collapsed !== container2.collapsed) {
      errors.push(`Container ${container1.id} collapsed mismatch: ${container1.collapsed} vs ${container2.collapsed}`);
    }
  }
  
  // Compare hyperEdges
  const hyperEdges1 = state1.allHyperEdges;
  const hyperEdges2 = state2.allHyperEdges;
  
  if (hyperEdges1.length !== hyperEdges2.length) {
    errors.push(`HyperEdge count mismatch: ${hyperEdges1.length} vs ${hyperEdges2.length}`);
  }
  
  if (errors.length > 0) {
    console.error(`State comparison failed for ${testName}:`);
    for (const error of errors) {
      console.error(`  - ${error}`);
    }
    return false;
  }
  
  return true;
}

/**
 * Test that container collapse and expand are true inverses
 */
function testCollapseExpandInverse(): void {
  console.log('Testing collapse/expand inverse property...');
  
  const state = createVisualizationState();
  
  // Create a test hierarchy
  state.setGraphNode('node1', { label: 'Node 1' });
  state.setGraphNode('node2', { label: 'Node 2' });
  state.setGraphNode('node3', { label: 'External Node' });
  
  state.setContainer('container1', {
    children: ['node1', 'node2'],
    label: 'Test Container'
  });
  
  state.setGraphEdge('edge1-2', { source: 'node1', target: 'node2' });
  state.setGraphEdge('edge1-3', { source: 'node1', target: 'node3' });
  
  // Capture initial state
  const initialState = copyState(state);
  
  // Apply collapse then expand
  state.collapseContainer('container1');
  state.expandContainer('container1');
  
  // Verify we're back to the initial state
  const finalState = copyState(state);
  
  if (!statesEqual(initialState, finalState, 'collapse->expand inverse')) {
    throw new Error('Collapse/expand is not a true inverse operation');
  }
  
  console.log('✓ Collapse/expand inverse test passed');
}

/**
 * Test multiple collapse/expand cycles
 */
function testMultipleCollapseExpandCycles(): void {
  console.log('Testing multiple collapse/expand cycles...');
  
  const state = createVisualizationState();
  
  // Create a more complex hierarchy
  state.setGraphNode('node1', { label: 'Node 1' });
  state.setGraphNode('node2', { label: 'Node 2' });
  state.setGraphNode('node3', { label: 'Node 3' });
  state.setGraphNode('external', { label: 'External Node' });
  
  state.setContainer('container1', {
    children: ['node1', 'node2'],
    label: 'Container 1'
  });
  
  state.setContainer('container2', {
    children: ['node3'],
    label: 'Container 2'
  });
  
  state.setGraphEdge('edge1-2', { source: 'node1', target: 'node2' });
  state.setGraphEdge('edge1-ext', { source: 'node1', target: 'external' });
  state.setGraphEdge('edge3-ext', { source: 'node3', target: 'external' });
  
  const initialState = copyState(state);
  
  // Perform multiple cycles
  for (let i = 0; i < 3; i++) {
    state.collapseContainer('container1');
    state.expandContainer('container1');
    
    state.collapseContainer('container2');
    state.expandContainer('container2');
    
    // Both containers
    state.collapseContainer('container1');
    state.collapseContainer('container2');
    state.expandContainer('container1');
    state.expandContainer('container2');
  }
  
  const finalState = copyState(state);
  
  if (!statesEqual(initialState, finalState, 'multiple collapse/expand cycles')) {
    throw new Error('Multiple collapse/expand cycles are not inverse operations');
  }
  
  console.log('✓ Multiple collapse/expand cycles test passed');
}

/**
 * Test nested container collapse/expand inverse
 */
function testNestedContainerInverse(): void {
  console.log('Testing nested container inverse...');
  
  const state = createVisualizationState();
  
  // Create nested containers
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
  
  state.setGraphEdge('edge1-2', { source: 'node1', target: 'node2' });
  state.setGraphEdge('edge1-ext', { source: 'node1', target: 'external' });
  
  const initialState = copyState(state);
  
  // Test various nested operations
  
  // Test 1: Collapse outer, expand outer
  state.collapseContainer('outerContainer');
  state.expandContainer('outerContainer');
  
  if (!statesEqual(initialState, copyState(state), 'nested: outer collapse/expand')) {
    throw new Error('Nested outer collapse/expand is not inverse');
  }
  
  // Test 2: Collapse inner, expand inner
  state.collapseContainer('innerContainer');
  state.expandContainer('innerContainer');
  
  if (!statesEqual(initialState, copyState(state), 'nested: inner collapse/expand')) {
    throw new Error('Nested inner collapse/expand is not inverse');
  }
  
  // Test 3: Complex nested sequence
  state.collapseContainer('innerContainer');
  state.collapseContainer('outerContainer');
  state.expandContainer('outerContainer');
  state.expandContainer('innerContainer');
  
  if (!statesEqual(initialState, copyState(state), 'nested: complex sequence')) {
    throw new Error('Nested complex sequence is not inverse');
  }
  
  console.log('✓ Nested container inverse test passed');
}

/**
 * Test that hide/show operations are inverses
 */
function testHideShowInverse(): void {
  console.log('Testing hide/show inverse...');
  
  const state = createVisualizationState();
  
  // Create test elements
  state.setGraphNode('node1', { label: 'Node 1' });
  state.setGraphNode('node2', { label: 'Node 2' });
  state.setGraphEdge('edge1', { source: 'node1', target: 'node2' });
  state.setContainer('container1', { children: ['node1'] });
  
  const initialState = copyState(state);
  
  // Test node hide/show
  state.updateNode('node1', { hidden: true });
  state.updateNode('node1', { hidden: false });
  
  if (!statesEqual(initialState, copyState(state), 'node hide/show')) {
    throw new Error('Node hide/show is not inverse');
  }
  
  // Test edge hide/show
  state.updateEdge('edge1', { hidden: true });
  state.updateEdge('edge1', { hidden: false });
  
  if (!statesEqual(initialState, copyState(state), 'edge hide/show')) {
    throw new Error('Edge hide/show is not inverse');
  }
  
  // Test container hide/show
  state.updateContainer('container1', { hidden: true });
  state.updateContainer('container1', { hidden: false });
  
  if (!statesEqual(initialState, copyState(state), 'container hide/show')) {
    throw new Error('Container hide/show is not inverse');
  }
  
  console.log('✓ Hide/show inverse test passed');
}

/**
 * Test clear and rebuild operations
 */
function testClearRebuildInverse(): void {
  console.log('Testing clear/rebuild operations...');
  
  const state = createVisualizationState();
  
  // Create complex state
  state.setGraphNode('node1', { label: 'Node 1' });
  state.setGraphNode('node2', { label: 'Node 2' });
  state.setGraphEdge('edge1', { source: 'node1', target: 'node2' });
  state.setContainer('container1', { children: ['node1'] });
  
  // Capture data for rebuilding
  const nodeData = state.visibleNodes.map(n => ({ 
    id: n.id, 
    label: n.label, 
    style: n.style 
  }));
  const edgeData = state.visibleEdges.map(e => ({
    id: e.id,
    source: e.source,
    target: e.target,
    style: e.style
  }));
  const containerData = state.visibleContainers.map(c => ({
    id: c.id,
    children: Array.from(state.getContainerChildren(c.id)),
    label: c.label
  }));
  
  const initialState = copyState(state);
  
  // Clear and rebuild
  state.clear();
  
  // Rebuild from captured data
  for (const node of nodeData) {
    state.setGraphNode(node.id, { label: node.label, style: node.style });
  }
  for (const edge of edgeData) {
    state.setGraphEdge(edge.id, { 
      source: edge.source, 
      target: edge.target, 
      style: edge.style 
    });
  }
  for (const container of containerData) {
    state.setContainer(container.id, { 
      children: container.children, 
      label: container.label 
    });
  }
  
  const rebuiltState = copyState(state);
  
  if (!statesEqual(initialState, rebuiltState, 'clear/rebuild')) {
    throw new Error('Clear/rebuild is not inverse');
  }
  
  console.log('✓ Clear/rebuild test passed');
}

// ============ Run All Tests ============

function runAllTests(): Promise<void> {
  return new Promise((resolve, reject) => {
    try {
      console.log('Running Symmetric Inverse tests...');
      
      testCollapseExpandInverse();
      testMultipleCollapseExpandCycles();
      testNestedContainerInverse();
      testHideShowInverse();
      testClearRebuildInverse();
      
      console.log('\n🎉 All symmetric inverse tests passed!');
      console.log('✅ All function pairs verified as mathematical inverses!');
      resolve();
    } catch (error: unknown) {
      console.error('\n❌ Symmetric inverse test failed:', error instanceof Error ? error.message : String(error));
      if (error instanceof Error) {
        console.error(error.stack);
      }
      reject(error);
    }
  });
}

// Export for potential use in other test files
export {
  testCollapseExpandInverse,
  testMultipleCollapseExpandCycles,
  testNestedContainerInverse,
  testHideShowInverse,
  testClearRebuildInverse,
  runAllTests
};

// Run tests if this file is executed directly
if (import.meta.url === `file://${process.argv[1]}`) {
  runAllTests().catch(() => process.exit(1));
}
