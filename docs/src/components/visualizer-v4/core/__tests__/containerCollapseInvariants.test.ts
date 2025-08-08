/**
 * @fileoverview Container Collapse Invariant Tests
 * 
 * These tests ensure that when containers are collapsed:
 * 1. Child nodes have hidden: true
 * 2. Child edges have hidden: true  
 * 3. Nested child containers are properly handled
 * 4. VisState never leaks hidden elements
 */

import { describe, test, expect, beforeEach } from 'vitest';
import { createVisualizationState } from '../VisState';
import type { VisualizationState } from '../VisState';

describe('Container Collapse Invariant Tests', () => {
  let visState: VisualizationState;

  beforeEach(() => {
    visState = createVisualizationState();
  });

  describe('Node Hiding Invariants', () => {
    test('should set hidden: true on nodes when their parent container is collapsed', () => {
      // Setup: Create container with child nodes
      visState.setContainer('container1', {
        children: ['node1', 'node2'],
        collapsed: false,
        hidden: false
      });
      
      visState.setGraphNode('node1', { label: 'Node 1', hidden: false });
      visState.setGraphNode('node2', { label: 'Node 2', hidden: false });
      
      // Initially: nodes should be visible
      expect(visState.getGraphNode('node1')?.hidden).toBe(false);
      expect(visState.getGraphNode('node2')?.hidden).toBe(false);
      
      // INVARIANT TEST: Collapse container - child nodes must become hidden
      visState.collapseContainer('container1');
      
      // After collapse: nodes should be hidden
      expect(visState.getGraphNode('node1')?.hidden).toBe(true);
      expect(visState.getGraphNode('node2')?.hidden).toBe(true);
      
      // INVARIANT: visibleNodes should not include hidden nodes
      const visibleNodeIds = visState.visibleNodes.map(n => n.id);
      expect(visibleNodeIds).not.toContain('node1');
      expect(visibleNodeIds).not.toContain('node2');
    });

    test('should set hidden: false on nodes when their parent container is expanded', () => {
      // Setup: Container collapsed with hidden children
      visState.setContainer('container1', {
        children: ['node1', 'node2'],
        collapsed: true,
        hidden: false
      });
      
      visState.setGraphNode('node1', { label: 'Node 1', hidden: true });
      visState.setGraphNode('node2', { label: 'Node 2', hidden: true });
      
      // Initially: nodes should be hidden
      expect(visState.getGraphNode('node1')?.hidden).toBe(true);
      expect(visState.getGraphNode('node2')?.hidden).toBe(true);
      
      // INVARIANT TEST: Expand container - child nodes must become visible
      visState.expandContainer('container1');
      
      // After expand: nodes should be visible
      expect(visState.getGraphNode('node1')?.hidden).toBe(false);
      expect(visState.getGraphNode('node2')?.hidden).toBe(false);
      
      // INVARIANT: visibleNodes should include visible nodes
      const visibleNodeIds = visState.visibleNodes.map(n => n.id);
      expect(visibleNodeIds).toContain('node1');
      expect(visibleNodeIds).toContain('node2');
    });

    test('should handle nested containers correctly', () => {
      // Setup: Nested container hierarchy
      visState.setContainer('outer', {
        children: ['inner'],
        collapsed: false,
        hidden: false
      });
      
      visState.setContainer('inner', {
        children: ['node1', 'node2'],
        collapsed: false,
        hidden: false
      });
      
      visState.setGraphNode('node1', { label: 'Node 1', hidden: false });
      visState.setGraphNode('node2', { label: 'Node 2', hidden: false });
      
      // Initially: all should be visible
      expect(visState.getGraphNode('node1')?.hidden).toBe(false);
      expect(visState.getGraphNode('node2')?.hidden).toBe(false);
      
      // INVARIANT TEST: Collapse outer container - all descendants should be hidden
      visState.collapseContainer('outer');
      
      // After collapse: nodes should be hidden (transitively)
      expect(visState.getGraphNode('node1')?.hidden).toBe(true);
      expect(visState.getGraphNode('node2')?.hidden).toBe(true);
      
      // Inner container should also be hidden
      expect(visState.getContainer('inner')?.hidden).toBe(true);
      
      // INVARIANT: visibleNodes should not include any descendants
      const visibleNodeIds = visState.visibleNodes.map(n => n.id);
      expect(visibleNodeIds).not.toContain('node1');
      expect(visibleNodeIds).not.toContain('node2');
      
      const visibleContainerIds = visState.visibleContainers.map(c => c.id);
      expect(visibleContainerIds).not.toContain('inner');
      expect(visibleContainerIds).toContain('outer'); // Collapsed containers are still visible
    });
  });

  describe('Edge Hiding Invariants', () => {
    test('should set hidden: true on edges when their endpoints are in collapsed containers', () => {
      // Setup: Two containers with nodes and edge between them
      visState.setContainer('container1', {
        children: ['node1'],
        collapsed: false,
        hidden: false
      });
      
      visState.setContainer('container2', {
        children: ['node2'],
        collapsed: false,
        hidden: false
      });
      
      visState.setGraphNode('node1', { label: 'Node 1', hidden: false });
      visState.setGraphNode('node2', { label: 'Node 2', hidden: false });
      visState.setGraphEdge('edge1', { source: 'node1', target: 'node2', hidden: false });
      
      // Initially: edge should be visible
      expect(visState.getGraphEdge('edge1')?.hidden).toBe(false);
      
      // INVARIANT TEST: Collapse one container - edge should become hidden
      visState.collapseContainer('container1');
      
      // After collapse: edge should be hidden (one endpoint hidden)
      expect(visState.getGraphEdge('edge1')?.hidden).toBe(true);
      
      // INVARIANT: visibleEdges should not include hidden edges
      const visibleEdgeIds = visState.visibleEdges.map(e => e.id);
      expect(visibleEdgeIds).not.toContain('edge1');
    });

    test('should handle internal edges within collapsed containers', () => {
      // Setup: Container with internal nodes and edge
      visState.setContainer('container1', {
        children: ['node1', 'node2'],
        collapsed: false,
        hidden: false
      });
      
      visState.setGraphNode('node1', { label: 'Node 1', hidden: false });
      visState.setGraphNode('node2', { label: 'Node 2', hidden: false });
      visState.setGraphEdge('internal_edge', { source: 'node1', target: 'node2', hidden: false });
      
      // Initially: edge should be visible
      expect(visState.getGraphEdge('internal_edge')?.hidden).toBe(false);
      
      // INVARIANT TEST: Collapse container - internal edge should become hidden
      visState.collapseContainer('container1');
      
      // After collapse: internal edge should be hidden
      expect(visState.getGraphEdge('internal_edge')?.hidden).toBe(true);
      
      // INVARIANT: visibleEdges should not include internal edges
      const visibleEdgeIds = visState.visibleEdges.map(e => e.id);
      expect(visibleEdgeIds).not.toContain('internal_edge');
    });
  });

  describe('Complete Visibility Invariants', () => {
    test('should never leak any hidden elements in visible* getters', () => {
      // Setup: Complex hierarchy
      visState.setContainer('root', {
        children: ['child_container', 'node_outside'],
        collapsed: false,
        hidden: false
      });
      
      visState.setContainer('child_container', {
        children: ['node_inside1', 'node_inside2'],
        collapsed: false,
        hidden: false
      });
      
      visState.setGraphNode('node_outside', { label: 'Outside', hidden: false });
      visState.setGraphNode('node_inside1', { label: 'Inside 1', hidden: false });
      visState.setGraphNode('node_inside2', { label: 'Inside 2', hidden: false });
      
      visState.setGraphEdge('edge_internal', { source: 'node_inside1', target: 'node_inside2', hidden: false });
      visState.setGraphEdge('edge_external', { source: 'node_outside', target: 'node_inside1', hidden: false });
      
      // INVARIANT TEST: Collapse child container
      visState.collapseContainer('child_container');
      
      // COMPREHENSIVE INVARIANT: No hidden elements should be visible
      const visibleNodes = visState.visibleNodes;
      const visibleEdges = visState.visibleEdges;
      const visibleContainers = visState.visibleContainers;
      
      // All visible elements must have hidden: false
      for (const node of visibleNodes) {
        expect(node.hidden).toBe(false);
      }
      
      for (const edge of visibleEdges) {
        expect(edge.hidden).toBe(false);
      }
      
      for (const container of visibleContainers) {
        expect(container.hidden).toBe(false);
      }
      
      // Specific checks
      expect(visibleNodes.find(n => n.id === 'node_inside1')).toBeUndefined();
      expect(visibleNodes.find(n => n.id === 'node_inside2')).toBeUndefined();
      expect(visibleEdges.find(e => e.id === 'edge_internal')).toBeUndefined();
      expect(visibleEdges.find(e => e.id === 'edge_external')).toBeUndefined();
      
      // node_outside should still be visible
      expect(visibleNodes.find(n => n.id === 'node_outside')).toBeDefined();
      
      // Containers: root and child_container should be visible (collapsed containers are visible)
      expect(visibleContainers.find(c => c.id === 'root')).toBeDefined();
      expect(visibleContainers.find(c => c.id === 'child_container')).toBeDefined();
    });
  });
});
