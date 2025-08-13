/**
 * Test to reproduce the chat.json expansion bug
 * Specifically tests the scenario where collapsing loc_0, then loc_1, then expanding loc_0
 * creates duplicate hyperEdges and validation warnings
 */

import { createVisualizationState } from '../core/VisualizationState';
import type { VisualizationState } from '../core/VisualizationState';
import type { HyperEdge } from '../core/types';

describe('Chat Expansion Bug Tests', () => {
  let state: VisualizationState;

  beforeEach(() => {
    state = createVisualizationState();
    
    // Create containers
    state.addContainer('loc_0', { x: 100, y: 100, width: 200, height: 150 });
    state.addContainer('loc_1', { x: 400, y: 100, width: 200, height: 150 });
    
    // Add nodes to containers
    state.addGraphNode('0', { x: 10, y: 10, width: 30, height: 20, containerId: 'loc_0' });
    state.addGraphNode('1', { x: 50, y: 10, width: 30, height: 20, containerId: 'loc_0' });
    state.addGraphNode('7', { x: 10, y: 50, width: 30, height: 20, containerId: 'loc_0' });
    state.addGraphNode('8', { x: 50, y: 50, width: 30, height: 20, containerId: 'loc_0' });
    
    state.addGraphNode('2', { x: 10, y: 10, width: 30, height: 20, containerId: 'loc_1' });
    state.addGraphNode('3', { x: 50, y: 10, width: 30, height: 20, containerId: 'loc_1' });
    state.addGraphNode('4', { x: 10, y: 50, width: 30, height: 20, containerId: 'loc_1' });
    state.addGraphNode('5', { x: 50, y: 50, width: 30, height: 20, containerId: 'loc_1' });
    state.addGraphNode('6', { x: 90, y: 50, width: 30, height: 20, containerId: 'loc_1' });
    
    // Add edges between containers
    state.addGraphEdge('e1', { source: '1', target: '2' });
    state.addGraphEdge('e6', { source: '6', target: '7' });
  });

  test('CHAT BUG: collapse loc_0, collapse loc_1, expand loc_0 should not create duplicate hyperEdges', () => {
    console.log('\n=== CHAT BUG TEST START ===');
    
    // Initial state - get visible edges count
    const initialVisibleEdges = Array.from(state.visibleEdges);
    const initialHyperEdges = Array.from((state as any)._collections.hyperEdges.values());
    console.log('Initial visible edges:', initialVisibleEdges.length);
    console.log('Initial hyperEdges:', initialHyperEdges.length);

    // Step 1: Collapse loc_0
    console.log('\n--- Step 1: Collapse loc_0 ---');
    state.setContainerState('loc_0', { collapsed: true });
    const afterLoc0Collapse = Array.from((state as any)._collections.hyperEdges.values());
    console.log('After loc_0 collapse - hyperEdges:', afterLoc0Collapse.length);

    // Step 2: Collapse loc_1  
    console.log('\n--- Step 2: Collapse loc_1 ---');
    state.setContainerState('loc_1', { collapsed: true });
    const afterLoc1Collapse = Array.from((state as any)._collections.hyperEdges.values());
    console.log('After loc_1 collapse - hyperEdges:', afterLoc1Collapse.length);

    // Step 3: Expand loc_0 (this is where the bug would occur)
    console.log('\n--- Step 3: Expand loc_0 (BUG REPRODUCTION) ---');
    state.setContainerState('loc_0', { collapsed: false });
    const afterLoc0Expansion = Array.from((state as any)._collections.hyperEdges.values());
    console.log('After loc_0 expansion - hyperEdges:', afterLoc0Expansion.length);

    // Check for duplicate hyperEdge IDs
    const hyperEdgeIds = afterLoc0Expansion.map((he: HyperEdge) => he.id);
    const uniqueIds = new Set(hyperEdgeIds);
    
    console.log('HyperEdge IDs:', hyperEdgeIds);
    console.log('Unique IDs:', Array.from(uniqueIds));
    
    expect(hyperEdgeIds.length).toBe(uniqueIds.size); // No duplicates
    
    // Check that all visible edges have unique IDs
    const visibleEdges = Array.from(state.visibleEdges);
    const visibleEdgeIds = visibleEdges.map(edge => edge.id);
    const uniqueVisibleIds = new Set(visibleEdgeIds);
    
    console.log('Visible edge IDs:', visibleEdgeIds);
    console.log('Unique visible IDs:', Array.from(uniqueVisibleIds));
    
    expect(visibleEdgeIds.length).toBe(uniqueVisibleIds.size); // No duplicates

    // Validate the state
    state.validateInvariants();
    
    console.log('=== CHAT BUG TEST END ===\n');
  });
});
