/**
 * @fileoverview Bridge Migration Validation Tests
 * 
 * These tests verify that the business logic moved from bridges to VisualizationState
 * produces the same results as the original bridge implementations would have.
 * This ensures our refactoring maintains behavioral compatibility.
 */

import { describe, test, expect, beforeEach } from 'vitest';
import { createVisualizationState } from '../VisualizationState';
import type { VisualizationState } from '../VisualizationState';
import { LAYOUT_CONSTANTS } from '../../shared/config';

describe('Bridge Migration Validation', () => {
  let visState: VisualizationState;

  beforeEach(() => {
    visState = createVisualizationState();
  });

  describe('ELKBridge Migration: Collapsed Container as Node Conversion', () => {
    test('should match original ELKBridge collapsed container conversion logic', () => {
      // Setup: Replicate the scenario from original ELKBridge.extractVisibleNodes()
      visState.setContainer('bt_26', {
        collapsed: true,
        hidden: false,
        children: ['node1', 'node2', 'node3'],
        width: 200,
        height: 150,
        x: 100,
        y: 200
      });

      // Execute: Use new VisualizationState method
      const collapsedAsNodes = visState.getCollapsedContainersAsNodes();

      // Verify: Should match original ELKBridge logic
      expect(collapsedAsNodes).toHaveLength(1);
      const containerAsNode = collapsedAsNodes[0];
      
      // This should match the original ELKBridge containerAsNode creation
      expect(containerAsNode).toMatchObject({
        id: 'bt_26',
        width: 200,  // Should use container dimensions, not defaults
        height: 150,
        x: 100,
        y: 200,
        hidden: false,
        collapsed: true,
        type: 'container-node'
      });
    });

    test('should handle containers without explicit dimensions (using defaults)', () => {
      // Setup: Container without width/height (would have used fallbacks in original)
      visState.setContainer('container_no_dims', {
        collapsed: true,
        hidden: false,
        children: ['node1']
        // No width/height specified
      });

      // Execute
      const collapsedAsNodes = visState.getCollapsedContainersAsNodes();

      // Verify: Should use LAYOUT_CONSTANTS instead of hardcoded bridge defaults
      expect(collapsedAsNodes).toHaveLength(1);
      expect(collapsedAsNodes[0]).toMatchObject({
        id: 'container_no_dims',
        width: LAYOUT_CONSTANTS.MIN_CONTAINER_WIDTH,
        height: LAYOUT_CONSTANTS.MIN_CONTAINER_HEIGHT,
        collapsed: true,
        type: 'container-node'
      });
    });
  });

  describe('ELKBridge Migration: Top Level Node Filtering', () => {
    test('should match original ELKBridge top-level node identification', () => {
      // Setup: Replicate complex hierarchy from original ELKBridge.buildELKGraph()
      visState.setGraphNode('topLevel1', { label: 'Top 1', width: 180, height: 60, hidden: false });
      visState.setGraphNode('topLevel2', { label: 'Top 2', width: 160, height: 50, hidden: false });
      visState.setGraphNode('inExpanded', { label: 'In Expanded', width: 150, height: 45, hidden: false });
      visState.setGraphNode('inCollapsed', { label: 'In Collapsed', width: 140, height: 40, hidden: false });
      
      visState.setContainer('expandedContainer', {
        collapsed: false,
        hidden: false,
        children: ['inExpanded']
      });
      
      visState.setContainer('collapsedContainer', {
        collapsed: true,
        hidden: false,
        children: ['inCollapsed']
      });

      // Execute: Get top-level nodes like original ELKBridge would
      const topLevelNodes = visState.getTopLevelNodes();
      const collapsedContainerIds = new Set(
        visState.visibleContainers.filter(c => c.collapsed).map(c => c.id)
      );

      // Verify: Should match original ELKBridge filtering logic
      const topLevelIds = topLevelNodes.map(n => n.id);
      
      // With our fix: nodes in collapsed containers are hidden, so only truly top-level nodes appear
      expect(topLevelIds).toContain('topLevel1'); // Not in any container
      expect(topLevelIds).toContain('topLevel2'); // Not in any container
      expect(topLevelIds).not.toContain('inCollapsed'); // In collapsed container, so hidden (not top-level)
      expect(topLevelIds).not.toContain('inExpanded'); // In expanded container
      
      // Verify no collapsed containers themselves are included
      expect(topLevelIds).not.toContain('collapsedContainer');
      expect(topLevelIds).not.toContain('expandedContainer');
    });

    test('should enforce dimension validation like improved ELKBridge', () => {
      // Setup: Node without dimensions (would have thrown error in improved bridge)
      visState.setGraphNode('nodeMissingDims', { 
        label: 'Missing Dims', 
        hidden: false
        // No width/height
      });

      // First: Validate dimensions
      visState.validateAndFixDimensions();

      // Execute: Get top-level nodes
      const topLevelNodes = visState.getTopLevelNodes();

      // Verify: Should have valid dimensions (no longer throws error)
      const node = topLevelNodes.find(n => n.id === 'nodeMissingDims');
      expect(node).toBeDefined();
      expect(node?.width).toBe(LAYOUT_CONSTANTS.DEFAULT_NODE_WIDTH);
      expect(node?.height).toBe(LAYOUT_CONSTANTS.DEFAULT_NODE_HEIGHT);
    });
  });

  describe('ReactFlowBridge Migration: Parent-Child Mapping', () => {
    test('should match original ReactFlowBridge.buildParentMap() logic', () => {
      // Setup: Replicate original ReactFlowBridge parent mapping scenario
      visState.setContainer('parent1', {
        collapsed: false, // Only expanded containers in original logic
        hidden: false,
        children: ['child1', 'child2', 'nested_container']
      });
      
      visState.setContainer('parent2', {
        collapsed: true, // Original logic excluded collapsed containers
        hidden: false,
        children: ['child3', 'child4']
      });
      
      visState.setContainer('nested_container', {
        collapsed: false,
        hidden: false,
        children: ['nested_child']
      });

      // Execute: Use new VisualizationState method
      const parentMap = visState.getParentChildMap();

      // Verify: Should match original ReactFlowBridge logic
      // Original: if (!container.collapsed) { container.children.forEach(childId => parentMap.set(childId, container.id)); }
      expect(parentMap.get('child1')).toBe('parent1');
      expect(parentMap.get('child2')).toBe('parent1');
      expect(parentMap.get('nested_container')).toBe('parent1');
      expect(parentMap.get('nested_child')).toBe('nested_container');
      
      // Children of collapsed containers should not be mapped (original behavior)
      expect(parentMap.get('child3')).toBeUndefined();
      expect(parentMap.get('child4')).toBeUndefined();
      
      expect(parentMap.size).toBe(4);
    });
  });

  describe('ReactFlowBridge Migration: Edge Handle Logic', () => {
    test('should match original ReactFlowBridge edge handle assignment', () => {
      // Setup: Create nodes first, then edges with various handle configurations
      visState.setGraphNode('node1', { label: 'Node 1' });
      visState.setGraphNode('node2', { label: 'Node 2' });
      visState.setGraphNode('node3', { label: 'Node 3' });
      
      visState.setGraphEdge('edge_with_handles', {
        source: 'node1',
        target: 'node2',
        sourceHandle: 'custom-out',
        targetHandle: 'custom-in',
        hidden: false
      });
      
      visState.setGraphEdge('edge_partial_handles', {
        source: 'node1',
        target: 'node3',
        sourceHandle: 'custom-out',
        // No targetHandle specified
        hidden: false
      });
      
      visState.setGraphEdge('edge_no_handles', {
        source: 'node2',
        target: 'node3',
        // No handles specified
        hidden: false
      });

      // Execute: Get handles like original ReactFlowBridge
      const handles1 = visState.getEdgeHandles('edge_with_handles');
      const handles2 = visState.getEdgeHandles('edge_partial_handles');
      const handles3 = visState.getEdgeHandles('edge_no_handles');

      // Verify: Should match original ReactFlowBridge logic
      // Original: edge.sourceHandle || 'default-out', edge.targetHandle || 'default-in'
      expect(handles1).toEqual({
        sourceHandle: 'custom-out',
        targetHandle: 'custom-in'
      });
      
      expect(handles2).toEqual({
        sourceHandle: 'custom-out',
        targetHandle: 'default-in'  // Original fallback
      });
      
      expect(handles3).toEqual({
        sourceHandle: 'default-out', // Original fallback
        targetHandle: 'default-in'   // Original fallback
      });
    });
  });

  describe('Cross-Bridge Consistency', () => {
    test('should provide consistent data between ELK and ReactFlow bridge needs', () => {
      // Setup: Complex scenario that both bridges would handle
      visState.setGraphNode('external', { label: 'External' });
      visState.setGraphNode('sharedNode', { label: 'Shared', width: 200, height: 100, hidden: false });
      
      // Create the edge first, before collapsing the container
      visState.setGraphEdge('sharedEdge', {
        source: 'external',
        target: 'sharedNode',
        sourceHandle: 'out-port',
        hidden: false
      });
      
      // Then set up the container - initially not collapsed
      visState.setContainer('sharedContainer', {
        collapsed: false,  // Start expanded so edge creation is valid
        hidden: false,
        children: ['sharedNode'],
        width: 250,
        height: 200,
        x: 50,
        y: 75
      });
      
      // Now collapse the container
      visState.collapseContainer('sharedContainer');

      // Execute: Get data as both bridges would
      const collapsedAsNodes = visState.getCollapsedContainersAsNodes(); // ELK Bridge
      const parentMap = visState.getParentChildMap(); // ReactFlow Bridge
      const topLevelNodes = visState.getTopLevelNodes(); // ELK Bridge
      const edgeHandles = visState.getEdgeHandles('sharedEdge'); // ReactFlow Bridge

      // Verify: Consistent view of the same data
      // ELK would see collapsed container as a node
      expect(collapsedAsNodes.find(n => n.id === 'sharedContainer')).toBeDefined();
      
      // ReactFlow would not map children of collapsed containers
      expect(parentMap.get('sharedNode')).toBeUndefined();
      
      // With our fix: sharedNode is hidden (in collapsed container), so not in top-level
      expect(topLevelNodes.find(n => n.id === 'sharedNode')).toBeUndefined();
      
      // ReactFlow would get consistent handle information
      expect(edgeHandles.sourceHandle).toBe('out-port');
      expect(edgeHandles.targetHandle).toBe('default-in');
    });
  });
});
