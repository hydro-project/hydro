/**
 * Simple Grounding Test (Fixed TypeScript Version)
 * 
 * Tests the basic container collapse/expand functionality with a minimal scenario.
 * This serves as a simple smoke test for the core visualization state operations.
 */

import { describe, it, expect } from 'vitest';
import { createVisualizationState, VisualizationState } from '../VisState';

describe('Simple Grounding Tests', () => {
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
    
    console.log('  Initial state verified');
    
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
    
    console.log('  Collapsed state verified');
    
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
    
    console.log('  Expanded state verified');
    console.log('✓ Simple grounding test passed');
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
    
    console.log('✓ Multiple containers grounding test passed');
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
    const hyperEdges = state.visibleEdges.filter(e => e.id?.startsWith('hyper_'));
    expect(hyperEdges.length).toBe(1);
    
    // Expand and verify restoration
    state.expandContainer('outerContainer');
    
    // After expansion, we should see inner container + inner nodes + external
    expect(state.visibleNodes.length).toBe(3); // innerNode1, innerNode2, external (inner container is expanded by default)
    expect(state.visibleEdges.length).toBe(2);
    
    const finalHyperEdges = state.visibleEdges.filter(e => e.id?.startsWith('hyper_'));
    expect(finalHyperEdges.length).toBe(0);
    
    console.log('✓ Nested container grounding test passed');
  });
});
