/**
 * @fileoverview Unit Tests for VisualizationState Bridge Support Methods
 * 
 * Tests the business logic methods that were moved from bridges to VisualizationState:
 * - getCollapsedContainersAsNodes()
 * - getParentChildMap() 
 * - getTopLevelNodes()
 * - getEdgeHandles()
 * - validateAndFixDimensions()
 */

import { describe, test, expect, beforeEach } from 'vitest';
import { createVisualizationState } from '../VisualizationState';
import type { VisualizationState } from '../VisualizationState';
import { ReactFlowBridge } from '../../bridges/ReactFlowBridge';
import { LAYOUT_CONSTANTS } from '../../shared/config';

describe('VisualizationState Bridge Support Methods', () => {
  let visState: VisualizationState;

  beforeEach(() => {
    visState = createVisualizationState();
  });

  describe('getCollapsedContainersAsNodes()', () => {
    test('should convert collapsed containers to nodes with proper dimensions', () => {
      // Setup: Create collapsed and expanded containers
      visState.setContainer('collapsed1', {
        collapsed: true,
        hidden: false,
        children: ['node1', 'node2'],
        width: 250,
        height: 180,
        x: 100,
        y: 200
      });
      
      visState.setContainer('expanded1', {
        collapsed: false,
        hidden: false,
        children: ['node3'],
        width: 300,
        height: 250
      });
      
      visState.setContainer('collapsed2', {
        collapsed: true,
        hidden: false,
        children: ['node4']
        // No explicit dimensions - should use defaults
      });

      // Execute
      const collapsedAsNodes = visState.getCollapsedContainersAsNodes();

      // Verify
      expect(collapsedAsNodes).toHaveLength(2);
      
      const collapsed1Node = collapsedAsNodes.find(n => n.id === 'collapsed1');
      expect(collapsed1Node).toBeDefined();
      expect(collapsed1Node).toMatchObject({
        id: 'collapsed1',
        label: 'collapsed1',
        width: 200,
        height: 150,
        x: 100,
        y: 200,
        hidden: false,
        style: 'default'
      });
      
      const collapsed2Node = collapsedAsNodes.find(n => n.id === 'collapsed2');
      expect(collapsed2Node).toBeDefined();
      expect(collapsed2Node).toMatchObject({
        id: 'collapsed2',
        label: 'collapsed2',
        width: LAYOUT_CONSTANTS.MIN_CONTAINER_WIDTH,
        height: LAYOUT_CONSTANTS.MIN_CONTAINER_HEIGHT,
        x: 0,
        y: 0,
        hidden: false,
        style: 'default'
      });
      
      // Should not include expanded containers
      expect(collapsedAsNodes.find(n => n.id === 'expanded1')).toBeUndefined();
    });

    test('should return empty array when no collapsed containers exist', () => {
      // Setup: Only expanded containers
      visState.setContainer('expanded1', {
        collapsed: false,
        hidden: false,
        children: ['node1']
      });

      // Execute
      const collapsedAsNodes = visState.getCollapsedContainersAsNodes();

      // Verify
      expect(collapsedAsNodes).toHaveLength(0);
    });

    test('should not include hidden collapsed containers', () => {
      // Setup: Hidden collapsed container
      visState.setContainer('hiddenCollapsed', {
        collapsed: true,
        hidden: true,
        children: ['node1']
      });

      // Execute
      const collapsedAsNodes = visState.getCollapsedContainersAsNodes();

      // Verify
      expect(collapsedAsNodes).toHaveLength(0);
    });
  });

  describe('getParentChildMap()', () => {
    test('should map children to their parent containers for expanded containers only', () => {
      // Setup: Mixed collapsed and expanded containers
      visState.setContainer('parent1', {
        collapsed: false,
        hidden: false,
        children: ['node1', 'node2', 'child_container']
      });
      
      visState.setContainer('parent2', {
        collapsed: true, // Collapsed - children should not appear in map
        hidden: false,
        children: ['node3', 'node4']
      });
      
      visState.setContainer('child_container', {
        collapsed: false,
        hidden: false,
        children: ['node5']
      });

      // Execute
      const parentMap = visState.getParentChildMap();

      // Verify
      expect(parentMap.get('node1')).toBe('parent1');
      expect(parentMap.get('node2')).toBe('parent1');
      expect(parentMap.get('child_container')).toBe('parent1');
      expect(parentMap.get('node5')).toBe('child_container');
      
      // Children of collapsed containers should not be in the map
      expect(parentMap.get('node3')).toBeUndefined();
      expect(parentMap.get('node4')).toBeUndefined();
      
      expect(parentMap.size).toBe(4);
    });

    test('should return empty map when no expanded containers exist', () => {
      // Setup: Only collapsed containers
      visState.setContainer('collapsed1', {
        collapsed: true,
        hidden: false,
        children: ['node1', 'node2']
      });

      // Execute
      const parentMap = visState.getParentChildMap();

      // Verify
      expect(parentMap.size).toBe(0);
    });

    test('should handle empty containers', () => {
      // Setup: Expanded container with no children
      visState.setContainer('empty', {
        collapsed: false,
        hidden: false,
        children: []
      });

      // Execute
      const parentMap = visState.getParentChildMap();

      // Verify
      expect(parentMap.size).toBe(0);
    });
  });

  describe('getTopLevelNodes()', () => {
    test('should return nodes that are not in any expanded container', () => {
      // Setup: Complex hierarchy
      visState.setGraphNode('topLevel1', { label: 'Top Level 1', hidden: false });
      visState.setGraphNode('topLevel2', { label: 'Top Level 2', hidden: false });
      visState.setGraphNode('insideExpanded', { label: 'Inside Expanded', hidden: false });
      visState.setGraphNode('insideCollapsed', { label: 'Inside Collapsed', hidden: false });
      visState.setGraphNode('hidden', { label: 'Hidden Node', hidden: true });
      
      visState.setContainer('expanded', {
        collapsed: false,
        hidden: false,
        children: ['insideExpanded']
      });
      
      visState.setContainer('collapsed', {
        collapsed: true,
        hidden: false,
        children: ['insideCollapsed']
      });

      // Execute
      const topLevelNodes = visState.getTopLevelNodes();

      // Verify
      const topLevelIds = topLevelNodes.map(n => n.id);
      expect(topLevelIds).toContain('topLevel1');
      expect(topLevelIds).toContain('topLevel2');
      expect(topLevelIds).not.toContain('insideCollapsed'); // Nodes in collapsed containers are hidden
      
      expect(topLevelIds).not.toContain('insideExpanded'); // In expanded container
      expect(topLevelIds).not.toContain('hidden'); // Hidden nodes are filtered by visibleNodes
      expect(topLevelIds).not.toContain('expanded'); // Containers are not nodes
      expect(topLevelIds).not.toContain('collapsed'); // Collapsed containers are handled separately
    });

    test('should return all visible nodes when no containers exist', () => {
      // Setup: Only nodes, no containers
      visState.setGraphNode('node1', { label: 'Node 1', hidden: false });
      visState.setGraphNode('node2', { label: 'Node 2', hidden: false });
      visState.setGraphNode('hidden', { label: 'Hidden', hidden: true });

      // Execute
      const topLevelNodes = visState.getTopLevelNodes();

      // Verify
      const topLevelIds = topLevelNodes.map(n => n.id);
      expect(topLevelIds).toContain('node1');
      expect(topLevelIds).toContain('node2');
      expect(topLevelIds).not.toContain('hidden');
      expect(topLevelNodes).toHaveLength(2);
    });
  });

  describe('getEdgeHandles()', () => {
    test('should return custom handles when specified', () => {
      // Setup: Create nodes first
      visState.setGraphNode('node1', { x: 0, y: 0, hidden: false });
      visState.setGraphNode('node2', { x: 100, y: 100, hidden: false });
      
      // Setup: Edge with custom handles
      visState.setGraphEdge('edge1', {
        source: 'node1',
        target: 'node2',
        sourceHandle: 'custom-out',
        targetHandle: 'custom-in',
        hidden: false
      });

      // Execute
      const reactFlowBridge = new ReactFlowBridge();
      const handles = reactFlowBridge.getEdgeHandles(visState, 'edge1');

      // Verify
      expect(handles).toEqual({
        sourceHandle: 'custom-out',
        targetHandle: 'custom-in'
      });
    });

    test('should return default handles when not specified', () => {
      // Setup: Create nodes first
      visState.setGraphNode('node1', { x: 0, y: 0, hidden: false });
      visState.setGraphNode('node2', { x: 100, y: 100, hidden: false });
      
      // Setup: Edge without custom handles
      visState.setGraphEdge('edge2', {
        source: 'node1',
        target: 'node2',
        hidden: false
      });

      // Execute
      const reactFlowBridge2 = new ReactFlowBridge();
      const handles = reactFlowBridge2.getEdgeHandles(visState, 'edge2');

      // Verify
      expect(handles).toEqual({
        sourceHandle: 'out-bottom', // Current system uses discrete handles
        targetHandle: 'in-top'
      });
    });

    test('should return empty object for non-existent edge', () => {
      // Execute
      const reactFlowBridge3 = new ReactFlowBridge();
      const handles = reactFlowBridge3.getEdgeHandles(visState, 'nonexistent');

      // Verify
      expect(handles).toEqual({});
    });

    test('should use defaults for partial handle specification', () => {
      // Setup: Create nodes first
      visState.setGraphNode('node1', { x: 0, y: 0, hidden: false });
      visState.setGraphNode('node2', { x: 100, y: 100, hidden: false });
      
      // Setup: Edge with only source handle specified
      visState.setGraphEdge('edge3', {
        source: 'node1',
        target: 'node2',
        sourceHandle: 'custom-out',
        hidden: false
      });

      // Execute
      const reactFlowBridge4 = new ReactFlowBridge();
      const handles = reactFlowBridge4.getEdgeHandles(visState, 'edge3');

      // Verify
      expect(handles).toEqual({
        sourceHandle: 'custom-out',
        targetHandle: 'in-top' // Current system uses discrete handles for defaults
      });
    });
  });

  describe('validateAndFixDimensions()', () => {
    test('should fix invalid node dimensions', () => {
      // Setup: Nodes with invalid dimensions
      visState.setGraphNode('validNode', { 
        label: 'Valid', 
        hidden: false,
        width: 100,
        height: 50
      });
      
      visState.setGraphNode('invalidNode', { 
        label: 'Invalid', 
        hidden: false,
        width: 0,
        height: -10
      });
      
      visState.setGraphNode('missingDims', { 
        label: 'Missing', 
        hidden: false
        // No width/height specified
      });

      // Execute
      visState.validateAndFixDimensions();

      // Verify
      const validNode = visState.getGraphNode('validNode');
      expect(validNode?.width).toBe(100);
      expect(validNode?.height).toBe(50);
      
      const invalidNode = visState.getGraphNode('invalidNode');
      expect(invalidNode?.width).toBe(LAYOUT_CONSTANTS.DEFAULT_NODE_WIDTH);
      expect(invalidNode?.height).toBe(LAYOUT_CONSTANTS.DEFAULT_NODE_HEIGHT);
      
      const missingNode = visState.getGraphNode('missingDims');
      expect(missingNode?.width).toBe(LAYOUT_CONSTANTS.DEFAULT_NODE_WIDTH);
      expect(missingNode?.height).toBe(LAYOUT_CONSTANTS.DEFAULT_NODE_HEIGHT);
    });

    test('should fix invalid container dimensions', () => {
      // Setup: Containers with invalid dimensions
      visState.setContainer('validContainer', {
        collapsed: false,
        hidden: false,
        children: [],
        width: 300,
        height: 200
      });
      
      visState.setContainer('invalidContainer', {
        collapsed: false,
        hidden: false,
        children: [],
        width: 0,
        height: -5
      });

      // Execute
      visState.validateAndFixDimensions();

      // Verify
      const validContainer = visState.getContainer('validContainer');
      expect(validContainer?.width).toBe(LAYOUT_CONSTANTS.MIN_CONTAINER_WIDTH); // External dimensions ignored due to encapsulation
      expect(validContainer?.height).toBe(LAYOUT_CONSTANTS.MIN_CONTAINER_HEIGHT);
      
      const invalidContainer = visState.getContainer('invalidContainer');
      expect(invalidContainer?.width).toBe(LAYOUT_CONSTANTS.MIN_CONTAINER_WIDTH);
      expect(invalidContainer?.height).toBe(LAYOUT_CONSTANTS.MIN_CONTAINER_HEIGHT);
    });

    test('should handle empty state gracefully', () => {
      // Execute on empty state
      expect(() => visState.validateAndFixDimensions()).not.toThrow();
    });
  });

  describe('Integration: Bridge Support Methods Working Together', () => {
    test('should provide consistent data for ELK Bridge workflow', () => {
      // Setup: Complex graph structure
      visState.setGraphNode('topNode', { label: 'Top Node', width: 180, height: 60, hidden: false });
      visState.setGraphNode('childNode1', { label: 'Child 1', width: 150, height: 50, hidden: false });
      visState.setGraphNode('childNode2', { label: 'Child 2', width: 160, height: 55, hidden: false });
      visState.setGraphNode('collapsedChild', { label: 'Collapsed Child', hidden: false });
      
      visState.setContainer('expandedContainer', {
        collapsed: false,
        hidden: false,
        children: ['childNode1', 'childNode2'],
        width: 400,
        height: 300
      });
      
      visState.setContainer('collapsedContainer', {
        collapsed: true,
        hidden: false,
        children: ['collapsedChild'],
        width: 200,
        height: 150
      });
      
      visState.setGraphEdge('edge1', {
        source: 'topNode',
        target: 'childNode1',
        sourceHandle: 'out-1',
        hidden: false
      });

      // Ensure dimensions are valid
      visState.validateAndFixDimensions();

      // Execute: Get all bridge support data
      const collapsedAsNodes = visState.getCollapsedContainersAsNodes();
      const parentMap = visState.getParentChildMap();
      const topLevelNodes = visState.getTopLevelNodes();
      const reactFlowBridge5 = new ReactFlowBridge();
      const edgeHandles = reactFlowBridge5.getEdgeHandles(visState, 'edge1');

      // Verify: Consistent bridge data
      expect(collapsedAsNodes).toHaveLength(1);
      expect(collapsedAsNodes[0].id).toBe('collapsedContainer');
      expect(collapsedAsNodes[0].width).toBe(200);
      expect(collapsedAsNodes[0].height).toBe(150);
      
      expect(parentMap.get('childNode1')).toBe('expandedContainer');
      expect(parentMap.get('childNode2')).toBe('expandedContainer');
      expect(parentMap.get('collapsedChild')).toBeUndefined(); // In collapsed container
      
      const topLevelIds = topLevelNodes.map(n => n.id);
      expect(topLevelIds).toContain('topNode');
      expect(topLevelIds).not.toContain('collapsedChild'); // Child of collapsed container is hidden
      expect(topLevelIds).not.toContain('childNode1'); // In expanded container
      expect(topLevelIds).not.toContain('childNode2'); // In expanded container
      
      expect(edgeHandles).toEqual({
        sourceHandle: 'out-1',
        targetHandle: 'in-top' // Current system uses discrete handles for defaults
      });

      // Verify: All nodes have valid dimensions
      const collapsedChildNode = visState.getGraphNode('collapsedChild');
      expect(collapsedChildNode?.width).toBe(LAYOUT_CONSTANTS.DEFAULT_NODE_WIDTH);
      expect(collapsedChildNode?.height).toBe(LAYOUT_CONSTANTS.DEFAULT_NODE_HEIGHT);
    });
  });
});
