/**
 * @fileoverview Symmetric Inverse Tests
 * 
 * Tests basic operations on VisualizationState to ensure consistency
 */

import { describe, it, expect } from 'vitest';
import { createVisualizationState, VisualizationState } from '../VisState';

describe('SymmetricInverse', () => {
  describe('basic state operations', () => {
    it('should create VisualizationState', () => {
      const state = createVisualizationState();
      expect(state).toBeDefined();
      expect(Array.isArray(state.visibleNodes)).toBe(true);
      expect(Array.isArray(state.visibleEdges)).toBe(true);
      expect(Array.isArray(state.visibleContainers)).toBe(true);
    });

    it('should add and manage nodes', () => {
      const state = createVisualizationState();
      
      // Add test node
      state.setGraphNode('node1', { 
        label: 'Test Node 1',
        style: 'default',
        hidden: false 
      });
      
      const nodes = state.visibleNodes;
      expect(nodes.length).toBe(1);
      
      const node1 = nodes.find(n => n.id === 'node1');
      expect(node1).toBeDefined();
      expect(node1!.label).toBe('Test Node 1');
      expect(node1!.style).toBe('default');
    });

    it('should add and manage edges', () => {
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
      
      const edge1 = edges.find(e => e.id === 'edge1');
      expect(edge1).toBeDefined();
      expect(edge1!.source).toBe('node1');
      expect(edge1!.target).toBe('node2');
    });

    it('should add and manage containers', () => {
      const state = createVisualizationState();
      
      // Add container
      state.setContainer('container1', {
        style: 'default',
        collapsed: false
      });
      
      const containers = state.visibleContainers;
      expect(containers.length).toBe(1);
      
      const container1 = containers.find(c => c.id === 'container1');
      expect(container1).toBeDefined();
      expect(container1!.collapsed).toBe(false);
      expect(container1!.style).toBe('default');
    });
  });

  describe('container operations', () => {
    it('should expand and collapse containers', () => {
      const state = createVisualizationState();
      
      // Add container
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
      
      // Expand (symmetric operation)
      state.expandContainer('container1');
      container = state.visibleContainers.find(c => c.id === 'container1');
      expect(container!.collapsed).toBe(false);
    });

    it('should handle container operations on non-existent containers', () => {
      const state = createVisualizationState();
      
      // Should throw for non-existent containers (this is expected behavior)
      expect(() => {
        state.collapseContainer('nonexistent');
      }).toThrow();
      
      expect(() => {
        state.expandContainer('nonexistent');
      }).toThrow();
    });
  });

  describe('state consistency', () => {
    it('should maintain consistent state after operations', () => {
      const state = createVisualizationState();
      
      // Add test data
      state.setGraphNode('node1', { label: 'Node 1', style: 'default', hidden: false });
      state.setContainer('container1', { style: 'default', collapsed: false });
      
      const initialNodeCount = state.visibleNodes.length;
      const initialContainerCount = state.visibleContainers.length;
      
      // Perform operations
      state.collapseContainer('container1');
      state.expandContainer('container1');
      
      // State should be consistent
      expect(state.visibleNodes.length).toBe(initialNodeCount);
      expect(state.visibleContainers.length).toBe(initialContainerCount);
    });

    it('should handle multiple node additions', () => {
      const state = createVisualizationState();
      
      // Add multiple nodes
      for (let i = 0; i < 5; i++) {
        state.setGraphNode(`node${i}`, {
          label: `Node ${i}`,
          style: 'default',
          hidden: false
        });
      }
      
      expect(state.visibleNodes.length).toBe(5);
      
      // All nodes should be retrievable
      for (let i = 0; i < 5; i++) {
        const node = state.visibleNodes.find(n => n.id === `node${i}`);
        expect(node).toBeDefined();
        expect(node!.label).toBe(`Node ${i}`);
      }
    });
  });

  describe('edge cases', () => {
    it('should handle empty state operations', () => {
      const state = createVisualizationState();
      
      // Operations on empty state should throw for non-existent containers (expected behavior)
      expect(() => {
        state.collapseContainer('nonexistent');
      }).toThrow();
      
      expect(() => {
        state.expandContainer('nonexistent');
      }).toThrow();
      
      // State should remain empty and consistent
      expect(state.visibleNodes.length).toBe(0);
      expect(state.visibleEdges.length).toBe(0);
      expect(state.visibleContainers.length).toBe(0);
    });

    // TODO: Add more comprehensive tests when methods are available
    it.skip('should test node visibility operations', () => {
      // This would test hide/show operations when they become available
      expect(true).toBe(true);
    });

    it.skip('should verify all operation pairs are true inverses', () => {
      // This would test that applying operation A followed by operation B 
      // returns the system to exactly the original state for all symmetric pairs
      expect(true).toBe(true);
    });
  });
});
