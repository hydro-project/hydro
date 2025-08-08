/**
 * @fileoverview VisState Hidden Children Leak Prevention Tests
 * 
 * Critical invariant tests to ensure VisState never leaks hidden children
 * of collapsed containers to ELKBridge or other consumers.
 */

import { describe, test, expect, beforeEach } from 'vitest';
import { VisualizationState } from '../VisState';

describe('VisState Hidden Children Leak Prevention', () => {
  let visState: VisualizationState;

  beforeEach(() => {
    visState = new VisualizationState();
  });

  test('should not leak nodes inside collapsed containers', () => {
    // Create a container with nodes
    visState.setContainer('parent', {
      children: ['node1', 'node2'],
      collapsed: false,
      hidden: false
    });
    
    visState.setGraphNode('node1', { label: 'Node 1', hidden: false });
    visState.setGraphNode('node2', { label: 'Node 2', hidden: false });
    
    // Initially, all should be visible
    expect(visState.visibleNodes.map(n => n.id)).toContain('node1');
    expect(visState.visibleNodes.map(n => n.id)).toContain('node2');
    expect(visState.visibleContainers.map(c => c.id)).toContain('parent');
    
    // Collapse the parent container
    visState.setContainer('parent', {
      children: ['node1', 'node2'],
      collapsed: true,
      hidden: false
    });
    
    // CRITICAL: Children nodes should be hidden from visibleNodes
    const visibleNodeIds = visState.visibleNodes.map(n => n.id);
    expect(visibleNodeIds).not.toContain('node1');
    expect(visibleNodeIds).not.toContain('node2');
    
    // Parent should still be visible
    expect(visState.visibleContainers.map(c => c.id)).toContain('parent');
  });

  test('should not leak nested containers inside collapsed containers', () => {
    // Create nested hierarchy: grandparent -> parent -> child
    visState.setContainer('grandparent', {
      children: ['parent'],
      collapsed: false,
      hidden: false
    });
    
    visState.setContainer('parent', {
      children: ['child'],
      collapsed: false,
      hidden: false
    });
    
    visState.setContainer('child', {
      children: ['node1'],
      collapsed: false,
      hidden: false
    });
    
    visState.setGraphNode('node1', { label: 'Node 1', hidden: false });
    
    // Initially, all should be visible
    expect(visState.visibleContainers.map(c => c.id)).toContain('grandparent');
    expect(visState.visibleContainers.map(c => c.id)).toContain('parent');
    expect(visState.visibleContainers.map(c => c.id)).toContain('child');
    expect(visState.visibleNodes.map(n => n.id)).toContain('node1');
    
    // Collapse the grandparent
    visState.setContainer('grandparent', {
      children: ['parent'],
      collapsed: true,
      hidden: false
    });
    
    // CRITICAL: All descendants should be hidden
    const visibleContainerIds = visState.visibleContainers.map(c => c.id);
    const visibleNodeIds = visState.visibleNodes.map(n => n.id);
    
    expect(visibleContainerIds).toContain('grandparent'); // Collapsed container itself is visible
    expect(visibleContainerIds).not.toContain('parent');  // Child containers hidden
    expect(visibleContainerIds).not.toContain('child');   // Nested child containers hidden
    expect(visibleNodeIds).not.toContain('node1');        // Nested nodes hidden
  });

  test('should not leak edges connecting hidden nodes', () => {
    // Create containers with nodes and edges
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
    
    visState.setGraphEdge('edge1', {
      source: 'node1',
      target: 'node2',
      hidden: false
    });
    
    // Initially, edge should be visible
    expect(visState.visibleEdges.map(e => e.id)).toContain('edge1');
    
    // Collapse container1 (hiding node1)
    visState.setContainer('container1', {
      children: ['node1'],
      collapsed: true,
      hidden: false
    });
    
    // CRITICAL: Edge should be hidden because node1 is now hidden
    const visibleEdgeIds = visState.visibleEdges.map(e => e.id);
    expect(visibleEdgeIds).not.toContain('edge1');
  });

  test('should handle complex collapse scenarios without leaks', () => {
    // Complex hierarchy with multiple levels and cross-container edges
    visState.setContainer('root', {
      children: ['container1', 'container2'],
      collapsed: false,
      hidden: false
    });
    
    visState.setContainer('container1', {
      children: ['node1', 'subcontainer1'],
      collapsed: false,
      hidden: false
    });
    
    visState.setContainer('container2', {
      children: ['node2'],
      collapsed: false,
      hidden: false
    });
    
    visState.setContainer('subcontainer1', {
      children: ['node3'],
      collapsed: false,
      hidden: false
    });
    
    visState.setGraphNode('node1', { label: 'Node 1', hidden: false });
    visState.setGraphNode('node2', { label: 'Node 2', hidden: false });
    visState.setGraphNode('node3', { label: 'Node 3', hidden: false });
    
    visState.setGraphEdge('edge1', { source: 'node1', target: 'node2', hidden: false });
    visState.setGraphEdge('edge2', { source: 'node1', target: 'node3', hidden: false });
    visState.setGraphEdge('edge3', { source: 'node2', target: 'node3', hidden: false });
    
    // Collapse the root - everything should be hidden except root itself
    visState.setContainer('root', {
      children: ['container1', 'container2'],
      collapsed: true,
      hidden: false
    });
    
    // Validate no leaks
    const visibleContainerIds = visState.visibleContainers.map(c => c.id);
    const visibleNodeIds = visState.visibleNodes.map(n => n.id);
    const visibleEdgeIds = visState.visibleEdges.map(e => e.id);
    
    // Only root should be visible
    expect(visibleContainerIds).toEqual(['root']);
    expect(visibleNodeIds).toEqual([]);
    expect(visibleEdgeIds).toEqual([]);
  });

  test('invariant: ELKBridge input should never have references to hidden entities', () => {
    // This test simulates what ELKBridge receives and validates no dangling references
    
    visState.setContainer('parent', {
      children: ['child1', 'child2'],
      collapsed: false,
      hidden: false
    });
    
    visState.setContainer('child1', {
      children: ['node1'],
      collapsed: false,
      hidden: false
    });
    
    visState.setContainer('child2', {
      children: ['node2'],
      collapsed: false,
      hidden: false
    });
    
    visState.setGraphNode('node1', { label: 'Node 1', hidden: false });
    visState.setGraphNode('node2', { label: 'Node 2', hidden: false });
    visState.setGraphEdge('edge1', { source: 'node1', target: 'node2', hidden: false });
    
    // Collapse parent
    visState.setContainer('parent', {
      children: ['child1', 'child2'],
      collapsed: true,
      hidden: false
    });
    
    // Get what ELKBridge would see
    const visibleContainers = visState.visibleContainers;
    const visibleNodes = visState.visibleNodes;
    const visibleEdges = visState.visibleEdges;
    
    // Validate no hidden entity references
    const allVisibleIds = new Set([
      ...visibleContainers.map(c => c.id),
      ...visibleNodes.map(n => n.id),
    ]);
    
    // Check edges don't reference hidden nodes
    for (const edge of visibleEdges) {
      expect(allVisibleIds.has(edge.source), 
        `Edge ${edge.id} references hidden source: ${edge.source}`).toBe(true);
      expect(allVisibleIds.has(edge.target), 
        `Edge ${edge.id} references hidden target: ${edge.target}`).toBe(true);
    }
    
    // Check containers don't have children that aren't visible
    for (const container of visibleContainers) {
      if (!container.collapsed) {
        for (const childId of container.children) {
          expect(allVisibleIds.has(childId), 
            `Container ${container.id} references hidden child: ${childId}`).toBe(true);
        }
      }
    }
  });
});
