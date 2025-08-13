/**
 * @fileoverview Integration Tests: VisualizationState with Clean Bridges
 * 
 * These tests verify that the cleaned-up bridges work correctly with the new
 * VisualizationState business logic methods, demonstrating the complete
 * separation of concerns achieved by our refactoring.
 */

import { describe, test, expect, beforeEach } from 'vitest';
import { createVisualizationState } from '../VisualizationState';
import type { VisualizationState } from '../VisualizationState';

describe('VisualizationState + Clean Bridges Integration', () => {
  let visState: VisualizationState;

  beforeEach(() => {
    visState = createVisualizationState();
  });

  describe('Simulated ELKBridge Integration', () => {
    test('should provide clean data for ELK layout without any business logic in bridge', () => {
      // Setup: Complex graph that would have caused issues in old bridges
      visState.setGraphNode('top1', { label: 'Top 1', width: 180, height: 60, hidden: false });
      visState.setGraphNode('top2', { label: 'Top 2', hidden: false }); // Missing dimensions
      visState.setGraphNode('child1', { label: 'Child 1', width: 150, height: 50, hidden: false });
      visState.setGraphNode('child2', { label: 'Child 2', width: 160, height: 55, hidden: false });
      visState.setGraphNode('collapsed_child', { label: 'Collapsed Child', hidden: false });

      visState.setContainer('expanded_container', {
        collapsed: false,
        hidden: false,
        children: ['child1', 'child2'],
        width: 400,
        height: 300
      });

      visState.setContainer('collapsed_container', {
        collapsed: true,
        hidden: false,
        children: ['collapsed_child'],
        width: 200,
        height: 150
      });

      // CRITICAL: Collapse the container to hide its children
      visState.collapseContainer('collapsed_container');

      visState.setGraphEdge('edge1', {
        source: 'top1',
        target: 'child1',
        hidden: false
      });

      // CRITICAL: Ensure dimensions are valid (no longer done in bridge)
      visState.validateAndFixDimensions();

      // Simulate ELKBridge.extractVisibleNodes() - now clean of business logic
      const visibleNodes = visState.visibleNodes;
      const collapsedAsNodes = visState.getCollapsedContainersAsNodes();
      const allNodesForELK = [...visibleNodes, ...collapsedAsNodes];

      // Simulate ELKBridge.extractVisibleContainers() - now clean
      const expandedContainers = visState.getExpandedContainers();

      // Simulate ELKBridge.buildELKGraph() top-level node logic - now clean
      const topLevelNodes = visState.getTopLevelNodes();

      // Verify: All data is properly prepared by VisualizationState
      expect(allNodesForELK).toHaveLength(5); // 4 visible nodes + 1 collapsed container as node
      expect(expandedContainers).toHaveLength(1); // Only expanded container
      expect(topLevelNodes).toHaveLength(2); // top1, top2 (collapsed_child is now hidden)

      // Verify: Dimensions are guaranteed valid (no more fallbacks in bridge)
      for (const node of allNodesForELK) {
        expect(node.width).toBeGreaterThan(0);
        expect(node.height).toBeGreaterThan(0);
      }

      // Verify: Collapsed container properly converted to node
      const collapsedAsNode = allNodesForELK.find(n => n.id === 'collapsed_container');
      expect(collapsedAsNode).toBeDefined();
      expect(collapsedAsNode).toMatchObject({
        id: 'collapsed_container',
        width: 200,
        height: 150,
        style: 'default'
      });

      // Verify: No business logic leakage - bridge would only translate formats
      expect(topLevelNodes.find(n => n.id === 'top1')).toBeDefined();
      expect(topLevelNodes.find(n => n.id === 'top2')).toBeDefined();
      expect(topLevelNodes.find(n => n.id === 'child1')).toBeUndefined(); // Child of expanded ≠ top-level
      expect(topLevelNodes.find(n => n.id === 'collapsed_child')).toBeUndefined(); // Hidden due to collapsed parent
    });

    test('should prevent the original ELK dimension explosion bug', () => {
      // Setup: Scenario that caused the original bt_26 dimension explosion
      visState.setContainer('bt_26', {
        collapsed: true,
        hidden: false,
        children: Array.from({ length: 23 }, (_, i) => `node_${i}`), // 23 children like original
        width: 200,
        height: 150
      });

      // Add all the children nodes
      for (let i = 0; i < 23; i++) {
        visState.setGraphNode(`node_${i}`, { 
          label: `Node ${i}`, 
          width: 180, 
          height: 60, 
          hidden: false // Will be set to hidden when container is collapsed
        });
      }

      // CRITICAL: Actually collapse the container to trigger hiding logic
      visState.collapseContainer('bt_26');

      // Simulate what ELK would receive (old bridge would have leaked children)
      const expandedContainers = visState.getExpandedContainers();
      const collapsedAsNodes = visState.getCollapsedContainersAsNodes();

      // Verify: ELK sees no expanded containers (so no children to layout)
      expect(expandedContainers).toHaveLength(0);

      // Verify: ELK sees collapsed container as simple 200x150 node
      expect(collapsedAsNodes).toHaveLength(1);
      expect(collapsedAsNodes[0]).toMatchObject({
        id: 'bt_26',
        width: 200,
        height: 150
      });

      // Verify: ELK would NEVER see the 23 children (preventing dimension explosion)
      const visibleNodes = visState.visibleNodes;
      expect(visibleNodes).toHaveLength(0); // All children are hidden

      // This prevents the original bug where ELK tried to fit 23 nodes in a 200x150 container
    });
  });

  describe('Simulated ReactFlowBridge Integration', () => {
    test('should provide clean parent mapping without business logic in bridge', () => {
      // Setup: Complex parent-child relationships
      visState.setGraphNode('child_a', { label: 'Child A', hidden: false });
      visState.setGraphNode('child_b', { label: 'Child B', hidden: false });
      visState.setGraphNode('child_c', { label: 'Child C', hidden: false });
      visState.setGraphNode('orphan', { label: 'Orphan', hidden: false });

      visState.setContainer('parent_expanded', {
        collapsed: false,
        hidden: false,
        children: ['child_a', 'child_b']
      });

      visState.setContainer('parent_collapsed', {
        collapsed: true,
        hidden: false,
        children: ['child_c']
      });

      // Simulate ReactFlowBridge.buildParentMap() - now clean of business logic
      const parentMap = visState.getParentChildMap();

      // Verify: Only expanded container children are mapped
      expect(parentMap.get('child_a')).toBe('parent_expanded');
      expect(parentMap.get('child_b')).toBe('parent_expanded');
      expect(parentMap.get('child_c')).toBeUndefined(); // Parent is collapsed
      expect(parentMap.get('orphan')).toBeUndefined(); // No parent

      expect(parentMap.size).toBe(2);
    });

    test('should provide consistent edge handle information', () => {
      // Setup: Create nodes first
      visState.setGraphNode('node1', { x: 0, y: 0, hidden: false });
      visState.setGraphNode('node2', { x: 100, y: 100, hidden: false });
      visState.setGraphNode('node3', { x: 200, y: 200, hidden: false });
      visState.setGraphNode('node4', { x: 300, y: 300, hidden: false });
      
      // Setup: Various edge handle scenarios
      visState.setGraphEdge('edge_custom', {
        source: 'node1',
        target: 'node2',
        sourceHandle: 'out-port-1',
        targetHandle: 'in-port-2',
        hidden: false
      });

      visState.setGraphEdge('edge_partial', {
        source: 'node2',
        target: 'node3',
        sourceHandle: 'out-port-3',
        hidden: false
      });

      visState.setGraphEdge('edge_defaults', {
        source: 'node3',
        target: 'node4',
        hidden: false
      });

      // Simulate ReactFlowBridge edge handle assignment - now clean
      const handles1 = visState.getEdgeHandles('edge_custom');
      const handles2 = visState.getEdgeHandles('edge_partial');
      const handles3 = visState.getEdgeHandles('edge_defaults');

      // Verify: Consistent handle logic (no more hardcoded defaults in bridge)
      expect(handles1).toEqual({
        sourceHandle: 'out-port-1',
        targetHandle: 'in-port-2'
      });

      expect(handles2).toEqual({
        sourceHandle: 'out-port-3',
        targetHandle: 'default-in'
      });

      expect(handles3).toEqual({
        sourceHandle: 'default-out',
        targetHandle: 'default-in'
      });
    });
  });

  describe('Cross-Bridge Data Consistency', () => {
    test('should provide consistent view of the same graph to both bridges', () => {
      // Setup: Graph that both ELK and ReactFlow bridges would process
      visState.setGraphNode('shared_node', { label: 'Shared', width: 200, height: 100, hidden: false });
      visState.setGraphNode('external_node', { label: 'External', width: 180, height: 80, hidden: false });

      visState.setContainer('shared_container', {
        collapsed: false, // Start expanded
        hidden: false,
        children: ['shared_node'],
        width: 250,
        height: 150
      });

      // Create the edge BEFORE collapsing so it can be properly processed
      visState.setGraphEdge('cross_edge', {
        source: 'external_node',
        target: 'shared_node',
        sourceHandle: 'out-1',
        targetHandle: 'in-1',
        hidden: false
      });

      // CRITICAL: Actually collapse to trigger invariants
      visState.collapseContainer('shared_container');

      // ELK Bridge perspective
      const elkNodes = [...visState.visibleNodes, ...visState.getCollapsedContainersAsNodes()];
      const elkTopLevel = visState.getTopLevelNodes();
      const elkContainers = visState.getExpandedContainers();

      // ReactFlow Bridge perspective  
      const reactFlowParents = visState.getParentChildMap();
      const reactFlowHandles = visState.getEdgeHandles('cross_edge');

      // Verify: Both bridges see consistent data
      // ELK sees: collapsed container as node, shared_node is hidden (not top-level anymore)
      expect(elkNodes.find(n => n.id === 'shared_container')).toBeDefined();
      expect(elkTopLevel.find(n => n.id === 'shared_node')).toBeUndefined(); // Hidden due to collapse
      expect(elkContainers).toHaveLength(0); // No expanded containers

      // ReactFlow sees: no parent mapping for shared_node (parent collapsed), correct handles
      expect(reactFlowParents.get('shared_node')).toBeUndefined();
      expect(reactFlowHandles).toEqual({
        sourceHandle: 'out-1',
        targetHandle: 'in-1'
      });

      // Both see the same fundamental structure, formatted for their needs
      expect(elkNodes.find(n => n.id === 'external_node')).toBeDefined();
      expect(elkTopLevel.find(n => n.id === 'external_node')).toBeDefined();
      expect(reactFlowParents.get('external_node')).toBeUndefined();
    });
  });
});
