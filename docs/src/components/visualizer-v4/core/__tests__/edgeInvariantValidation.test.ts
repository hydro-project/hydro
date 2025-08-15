/**
 * Test ed  test('should catch visible edges referencing non-existent containers', () => {
    const visState = createVisualizationState();
    
    // This should throw immediately at the API boundary because edge references non-existent nodes
    expect(() => {
      visState.setGraphEdge('edge1', {
        source: 'node1', 
        target: 'bt_163', // This container doesn't exist!
        hidden: false
      });
    }).toThrow('VisualizationState invariant violations detected');
  });lidation to catch ELK "Referenced shape does not exist" errors
 * 
 * These tests validate the specific invariants that should catch bugs where:
 * 1. Visible edges reference non-existent entities
 * 2. Visible edges reference hidden entities  
 * 3. Collapsed containers don't have proper hyperEdge routing
 */

import { describe, test, expect } from 'vitest';
import { createVisualizationState } from '../VisualizationState';

describe('Edge Invariant Validation', () => {
  
  test('should catch visible edges referencing non-existent containers', () => {
    const visState = createVisualizationState();
    
    // This should throw immediately when trying to create edge with non-existent source/target
    expect(() => {
      visState.setGraphEdge('edge1', {
        source: 'node1', 
        target: 'bt_163', // This container doesn't exist!
        hidden: false
      });
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
    
    // This should throw immediately when creating edge to hidden container
    expect(() => {
      visState.setGraphEdge('edge1', {
        source: 'node1',
        target: 'container1', // References hidden container
        hidden: false // Edge is visible
      });
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
    // No need to manually validate, API boundary validation would have thrown if invalid
  });

  test('should validate the exact browser error scenario', () => {
    const visState = createVisualizationState();
    
    // Recreate the exact scenario from the browser console error:
    // "Referenced shape does not exist: bt_163"
    
    // Create some nodes
    visState.setGraphNode('node1', { label: 'Node 1' });
    visState.setGraphNode('node2', { label: 'Node 2' });
    
    // This should throw immediately when trying to create edge to non-existent target
    expect(() => {
      visState.setGraphEdge('edge_to_bt_163', {
        source: 'node1',
        target: 'bt_163', // This is the exact entity from the error!
        hidden: false
      });
    }).toThrow(/Edge edge_to_bt_163 references non-existent target bt_163/);
  });
});
