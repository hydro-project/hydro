/**
 * Container Collapse/Expand Tests
 * 
 * Tests the container collapse/expand functionality including edge lifting/grounding,
 * hyperedge creation/restoration, and complex cross-container scenarios.
 * These tests ensure symmetric operations and proper edge preservation.
 */

import { describe, it, expect } from 'vitest';
import { createVisualizationState, VisualizationState } from '../VisualizationState';

describe('Container Collapse/Expand Tests', () => {
  /**
   * Test simple grounding with minimal container scenario
   */
  it('should handle basic container collapse/expand functionality', () => {
    const state: VisualizationState = createVisualizationState();
    
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
    
    expect(internalNode?.hidden).toBe(false);
    expect(externalNode?.hidden).toBe(false);
    expect(edge?.hidden).toBe(false);
    
    // Count hyperEdges via visibleEdges (hyperEdges are included when containers are collapsed)
    const initialHyperEdges = state.visibleEdges.filter(e => e.id?.startsWith('hyper_'));
    expect(initialHyperEdges.length).toBe(0);
    
    // // console.log((('  Initial state verified')));
    
    // Collapse the container
    state.collapseContainer('container1');
    
    // Verify collapsed state using public API
    const internalNodeAfterCollapse = state.getGraphNode('internal');
    const externalNodeAfterCollapse = state.getGraphNode('external');
    const edgeAfterCollapse = state.getGraphEdge('edge1');
    
    expect(internalNodeAfterCollapse?.hidden).toBe(true);
    expect(externalNodeAfterCollapse?.hidden).toBe(false);
    expect(edgeAfterCollapse?.hidden).toBe(true);
    
    // Check for hyperEdges via visibleEdges (they have id starting with 'hyper_')
    const hyperEdges = state.visibleEdges.filter(e => e.id?.startsWith('hyper_'));
    expect(hyperEdges.length).toBe(1);
    
    const hyperEdge = hyperEdges[0];
    expect(hyperEdge.id).toBe('hyper_container1_to_external');
    
    // // console.log((('  Collapsed state verified')));
    
    // Expand the container
    state.expandContainer('container1');
    
    // Verify expanded state (should be exactly like initial state)
    const internalNodeAfterExpand = state.getGraphNode('internal');
    const externalNodeAfterExpand = state.getGraphNode('external');
    const edgeAfterExpand = state.getGraphEdge('edge1');
    
    expect(internalNodeAfterExpand?.hidden).toBe(false);
    expect(externalNodeAfterExpand?.hidden).toBe(false);
    expect(edgeAfterExpand?.hidden).toBe(false);
    
    const finalHyperEdges = state.visibleEdges.filter(e => e.id?.startsWith('hyper_'));
    expect(finalHyperEdges.length).toBe(0);
    
    // // console.log((('  Expanded state verified')));
    // // console.log((('✓ Simple grounding test passed')));
  });

  /**
   * Test multiple containers with interconnected nodes
   */
  it('should handle multiple containers with interconnected nodes', () => {
    const state: VisualizationState = createVisualizationState();
    
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
    expect(state.visibleNodes.length).toBe(5);
    expect(state.visibleEdges.length).toBe(5);
    
    const initialHyperEdges = state.visibleEdges.filter(e => e.id?.startsWith('hyper_'));
    expect(initialHyperEdges.length).toBe(0);
    
    // Collapse containerA
    state.collapseContainer('containerA');
    
    // Verify partial collapse state - containerA is replaced by a collapsed node
    expect(state.visibleNodes.length).toBe(3); // node3, node4, external (containerA is not in visibleNodes when collapsed)
    
    // Check visible edges (should include hyperEdges for A's external connections)
    const partialCollapseEdges = state.visibleEdges;
    const hyperEdgesAfterA = partialCollapseEdges.filter(e => e.id?.startsWith('hyper_'));
    expect(hyperEdgesAfterA.length).toBeGreaterThan(0); // Should have hyperEdges for A connections
    
    // Collapse containerB as well
    state.collapseContainer('containerB');
    
    // Verify full collapse state
    expect(state.visibleNodes.length).toBe(1); // Just external (both containers collapsed)
    
    const fullCollapseEdges = state.visibleEdges;
    const hyperEdgesAfterBoth = fullCollapseEdges.filter(e => e.id?.startsWith('hyper_'));
    expect(hyperEdgesAfterBoth.length).toBeGreaterThan(0); // Should have hyperEdges for connections
    
    // Expand both containers
    state.expandContainer('containerA');
    state.expandContainer('containerB');
    
    // Verify full expansion
    expect(state.visibleNodes.length).toBe(5);
    expect(state.visibleEdges.length).toBe(5);
    
    const finalHyperEdges = state.visibleEdges.filter(e => e.id?.startsWith('hyper_'));
    expect(finalHyperEdges.length).toBe(0);
    
    // // console.log((('✓ Multiple containers grounding test passed')));
  });

  /**
   * Test nested container grounding
   */
  it('should handle nested container grounding', () => {
    const state: VisualizationState = createVisualizationState();
    
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
    
    // Verify collapse - outer container contains the inner container
    expect(state.visibleNodes.length).toBe(1); // Just external (outer container is collapsed)
    
    // Check for hyperEdges (should have connection from outer to external)
    const hyperEdges = state.visibleHyperEdges;
    expect(hyperEdges.length).toBe(1);
    
    // Expand and verify restoration (using recursive expansion for nested case)
    state.expandContainerRecursive('outerContainer');
    
    // After expansion, we should see inner container + inner nodes + external
    expect(state.visibleNodes.length).toBe(3); // innerNode1, innerNode2, external (inner container is expanded by default)
    expect(state.visibleEdges.length).toBe(2);
    
    const finalHyperEdges = state.visibleHyperEdges;
    expect(finalHyperEdges.length).toBe(0);
    
    // // console.log((('✓ Nested container grounding test passed')));
  });

  /**
   * Test cross-container hyperedge preservation bug fix
   * 
   * This test reproduces the scenario where cross-container hyperedges disappear
   * when both containers are collapsed and then one is expanded.
   * 
   * Scenario:
   * 1. Two containers with nodes connected across containers
   * 2. Collapse both containers (creates cross-container hyperedges)
   * 3. Expand one container
   * 4. Verify cross-container connections are preserved
   */
  it('should preserve cross-container hyperedges when expanding one container while other remains collapsed', () => {
    const state: VisualizationState = createVisualizationState();
    
    // Create two containers with cross-container connections
    state.setGraphNode('nodeA1', { label: 'Node A1' });
    state.setGraphNode('nodeA2', { label: 'Node A2' });
    state.setGraphNode('nodeB1', { label: 'Node B1' });
    state.setGraphNode('nodeB2', { label: 'Node B2' });
    
    state.setContainer('containerA', {
      children: ['nodeA1', 'nodeA2'],
      label: 'Container A'
    });
    
    state.setContainer('containerB', {
      children: ['nodeB1', 'nodeB2'],
      label: 'Container B'
    });
    
    // Create cross-container edges (the key to reproducing the bug)
    state.setGraphEdge('crossEdge1', { source: 'nodeA1', target: 'nodeB1' });
    state.setGraphEdge('crossEdge2', { source: 'nodeA2', target: 'nodeB2' });
    // Add a reverse edge to test bidirectional hyperedges
    state.setGraphEdge('reverseEdge1', { source: 'nodeB1', target: 'nodeA1' });
    
    // Also create some internal edges for completeness
    state.setGraphEdge('internalA', { source: 'nodeA1', target: 'nodeA2' });
    state.setGraphEdge('internalB', { source: 'nodeB1', target: 'nodeB2' });
    
    // Verify initial state
    expect(state.visibleNodes.length).toBe(4);
    expect(state.visibleEdges.length).toBe(5);  // Updated count
    
    const initialHyperEdges = state.visibleEdges.filter(e => e.id?.startsWith('hyper_'));
    expect(initialHyperEdges.length).toBe(0);
    
    // // console.log((('  Initial state: 4 nodes, 5 edges, 0 hyperedges')));
    
    // Step 1: Collapse containerA
    state.collapseContainer('containerA');
    
    const afterCollapseA = state.visibleEdges.filter(e => e.id?.startsWith('hyper_'));
    expect(afterCollapseA.length).toBe(3); // Should have hyperEdges for A->B and B->A connections
    
    // // console.log((('  After collapsing A: found', afterCollapseA.length, 'hyperedges')));
    
    // Step 2: Collapse containerB (this creates cross-container hyperedges)
    state.collapseContainer('containerB');
    
    const afterCollapseB = state.visibleEdges.filter(e => e.id?.startsWith('hyper_'));
    expect(afterCollapseB.length).toBe(2); // Should have A <-> B hyperedges
    
    // Verify we have the cross-container hyperedges
    const crossContainerHyperEdges = afterCollapseB.filter(e => 
      (e.source === 'containerA' && e.target === 'containerB') ||
      (e.source === 'containerB' && e.target === 'containerA')
    );
    expect(crossContainerHyperEdges.length).toBe(2); // Both directions
    
    // // console.log((('  After collapsing B: found', afterCollapseB.length, 'hyperedges')));
    // // console.log((('  Cross-container hyperedges:', crossContainerHyperEdges.map(e => e.id))));
    
    // Step 3: Expand containerA (this is where the bug occurred)
    state.expandContainer('containerA');
    
    const afterExpandA = state.visibleEdges.filter(e => e.id?.startsWith('hyper_'));
    
    // CRITICAL: We should still have hyperedges connecting A's internal nodes to containerB
    expect(afterExpandA.length).toBeGreaterThan(0); // Should preserve connections to still-collapsed B
    
    // Check that we have hyperedges from A's nodes to containerB
    const nodeToContainerHyperEdges = afterExpandA.filter(e => 
      (e.source === 'nodeA1' || e.source === 'nodeA2') && e.target === 'containerB' ||
      (e.target === 'nodeA1' || e.target === 'nodeA2') && e.source === 'containerB'
    );
    expect(nodeToContainerHyperEdges.length).toBeGreaterThan(0); // This was the bug - these were disappearing
    
    // // console.log((('  After expanding A: found', afterExpandA.length, 'hyperedges')));
    // // console.log((('  Node-to-container hyperedges:', nodeToContainerHyperEdges.map(e => e.id))));
    
    // Step 4: Expand containerB to fully restore original state
    state.expandContainer('containerB');
    
    // Verify full restoration
    expect(state.visibleNodes.length).toBe(4);
    expect(state.visibleEdges.length).toBe(5);  // Updated count
    
    const finalHyperEdges = state.visibleEdges.filter(e => e.id?.startsWith('hyper_'));
    expect(finalHyperEdges.length).toBe(0);
    
    // Verify original edges are restored
    const crossEdge1 = state.getGraphEdge('crossEdge1');
    const crossEdge2 = state.getGraphEdge('crossEdge2');
    const reverseEdge1 = state.getGraphEdge('reverseEdge1');
    expect(crossEdge1?.hidden).toBe(false);
    expect(crossEdge2?.hidden).toBe(false);
    expect(reverseEdge1?.hidden).toBe(false);
    
    // // console.log((('  Final state: fully restored to original')));
    // // console.log((('✓ Cross-container hyperedge preservation test passed')));
  });

  /**
   * Test bidirectional cross-container hyperedges with multiple iterations
   * 
   * This test addresses two critical edge cases:
   * 1. Bidirectional edges between containers (A->B and B->A)
   * 2. Multiple collapse/expand cycles to detect state accumulation bugs
   * 
   * Scenario:
   * 1. Two containers with bidirectional cross-container connections
   * 2. Run collapse/expand cycle twice to detect state bugs
   * 3. Verify hyperedges are preserved correctly in both directions
   */
  it('should handle bidirectional cross-container hyperedges through multiple collapse/expand cycles', () => {
    const state: VisualizationState = createVisualizationState();
    
    // Create two containers with bidirectional cross-container connections
    state.setGraphNode('nodeA1', { label: 'Node A1' });
    state.setGraphNode('nodeA2', { label: 'Node A2' });
    state.setGraphNode('nodeB1', { label: 'Node B1' });
    state.setGraphNode('nodeB2', { label: 'Node B2' });
    
    state.setContainer('containerA', {
      children: ['nodeA1', 'nodeA2'],
      label: 'Container A'
    });
    
    state.setContainer('containerB', {
      children: ['nodeB1', 'nodeB2'],
      label: 'Container B'
    });
    
    // Create BIDIRECTIONAL cross-container edges (the key difference)
    state.setGraphEdge('A1_to_B1', { source: 'nodeA1', target: 'nodeB1' }); // A → B
    state.setGraphEdge('A2_to_B2', { source: 'nodeA2', target: 'nodeB2' }); // A → B
    state.setGraphEdge('B1_to_A1', { source: 'nodeB1', target: 'nodeA1' }); // B → A 
    state.setGraphEdge('B2_to_A2', { source: 'nodeB2', target: 'nodeA2' }); // B → A
    
    // Also create some internal edges
    state.setGraphEdge('internalA', { source: 'nodeA1', target: 'nodeA2' });
    state.setGraphEdge('internalB', { source: 'nodeB1', target: 'nodeB2' });
    
    // Verify initial state
    expect(state.visibleNodes.length).toBe(4);
    expect(state.visibleEdges.length).toBe(6);
    
    const initialHyperEdges = state.visibleEdges.filter(e => e.id?.startsWith('hyper_'));
    expect(initialHyperEdges.length).toBe(0);
    
    // // console.log((('  Initial state: 4 nodes, 6 edges (4 cross-container bidirectional), 0 hyperedges')));
    
    // Run the collapse/expand cycle TWICE to detect accumulation bugs
    for (let iteration = 1; iteration <= 2; iteration++) {
      // // console.log(((`  \n=== ITERATION ${iteration} ===`)));
      
      // Step 1: Collapse containerA
      state.collapseContainer('containerA');
      
      const afterCollapseA = state.visibleEdges.filter(e => e.id?.startsWith('hyper_'));
      expect(afterCollapseA.length).toBe(4); // Should have hyperEdges for both A→B and B→A directions
      
      // // console.log(((`  Iter ${iteration} - After collapsing A: found ${afterCollapseA.length} hyperedges`)));
      
      // Step 2: Collapse containerB (this creates cross-container hyperedges)
      state.collapseContainer('containerB');
      
      const afterCollapseB = state.visibleEdges.filter(e => e.id?.startsWith('hyper_'));
      expect(afterCollapseB.length).toBe(2); // Should have bidirectional A ↔ B hyperedges
      
      // Verify we have BOTH directions of cross-container hyperedges
      const crossContainerHyperEdges = afterCollapseB.filter(e => 
        (e.source === 'containerA' && e.target === 'containerB') ||
        (e.source === 'containerB' && e.target === 'containerA')
      );
      expect(crossContainerHyperEdges.length).toBe(2); // Both A→B and B→A
      
      // // console.log(((`  Iter ${iteration} - After collapsing B: found ${afterCollapseB.length} hyperedges`)));
      // // console.log(((`  Iter ${iteration} - Cross-container hyperedges:`, crossContainerHyperEdges.map(e => `${e.source}→${e.target}`))));
      
      // Step 3: Expand containerA (critical test point)
      state.expandContainer('containerA');
      
      const afterExpandA = state.visibleEdges.filter(e => e.id?.startsWith('hyper_'));
      
      // CRITICAL: We should still have hyperedges connecting A's internal nodes to containerB
      // This should work in BOTH directions since we have bidirectional edges
      expect(afterExpandA.length).toBeGreaterThan(0);
      
      // Check that we have hyperedges from A's nodes to containerB AND from containerB to A's nodes
      const nodeToContainerHyperEdges = afterExpandA.filter(e => 
        // A nodes → containerB
        ((e.source === 'nodeA1' || e.source === 'nodeA2') && e.target === 'containerB') ||
        // containerB → A nodes  
        (e.source === 'containerB' && (e.target === 'nodeA1' || e.target === 'nodeA2'))
      );
      expect(nodeToContainerHyperEdges.length).toBeGreaterThan(0);
      
      // // console.log(((`  Iter ${iteration} - After expanding A: found ${afterExpandA.length} hyperedges`)));
      // // console.log(((`  Iter ${iteration} - Node-to-container hyperedges:`, nodeToContainerHyperEdges.map(e => `${e.source}→${e.target}`))));
      
      // Step 4: Expand containerB to fully restore original state
      state.expandContainer('containerB');
      
      // Verify full restoration
      expect(state.visibleNodes.length).toBe(4);
      expect(state.visibleEdges.length).toBe(6);
      
      const finalHyperEdges = state.visibleEdges.filter(e => e.id?.startsWith('hyper_'));
      expect(finalHyperEdges.length).toBe(0);
      
      // Verify ALL original edges are restored (bidirectional)
      const A1_to_B1 = state.getGraphEdge('A1_to_B1');
      const A2_to_B2 = state.getGraphEdge('A2_to_B2');
      const B1_to_A1 = state.getGraphEdge('B1_to_A1');
      const B2_to_A2 = state.getGraphEdge('B2_to_A2');
      
      expect(A1_to_B1?.hidden).toBe(false);
      expect(A2_to_B2?.hidden).toBe(false);
      expect(B1_to_A1?.hidden).toBe(false);
      expect(B2_to_A2?.hidden).toBe(false);
      
      // // console.log(((`  Iter ${iteration} - Final state: fully restored to original`)));
      
      // Critical check: Verify no hyperedge accumulation between iterations
      if (iteration === 2) {
        // On the second iteration, make sure we don't have any leftover hyperedges
        // or corrupted state from the first iteration
        const allVisibleHyperEdges = state.visibleEdges.filter(e => e.id?.startsWith('hyper_'));
        expect(allVisibleHyperEdges.length).toBe(0);
        // // console.log(((`  Iter ${iteration} - ✅ No hyperedge accumulation detected`)));
      }
    }
    
    // // console.log((('✓ Bidirectional cross-container hyperedge test passed through multiple iterations')));
  });

  /**
   * Test hyperedge ID conflicts and edge case scenarios
   * 
   * This test specifically targets potential bugs with:
   * 1. Hyperedge ID conflicts when the same edge is processed multiple times
   * 2. Partial expansion scenarios (expand A, collapse A, expand B, etc.)
   * 3. Edge case ordering that might reveal state corruption
   */
  it('should handle complex expansion patterns and hyperedge ID conflicts', () => {
    const state: VisualizationState = createVisualizationState();
    
    // Create a more complex scenario with multiple cross-container connections
    state.setGraphNode('nodeA1', { label: 'Node A1' });
    state.setGraphNode('nodeA2', { label: 'Node A2' });
    state.setGraphNode('nodeB1', { label: 'Node B1' });
    state.setGraphNode('nodeB2', { label: 'Node B2' });
    state.setGraphNode('external', { label: 'External Node' });
    
    state.setContainer('containerA', {
      children: ['nodeA1', 'nodeA2'],
      label: 'Container A'
    });
    
    state.setContainer('containerB', {
      children: ['nodeB1', 'nodeB2'],
      label: 'Container B'
    });
    
    // Create a complex web of connections that could cause ID conflicts
    state.setGraphEdge('A1_to_B1', { source: 'nodeA1', target: 'nodeB1' });
    state.setGraphEdge('A1_to_B2', { source: 'nodeA1', target: 'nodeB2' }); 
    state.setGraphEdge('A2_to_B1', { source: 'nodeA2', target: 'nodeB1' });
    state.setGraphEdge('B1_to_A1', { source: 'nodeB1', target: 'nodeA1' }); // Creates potential ID conflicts
    state.setGraphEdge('A1_to_ext', { source: 'nodeA1', target: 'external' });
    state.setGraphEdge('B1_to_ext', { source: 'nodeB1', target: 'external' });
    
    // Verify initial state
    expect(state.visibleNodes.length).toBe(5);
    expect(state.visibleEdges.length).toBe(6);
    
    // // console.log((('  Complex scenario: 5 nodes, 6 edges with potential ID conflicts')));
    
    // Perform a complex sequence of operations that could reveal bugs
    
    // Step 1: Collapse both containers
    state.collapseContainer('containerA');
    state.collapseContainer('containerB');
    
    const afterBothCollapsed = state.visibleEdges.filter(e => e.id?.startsWith('hyper_'));
    // // console.log((('  After collapsing both:', afterBothCollapsed.map(e => e.id))));
    
    // Step 2: Expand A, then immediately collapse A again (potential state corruption)
    state.expandContainer('containerA');
    const afterExpandA = state.visibleEdges.filter(e => e.id?.startsWith('hyper_'));
    // // console.log((('  After expanding A:', afterExpandA.map(e => e.id))));
    
    state.collapseContainer('containerA'); // This might reuse IDs or create conflicts
    const afterRecollapseA = state.visibleEdges.filter(e => e.id?.startsWith('hyper_'));
    // // console.log((('  After re-collapsing A:', afterRecollapseA.map(e => e.id))));
    
    // Step 3: Now expand B, then expand A (different order)
    state.expandContainer('containerB');
    const afterExpandB = state.visibleEdges.filter(e => e.id?.startsWith('hyper_'));
    // // console.log((('  After expanding B:', afterExpandB.map(e => e.id))));
    
    state.expandContainer('containerA');
    const afterExpandBothNew = state.visibleEdges.filter(e => e.id?.startsWith('hyper_'));
    // // console.log((('  After expanding both (new order):', afterExpandBothNew.map(e => e.id))));
    
    // Step 4: Verify final state is correct
    expect(state.visibleNodes.length).toBe(5);
    expect(state.visibleEdges.length).toBe(6);
    expect(afterExpandBothNew.length).toBe(0); // No hyperedges should remain
    
    // Step 5: Verify all original edges are restored and not corrupted
    const A1_to_B1 = state.getGraphEdge('A1_to_B1');
    const A1_to_B2 = state.getGraphEdge('A1_to_B2');
    const A2_to_B1 = state.getGraphEdge('A2_to_B1');
    const B1_to_A1 = state.getGraphEdge('B1_to_A1');
    const A1_to_ext = state.getGraphEdge('A1_to_ext');
    const B1_to_ext = state.getGraphEdge('B1_to_ext');
    
    expect(A1_to_B1?.hidden).toBe(false);
    expect(A1_to_B2?.hidden).toBe(false);
    expect(A2_to_B1?.hidden).toBe(false);
    expect(B1_to_A1?.hidden).toBe(false);
    expect(A1_to_ext?.hidden).toBe(false);
    expect(B1_to_ext?.hidden).toBe(false);
    
    // // console.log((('✓ Complex expansion patterns and ID conflict test passed')));
  });
});
