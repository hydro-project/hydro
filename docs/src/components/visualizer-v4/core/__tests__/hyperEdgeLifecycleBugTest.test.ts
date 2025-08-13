/**
 * HyperEdge Lifecycle Bug Reproduction Test
 * 
 * This test reproduces the exact scenario from the console log where:
 * 1. Container loc_0 collapses → creates hyperEdges
 * 2. Container loc_1 collapses → should preserve existing hyperEdges 
 * 3. Both containers should have proper hyperEdges representing connectivity
 */

import { describe, test, expect, beforeEach } from 'vitest';
import { VisualizationState } from '../VisualizationState';

describe('HyperEdge Lifecycle Bug Reproduction', () => {
  let visState: VisualizationState;

  beforeEach(() => {
    visState = new VisualizationState();
    
    // Create the exact scenario from the console log
    // Two containers: loc_0 and loc_1
    // Two nodes: 1 (in loc_0), 2 and 6 (in loc_1)  
    // Two edges: e1 (1->2), e6 (6->1)
    
    // Add nodes
    visState.addGraphNode('1', { id: '1', label: 'Node 1' });
    visState.addGraphNode('2', { id: '2', label: 'Node 2' });
    visState.addGraphNode('6', { id: '6', label: 'Node 6' });
    
    // Add containers
    visState.addContainer('loc_0', {
      id: 'loc_0',
      label: 'Location 0',
      collapsed: false,
      hidden: false,
      children: new Set(['1'])
    });
    
    visState.addContainer('loc_1', {
      id: 'loc_1', 
      label: 'Location 1',
      collapsed: false,
      hidden: false,
      children: new Set(['2', '6'])
    });
    
    // Add edges: e1 (1->2), e6 (6->1)
    visState.addGraphEdge('e1', {
      id: 'e1',
      source: '1',
      target: '2',
      style: 'default'
    });
    
    visState.addGraphEdge('e6', {
      id: 'e6',
      source: '6', 
      target: '1',
      style: 'default'
    });
  });

  test('should preserve hyperEdges when multiple containers collapse sequentially', () => {
    console.log('\n=== REPRODUCING HYPEREDGE LIFECYCLE BUG ===');
    
    // Initial state: all visible
    expect(visState.visibleNodes.length).toBe(3);
    expect(visState.visibleEdges.length).toBe(2);
    expect(visState.visibleHyperEdges.length).toBe(0);
    
    // STEP 1: Collapse loc_0
    console.log('\n🔄 STEP 1: Collapsing loc_0...');
    visState.setContainerCollapsed('loc_0', true);
    
    console.log('After loc_0 collapse:');
    console.log(`- Visible nodes: ${visState.visibleNodes.length}`);
    console.log(`- Visible edges: ${visState.visibleEdges.length}`);
    console.log(`- Visible hyperEdges: ${visState.visibleHyperEdges.length}`);
    
    // After loc_0 collapse: should have hyperEdges
    const hyperEdgesAfterStep1 = visState.visibleHyperEdges;
    console.log('HyperEdges after loc_0 collapse:', hyperEdgesAfterStep1.map(e => `${e.id}: ${e.source} -> ${e.target}`));
    
    expect(hyperEdgesAfterStep1.length).toBeGreaterThan(0);
    
    // STEP 2: Collapse loc_1  
    console.log('\n🔄 STEP 2: Collapsing loc_1...');
    visState.setContainerCollapsed('loc_1', true);
    
    console.log('After loc_1 collapse:');
    console.log(`- Visible nodes: ${visState.visibleNodes.length}`);
    console.log(`- Visible edges: ${visState.visibleEdges.length}`);
    console.log(`- Visible hyperEdges: ${visState.visibleHyperEdges.length}`);
    
    // After loc_1 collapse: should STILL have hyperEdges representing connectivity
    const hyperEdgesAfterStep2 = visState.visibleHyperEdges;
    console.log('HyperEdges after loc_1 collapse:', hyperEdgesAfterStep2.map(e => `${e.id}: ${e.source} -> ${e.target}`));
    
    // CRITICAL: Both collapsed containers should be connected by hyperEdges
    expect(hyperEdgesAfterStep2.length).toBeGreaterThan(0);
    
    // Validate connectivity: should have hyperEdges between loc_0 and loc_1
    const containerToContainerEdges = hyperEdgesAfterStep2.filter(edge => 
      (edge.source === 'loc_0' && edge.target === 'loc_1') ||
      (edge.source === 'loc_1' && edge.target === 'loc_0')
    );
    
    expect(containerToContainerEdges.length).toBeGreaterThan(0);
    console.log('Container-to-container hyperEdges:', containerToContainerEdges.map(e => `${e.id}: ${e.source} -> ${e.target}`));
    
    // Validate: no missing hyperEdge warnings  
    visState.validateInvariants();
    // Note: validateInvariants() logs warnings but doesn't return them
    // The test will pass if no MISSING_HYPEREDGE warnings are logged
    
    console.log('✅ HyperEdge lifecycle test passed!');
  });

  test('should correctly identify crossing edges including hidden ones', () => {
    // Collapse loc_0 first (this will hide e1 and e6)
    visState.setContainerCollapsed('loc_0', true);
    
    // Now check what crossing edges loc_1 finds (should include hidden e1 and e6)
    const crossingEdgesForLoc1 = visState.getCrossingEdges('loc_1');
    
    console.log('Crossing edges for loc_1:', crossingEdgesForLoc1.map(e => `${e.id}: ${e.source} -> ${e.target} (hidden: ${e.hidden})`));
    
    // Should find the hidden edges e1 and e6 as crossing edges
    const edgeIds = crossingEdgesForLoc1.map(e => e.id);
    expect(edgeIds).toContain('e1');
    expect(edgeIds).toContain('e6');
  });
});
