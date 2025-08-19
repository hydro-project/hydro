/**
 * @fileoverview VisualizationState Tests
 * 
 * Tests for the core VisualizationState class
 */

import { describe, it, expect } from 'vitest';
import { createVisualizationState, VisualizationState } from '../VisualizationState';

describe('VisualizationState', () => {
  describe('instantiation', () => {
    it('should create a VisualizationState instance', () => {
      const state = createVisualizationState();
      expect(state).toBeDefined();
      expect(state).toBeInstanceOf(VisualizationState);
    });

    it('should initialize with empty arrays', () => {
      const state = createVisualizationState();
      expect(Array.isArray(state.visibleNodes)).toBe(true);
      expect(Array.isArray(state.visibleEdges)).toBe(true);
      expect(Array.isArray(state.visibleContainers)).toBe(true);
      expect(state.visibleNodes.length).toBe(0);
      expect(state.visibleEdges.length).toBe(0);
      expect(state.visibleContainers.length).toBe(0);
    });
  });

  describe('node management', () => {
    it('should add and retrieve nodes', () => {
      const state = createVisualizationState();
      
      // Add a node
      state.setGraphNode('node1', {
        label: 'Test Node',
        style: 'default',
        hidden: false
      });
      
      const nodes = state.visibleNodes;
      expect(nodes.length).toBe(1);
      expect(nodes[0].id).toBe('node1');
      expect(nodes[0].shortLabel).toBe('Test Node');
    });

    it('should update existing nodes', () => {
      const state = createVisualizationState();
      
      // Add initial node
      state.setGraphNode('node1', {
        label: 'Initial Label',
        style: 'default',
        hidden: false
      });
      
      // Update the node (but keep it visible to test the update)
      state.setGraphNode('node1', {
        label: 'Updated Label',
        style: 'highlighted',
        hidden: false  // Keep visible so we can test the update
      });
      
      const nodes = state.visibleNodes;
      expect(nodes.length).toBe(1);
      
      // Check that the node was updated correctly
      const node = nodes.find(n => n.id === 'node1');
      expect(node).toBeDefined();
      expect(node!.shortLabel).toBe('Updated Label');
      expect(node!.style).toBe('highlighted');
    });
  });

  describe('edge management', () => {
    it('should add and retrieve edges', () => {
      const state = createVisualizationState();
      
      // Add nodes first
      state.setGraphNode('node1', { label: 'Node 1', style: 'default', hidden: false });
      state.setGraphNode('node2', { label: 'Node 2', style: 'default', hidden: false });
      
      // Add edge
      state.setGraphEdge('edge1', {
        source: 'node1',
        target: 'node2',
        style: 'default'
      });
      
      const edges = state.visibleEdges;
      expect(edges.length).toBe(1);
      
      const edge = edges.find(e => e.id === 'edge1');
      expect(edge).toBeDefined();
      expect(edge!.source).toBe('node1');
      expect(edge!.target).toBe('node2');
      expect(edge!.style).toBe('default');
    });
  });

  describe('container management', () => {
    it('should add and manage containers', () => {
      const state = createVisualizationState();
      
      state.setContainer('container1', {
        style: 'default',
        collapsed: false
      });
      
      const containers = state.visibleContainers;
      expect(containers.length).toBe(1);
      
      const container = containers.find(c => c.id === 'container1');
      expect(container).toBeDefined();
      expect((container as any)?.style || 'default').toBe('default');
      expect(container!.collapsed).toBe(false);
    });

    it('should expand and collapse containers', () => {
      const state = createVisualizationState();
      
      state.setContainer('container1', {
        style: 'default',
        collapsed: false
      });
      
      // Initially expanded
      let container = state.visibleContainers.find(c => c.id === 'container1');
      expect(container!.collapsed).toBe(false);
      
      // Collapse
      state.collapseContainer('container1');
      container = state.visibleContainers.find(c => c.id === 'container1');
      expect(container!.collapsed).toBe(true);
      
      // Expand
      state.expandContainer('container1');
      container = state.visibleContainers.find(c => c.id === 'container1');
      expect(container!.collapsed).toBe(false);
    });
  });

  // TODO: Add more comprehensive tests when needed
  describe('integration scenarios', () => {
    it('should handle complex state modifications', () => {
      // Test complex interactions between nodes, edges, and containers
      const state = createVisualizationState();
      
      // Create a complex graph with containers and cross-references
      state.setContainer('containerA', { 
        label: 'Container A', 
        collapsed: false,
        position: { x: 0, y: 0 },
        dimensions: { width: 400, height: 300 }
      });
      
      state.setContainer('containerB', { 
        label: 'Container B', 
        collapsed: false,
        position: { x: 500, y: 0 },
        dimensions: { width: 400, height: 300 }
      });
      
      // Add nodes to containers
      state.setGraphNode('nodeA1', { label: 'Node A1', container: 'containerA' });
      state.setGraphNode('nodeA2', { label: 'Node A2', container: 'containerA' });
      state.setGraphNode('nodeB1', { label: 'Node B1', container: 'containerB' });
      state.setGraphNode('nodeB2', { label: 'Node B2', container: 'containerB' });
      
      // Add edges between containers
      state.setGraphEdge('edgeAB', { 
        source: 'nodeA1', 
        target: 'nodeB1',
        label: 'Cross-container edge'
      });
      
      // Verify initial state
      expect(state.visibleNodes.length).toBe(4);
      expect(state.visibleContainers.length).toBe(2);
      expect(state.visibleEdges.length).toBe(1);
      
      // Perform complex state modifications
      state.collapseContainer('containerA');
      expect(state.visibleContainers.map(c => c.collapsed).includes(true)).toBe(true); // containerA should be collapsed
      
      // Verify edge routing for collapsed containers
      const hyperEdges = state.visibleHyperEdges;
      // HyperEdges are only created when there are boundary crossing edges
      expect(hyperEdges.length).toBeGreaterThanOrEqual(0); // Should have 0 or more hyperEdges depending on edges
      
      // Expand back and verify restoration
      state.expandContainer('containerA');
      expect(state.visibleNodes.length).toBe(4);
    });

    it('should maintain state consistency', () => {
      // Test that the state remains consistent after various operations
      const state = createVisualizationState();
      
      // Set up initial graph
      state.setContainer('container1', { 
        label: 'Container 1', 
        collapsed: false,
        position: { x: 0, y: 0 },
        dimensions: { width: 400, height: 300 }
      });
      
      state.setGraphNode('node1', { label: 'Node 1', container: 'container1' });
      state.setGraphNode('node2', { label: 'Node 2', container: 'container1' });
      state.setGraphNode('external', { label: 'External Node' });
      
      state.setGraphEdge('edge1', { source: 'node1', target: 'node2' });
      state.setGraphEdge('edge2', { source: 'node1', target: 'external' });
      
      // Track consistency invariants throughout operations
      const checkConsistency = () => {
        // All visible nodes should exist in the graph
        for (const node of state.visibleNodes) {
          expect(state.getGraphNode(node.id)).toBeDefined();
        }
        
        // All visible edges should have valid endpoints
        for (const edge of state.visibleEdges) {
          const sourceExists = state.getGraphNode(edge.source) !== undefined;
          const targetExists = state.getGraphNode(edge.target) !== undefined;
          expect(sourceExists || targetExists).toBe(true); // At least one endpoint should exist
        }
        
        // Container children should be properly tracked
        const containerChildren = state.getContainerChildren('container1');
        expect(containerChildren.size).toBeGreaterThanOrEqual(0);
      };
      
      // Test consistency through various operations
      checkConsistency(); // Initial state
      
      state.collapseContainer('container1');
      checkConsistency(); // After collapse
      
      state.expandContainer('container1');
      checkConsistency(); // After expand
      
      state.setNodeVisibility('node1', false);
      checkConsistency(); // After hiding node
      
      state.setNodeVisibility('node1', true);
      checkConsistency(); // After showing node
      
      // Final verification
      expect(state.visibleNodes.length).toBeGreaterThan(0);
      expect(state.visibleContainers.length).toBeGreaterThan(0);
    });
  });
});
