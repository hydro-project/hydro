/**
 * Test edge invariant validation to catch ELK "Referenced shape does not exist" errors
 * 
 * These tests validate the specific invariants that should catch bugs where:
 * 1. Visible edges reference non-existent entities
 * 2. Visible edges reference hidden entities  
 * 3. Collapsed containers don't have proper hyperEdge routing
 */

import { describe, test, expect } from 'vitest';
import { createVisualizationState } from '../VisState';

describe('Edge Invariant Validation', () => {
  
  test('should catch visible edges referencing non-existent containers', () => {
    const visState = createVisualizationState();
    
    // Create an edge that references a non-existent container
    visState.setGraphEdge('edge1', {
      source: 'node1', 
      target: 'bt_163', // This container doesn't exist!
      hidden: false
    });
    
    // This should throw because edge references non-existent target
    expect(() => {
      visState.validateInvariants();
    }).toThrow(/references non-existent target bt_163/);
  });

  test('should catch visible edges referencing hidden containers', () => {
    const visState = createVisualizationState();
    
    // Create a node and a collapsed+hidden container (valid state)
    visState.setGraphNode('node1', { label: 'Node 1' });
    visState.setContainer('container1', { 
      children: ['node2'],
      collapsed: true, // Must be collapsed if hidden
      hidden: true     // Container is hidden
    });
    visState.setGraphNode('node2', { label: 'Node 2', hidden: true });
    
    // Create an edge to the hidden container
    visState.setGraphEdge('edge1', {
      source: 'node1',
      target: 'container1', // References hidden container
      hidden: false // Edge is visible
    });
    
    // This should throw because edge references hidden target
    expect(() => {
      visState.validateInvariants();
    }).toThrow(/references hidden target container1/);
  });

  test('should allow edges to visible collapsed containers', () => {
    const visState = createVisualizationState();
    
    // Create a collapsed but visible container
    visState.setGraphNode('node1', { label: 'Node 1' });
    visState.setContainer('container1', { 
      children: ['node2'],
      collapsed: true, // Collapsed
      hidden: false    // But still visible
    });
    visState.setGraphNode('node2', { label: 'Node 2', hidden: true });
    
    // Create an edge to the visible collapsed container
    visState.setGraphEdge('edge1', {
      source: 'node1',
      target: 'container1', // References visible collapsed container
      hidden: false
    });
    
    // This should NOT throw - edges to visible collapsed containers are valid
    expect(() => {
      visState.validateInvariants();
    }).not.toThrow();
  });

  test('should catch missing hyperEdges for collapsed containers', () => {
    const visState = createVisualizationState();
    
    // Create a scenario with crossing edges but no hyperEdges
    visState.setGraphNode('nodeA', { label: 'Node A' });
    visState.setGraphNode('nodeB', { label: 'Node B' });
    visState.setGraphNode('nodeC', { label: 'Node C', hidden: true });
    
    visState.setContainer('container1', {
      children: ['nodeC'],
      collapsed: true,
      hidden: false
    });
    
    // Create a crossing edge that should be hidden when container is collapsed
    visState.setGraphEdge('crossingEdge', {
      source: 'nodeA',
      target: 'nodeB', 
      hidden: false // Should be hidden for collapsed container
    });
    
    // Mock getCrossingEdges to return our crossing edge
    const originalGetCrossingEdges = visState.getCrossingEdges;
    visState.getCrossingEdges = (containerId: string) => {
      if (containerId === 'container1') {
        return [visState.getGraphEdge('crossingEdge')];
      }
      return [];
    };
    
    // This should catch that we have a crossing edge but no hyperEdge
    expect(() => {
      visState.validateInvariants();
    }).toThrow(/no corresponding hyperEdge/);
    
    // Restore original method
    visState.getCrossingEdges = originalGetCrossingEdges;
  });

  test('should catch crossing edges that are not hidden when container is collapsed', () => {
    const visState = createVisualizationState();
    
    // Create nodes and container
    visState.setGraphNode('nodeA', { label: 'Node A' });
    visState.setGraphNode('nodeB', { label: 'Node B' });
    visState.setGraphNode('nodeC', { label: 'Node C', hidden: true });
    
    visState.setContainer('container1', {
      children: ['nodeC'],
      collapsed: true,
      hidden: false
    });
    
    // Create a crossing edge that is visible (should be hidden)
    visState.setGraphEdge('crossingEdge', {
      source: 'nodeA',
      target: 'nodeB',
      hidden: false // This should be hidden when container is collapsed
    });
    
    // Mock getCrossingEdges to return our crossing edge
    const originalGetCrossingEdges = visState.getCrossingEdges;
    visState.getCrossingEdges = (containerId: string) => {
      if (containerId === 'container1') {
        return [visState.getGraphEdge('crossingEdge')];
      }
      return [];
    };
    
    // This should catch that crossing edge is not hidden
    expect(() => {
      visState.validateInvariants();
    }).toThrow(/crosses collapsed container.*but is not hidden/);
    
    // Restore original method
    visState.getCrossingEdges = originalGetCrossingEdges;
  });

  test('should validate the exact browser error scenario', () => {
    const visState = createVisualizationState();
    
    // Recreate the exact scenario from the browser console error:
    // "Referenced shape does not exist: bt_163"
    
    // Create some nodes
    visState.setGraphNode('node1', { label: 'Node 1' });
    visState.setGraphNode('node2', { label: 'Node 2' });
    
    // Create an edge that references bt_163 (which doesn't exist)
    visState.setGraphEdge('edge_to_bt_163', {
      source: 'node1',
      target: 'bt_163', // This is the exact entity from the error!
      hidden: false
    });
    
    // This should catch the exact error pattern that ELK was hitting
    expect(() => {
      visState.validateInvariants();
    }).toThrow(/Edge edge_to_bt_163 references non-existent target bt_163/);
  });
});
