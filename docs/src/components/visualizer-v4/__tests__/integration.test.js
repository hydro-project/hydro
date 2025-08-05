/**
 * Integration test for visualizer-v4
 * 
 * Tests the integration of v3 core/bridges with v2 frontend
 */
import { describe, it, expect } from 'vitest';
import { createIntegratedStateManager } from '../integration/StateAdapter.js';

describe('Visualizer v4 Integration', () => {
  it('should integrate v3 core/bridges with v2 frontend patterns', () => {
    const stateManager = createIntegratedStateManager();
    
    // Test data in v2 format
    const testData = {
      nodes: [
        { id: 'node1', label: 'Node 1' },
        { id: 'node2', label: 'Node 2' }
      ],
      edges: [
        { id: 'edge1', source: 'node1', target: 'node2' }
      ],
      containers: [
        { 
          id: 'container1', 
          children: ['node1'],
          label: 'Container 1'
        }
      ]
    };
    
    // Set data using v2-style interface
    stateManager.setGraphData(testData);
    
    // Verify data is stored in v3 VisState
    const state = stateManager.getState();
    expect(state.nodes).toHaveLength(2);
    expect(state.edges).toHaveLength(1);
    expect(state.containers).toHaveLength(1);
    
    // Test container operations using v3 VisState
    const initialNodes = stateManager.getVisibleNodes().length;
    stateManager.collapseContainer('container1');
    const collapsedNodes = stateManager.getVisibleNodes().length;
    
    // Should have fewer visible nodes after collapse
    expect(collapsedNodes).toBeLessThan(initialNodes);
    
    // Expand should restore
    stateManager.expandContainer('container1');
    const expandedNodes = stateManager.getVisibleNodes().length;
    expect(expandedNodes).toBe(initialNodes);
  });

  it('should use v3 bridges for ReactFlow conversion', () => {
    const stateManager = createIntegratedStateManager();
    
    const testData = {
      nodes: [
        { id: 'node1', label: 'Node 1' },
        { id: 'node2', label: 'Node 2' }
      ],
      edges: [
        { id: 'edge1', source: 'node1', target: 'node2' }
      ]
    };
    
    stateManager.setGraphData(testData);
    
    // Convert using v3 ReactFlow bridge
    const reactFlowData = stateManager.getReactFlowData();
    
    expect(reactFlowData).toHaveProperty('nodes');
    expect(reactFlowData).toHaveProperty('edges');
    expect(reactFlowData.nodes).toHaveLength(2);
    expect(reactFlowData.edges).toHaveLength(1);
  });

  it('should maintain VisState as single source of truth', () => {
    const stateManager = createIntegratedStateManager();
    
    // Access to the underlying VisState
    expect(stateManager.visState).toBeDefined();
    expect(typeof stateManager.visState.setGraphNode).toBe('function');
    expect(typeof stateManager.visState.getGraphNode).toBe('function');
    
    // Bridges should be stateless
    expect(stateManager.elkBridge).toBeDefined();
    expect(stateManager.reactFlowBridge).toBeDefined();
  });
});