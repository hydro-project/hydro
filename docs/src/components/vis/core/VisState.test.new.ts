/**
 * @fileoverview VisualizationState Tests
 * 
 * Tests for the core VisualizationState class
 */

import { describe, it, expect } from 'vitest';
import { createVisualizationState, VisualizationState } from '../core/VisState';

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
      
      state.setGraphNode('node1', {
        label: 'Test Node',
        style: 'default',
        hidden: false
      });
      
      const nodes = state.visibleNodes;
      expect(nodes.length).toBe(1);
      
      const node = nodes.find(n => n.id === 'node1');
      expect(node).toBeDefined();
      expect(node!.label).toBe('Test Node');
      expect(node!.style).toBe('default');
      expect(node!.hidden).toBe(false);
    });

    it('should update existing nodes', () => {
      const state = createVisualizationState();
      
      // Add initial node
      state.setGraphNode('node1', {
        label: 'Initial Label',
        style: 'default',
        hidden: false
      });
      
      // Update the node
      state.setGraphNode('node1', {
        label: 'Updated Label',
        style: 'highlighted',
        hidden: true
      });
      
      const nodes = state.visibleNodes;
      expect(nodes.length).toBe(1);
      
      const node = nodes.find(n => n.id === 'node1');
      expect(node!.label).toBe('Updated Label');
      expect(node!.style).toBe('highlighted');
      expect(node!.hidden).toBe(true);
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
      expect(container!.style).toBe('default');
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
    it.skip('should handle complex state modifications', () => {
      // This would test complex interactions between nodes, edges, and containers
      expect(true).toBe(true);
    });

    it.skip('should maintain state consistency', () => {
      // This would test that the state remains consistent after various operations
      expect(true).toBe(true);
    });
  });
});
