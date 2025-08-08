/**
 * @fileoverview ELK Dimension Explosion Bug Prevention - Regression Tests
 * 
 * This test suite ensures that the ELK dimension explosion bug that affected
 * paxos-flipped.json never happens again. It specifically tests that:
 * 
 * 1. Containers created with collapsed=true automatically hide their children
 * 2. visibleNodes never contains children of collapsed containers
 * 3. ELK Bridge receives clean data with no dimension explosion risk
 * 
 * Historical Context: The original bug caused ELK to try to layout thousands
 * of hidden nodes inside small collapsed containers, creating massive spacing.
 */

import { describe, test, expect, beforeEach } from 'vitest';
import { createVisualizationState } from '../VisState';
import type { VisualizationState } from '../VisState';

describe('ELK Dimension Explosion Bug Prevention (Regression Tests)', () => {
  let visState: VisualizationState;

  beforeEach(() => {
    visState = createVisualizationState();
  });

  describe('Core Bug Prevention', () => {
    test('should automatically hide children when containers are created with collapsed=true', () => {
      // Setup: Recreate the bt_26 scenario that caused dimension explosion
      
      // Add 23 children nodes first
      const childIds = [];
      for (let i = 0; i < 23; i++) {
        const nodeId = `node_bt26_${i}`;
        childIds.push(nodeId);
        
        visState.setGraphNode(nodeId, {
          label: `BT26 Child ${i}`,
          width: 180,
          height: 60,
          hidden: false // Initially visible
        });
      }

      // Create a collapsed container with ALL the children
      // CRITICAL: This should automatically hide the children
      visState.setContainer('bt_26', {
        collapsed: true,
        hidden: false,
        children: childIds,
        width: 200,
        height: 150
      });

      // Verify: No children of collapsed containers should be visible
      const visibleNodes = visState.visibleNodes;
      const visibleNodeIds = visibleNodes.map(n => n.id);
      
      for (const childId of childIds) {
        expect(visibleNodeIds).not.toContain(childId);
      }
      
      expect(visibleNodes).toHaveLength(0); // No children should be visible

      // Verify: ELK sees clean data
      const expandedContainers = visState.expandedContainers;
      const collapsedAsNodes = visState.getCollapsedContainersAsNodes();
      
      expect(expandedContainers).toHaveLength(0); // bt_26 is collapsed, so no expanded containers
      expect(collapsedAsNodes).toHaveLength(1);
      expect(collapsedAsNodes[0]).toMatchObject({
        id: 'bt_26',
        width: 200,
        height: 150
      });

      // ELK should see: 1 simple node (bt_26) with NO children
      const elkNodes = [...visibleNodes, ...collapsedAsNodes];
      expect(elkNodes).toHaveLength(1); // Only the collapsed container as a node
      expect(expandedContainers).toHaveLength(0); // No container hierarchies
    });

    test('should prevent dimension explosion with nested collapsed containers', () => {
      // Setup: Even more complex scenario - nested collapsed containers
      
      // Add all nodes first
      const outerNodes = [];
      const inner1Nodes = [];
      const inner2Nodes = [];
      
      for (let i = 0; i < 15; i++) {
        const outerNodeId = `outer_node_${i}`;
        const inner1NodeId = `inner1_node_${i}`;
        const inner2NodeId = `inner2_node_${i}`;
        
        outerNodes.push(outerNodeId);
        inner1Nodes.push(inner1NodeId);
        inner2Nodes.push(inner2NodeId);
        
        visState.setGraphNode(outerNodeId, {
          label: `Outer Node ${i}`,
          hidden: false
        });
        
        visState.setGraphNode(inner1NodeId, {
          label: `Inner1 Node ${i}`,
          hidden: false
        });
        
        visState.setGraphNode(inner2NodeId, {
          label: `Inner2 Node ${i}`,
          hidden: false
        });
      }

      // Create containers with their children
      visState.setContainer('inner_container_1', {
        collapsed: true,
        hidden: false,
        children: inner1Nodes,
        width: 250,
        height: 150
      });

      visState.setContainer('inner_container_2', {
        collapsed: true,
        hidden: false,
        children: inner2Nodes,
        width: 250,
        height: 150
      });

      visState.setContainer('outer_container', {
        collapsed: true,
        hidden: false,
        children: [...outerNodes, 'inner_container_1', 'inner_container_2'],
        width: 300,
        height: 200
      });

      // Verify: ELK should only see the outer container, no hierarchy
      const visibleNodes = visState.visibleNodes;
      const expandedContainers = visState.expandedContainers;
      const collapsedAsNodes = visState.getCollapsedContainersAsNodes();

      expect(visibleNodes).toHaveLength(0); // All 45 nodes are hidden
      expect(expandedContainers).toHaveLength(0); // No expanded containers
      
      // Only the outer container should be visible as a collapsed node
      // Inner containers are hidden because their parent is collapsed
      expect(collapsedAsNodes).toHaveLength(1); // Only outer_container
      expect(collapsedAsNodes[0].id).toBe('outer_container');

      // ELK sees 1 simple node instead of trying to layout 45+ nodes in nested hierarchies
      const elkInput = [...visibleNodes, ...collapsedAsNodes];
      expect(elkInput).toHaveLength(1);
      expect(elkInput.every(node => node.width > 0 && node.height > 0)).toBe(true);
    });

    test('should handle mixed expanded and collapsed containers correctly', () => {
      // Setup: Mix of expanded and collapsed containers
      
      // Add nodes first
      const expandedNodes = [];
      const collapsedNodes = [];
      
      for (let i = 0; i < 10; i++) {
        const expandedNodeId = `expanded_node_${i}`;
        const collapsedNodeId = `collapsed_node_${i}`;
        
        expandedNodes.push(expandedNodeId);
        collapsedNodes.push(collapsedNodeId);
        
        visState.setGraphNode(expandedNodeId, {
          label: `Expanded Node ${i}`,
          hidden: false
        });
        
        visState.setGraphNode(collapsedNodeId, {
          label: `Collapsed Node ${i}`,
          hidden: false
        });
      }

      // Create collapsed child container first
      visState.setContainer('collapsed_child', {
        collapsed: true,
        hidden: false,
        children: collapsedNodes,
        width: 200,
        height: 150
      });

      // Create expanded parent that contains both nodes and the collapsed child
      visState.setContainer('expanded_parent', {
        collapsed: false,
        hidden: false,
        children: [...expandedNodes, 'collapsed_child'],
        width: 400,
        height: 300
      });

      // Verify: ELK should see the expanded hierarchy correctly
      const visibleNodes = visState.visibleNodes;
      const expandedContainers = visState.expandedContainers;
      const collapsedAsNodes = visState.getCollapsedContainersAsNodes();

      // Only nodes in expanded containers should be visible
      const visibleNodeIds = visibleNodes.map(n => n.id);
      
      // Expanded parent's direct children should be visible
      for (let i = 0; i < 10; i++) {
        expect(visibleNodeIds).toContain(`expanded_node_${i}`);
      }
      
      // Collapsed child's children should NOT be visible
      for (let i = 0; i < 10; i++) {
        expect(visibleNodeIds).not.toContain(`collapsed_node_${i}`);
      }

      expect(expandedContainers).toHaveLength(1); // Only expanded_parent
      expect(collapsedAsNodes).toHaveLength(1); // Only collapsed_child as node
    });
  });

  describe('Runtime Assertion Protection', () => {
    test('should detect violations with assertion in development', () => {
      // This test verifies that our development-time assertion catches bugs
      
      // Add children that should NEVER appear in visibleNodes for collapsed containers
      const problematicChildren = ['node_a', 'node_b', 'node_c'];
      problematicChildren.forEach(nodeId => {
        visState.setGraphNode(nodeId, {
          label: nodeId,
          hidden: false
        });
      });
      
      // Create scenario that would trigger assertion if children leak through
      visState.setContainer('problematic_container', {
        collapsed: true,
        hidden: false,
        children: problematicChildren,
        width: 200,
        height: 150
      });

      // Manually corrupt _visibleNodes to simulate a hypothetical bug
      problematicChildren.forEach(nodeId => {
        (visState as any)._visibleNodes.set(nodeId, {
          id: nodeId,
          label: nodeId,
          hidden: false
        });
      });

      // This should throw an assertion error in development
      expect(() => {
        visState.visibleNodes; // Accessing the getter should trigger assertion
      }).toThrow(/BUG: Node .* is in _visibleNodes but its parent container .* is collapsed/);
    });
  });

  describe('ELK Bridge Integration', () => {
    test('should provide clean data for ELK Bridge with no dimension explosion risk', () => {
      // Create scenario that simulates real-world complex graphs
      
      // Add many containers and nodes
      const containerIds = [];
      const nodeIds = [];
      
      for (let i = 0; i < 10; i++) {
        const containerId = `container_${i}`;
        const containerNodes = [];
        
        // Add 20 nodes per container
        for (let j = 0; j < 20; j++) {
          const nodeId = `${containerId}_node_${j}`;
          nodeIds.push(nodeId);
          containerNodes.push(nodeId);
          
          visState.setGraphNode(nodeId, {
            label: `Node ${i}-${j}`,
            hidden: false
          });
        }
        
        // Create containers with some collapsed, some expanded
        visState.setContainer(containerId, {
          collapsed: i % 3 === 0, // Every 3rd container is collapsed
          hidden: false,
          children: containerNodes,
          width: 200,
          height: 150
        });
        
        containerIds.push(containerId);
      }

      // Verify ELK sees clean, manageable data
      const visibleNodes = visState.visibleNodes;
      const expandedContainers = visState.expandedContainers;
      const collapsedAsNodes = visState.getCollapsedContainersAsNodes();
      
      // Check no container has excessive children that would cause dimension explosion
      expandedContainers.forEach(container => {
        expect(container.children.size).toBeLessThanOrEqual(20); // Reasonable limit
      });
      
      // Verify total ELK input is manageable
      const totalELKNodes = visibleNodes.length + collapsedAsNodes.length;
      const totalContainerHierarchies = expandedContainers.length;
      
      // Should be much less than the 200 total nodes we created
      expect(totalELKNodes).toBeLessThan(150); // Most should be hidden in collapsed containers
      expect(totalContainerHierarchies).toBeLessThanOrEqual(7); // Only expanded containers
      
      // Collapsed containers should appear as simple nodes
      expect(collapsedAsNodes.length).toBeGreaterThan(0);
      collapsedAsNodes.forEach(node => {
        expect(node.width).toBeGreaterThan(0);
        expect(node.height).toBeGreaterThan(0);
      });
    });
  });
});
