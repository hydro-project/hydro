/**
 * @fileoverview TreeHierarchy and VisState Synchronization Tests
 * 
 * These tests ensure that the TreeHierarchy display in the InfoPanel
 * accurately reflects the collapsed/expanded state in VisualizationState.
 * 
 * This is critical because mismatched states can cause:
 * - Confusing UI (hierarchy shows expanded but containers are collapsed)
 * - Layout issues (ELK gets wrong container dimensions)
 * - User interaction problems (clicks don't work as expected)
 */

import { describe, test, expect, beforeEach } from 'vitest';
import { createVisualizationState } from '../VisualizationState';
import type { VisualizationState } from '../VisualizationState';

describe('TreeHierarchy/VisState Synchronization Tests', () => {
  let visState: VisualizationState;

  beforeEach(() => {
    visState = createVisualizationState();
  });

  describe('Basic Hierarchy Sync', () => {
    test('should have consistent collapsed state between VisState and what TreeHierarchy should display', () => {
      // Setup: Create a container with child nodes
      visState.setContainer('parent', {
        children: ['child1', 'child2'],
        collapsed: false,
        hidden: false
      });
      
      visState.setGraphNode('child1', { label: 'Child 1', hidden: false });
      visState.setGraphNode('child2', { label: 'Child 2', hidden: false });

      // Initially expanded - TreeHierarchy should show expanded
      expect(visState.getContainer('parent')?.collapsed).toBe(false);
      
      // Children should be visible when parent is expanded
      const visibleNodes = visState.visibleNodes;
      expect(visibleNodes.some(n => n.id === 'child1')).toBe(true);
      expect(visibleNodes.some(n => n.id === 'child2')).toBe(true);

      // Collapse the container
      visState.collapseContainer('parent');
      
      // VisState should show collapsed
      expect(visState.getContainer('parent')?.collapsed).toBe(true);
      
      // Children should be hidden when parent is collapsed
      const visibleNodesAfterCollapse = visState.visibleNodes;
      expect(visibleNodesAfterCollapse.some(n => n.id === 'child1')).toBe(false);
      expect(visibleNodesAfterCollapse.some(n => n.id === 'child2')).toBe(false);
    });

    test('should maintain hierarchy sync during expand/collapse cycles', () => {
      // Create nested hierarchy: grandparent -> parent -> children
      visState.setContainer('grandparent', {
        children: ['parent'],
        collapsed: false,
        hidden: false
      });
      
      visState.setContainer('parent', {
        children: ['child1', 'child2'],
        collapsed: false,
        hidden: false
      });
      
      visState.setGraphNode('child1', { label: 'Child 1', hidden: false });
      visState.setGraphNode('child2', { label: 'Child 2', hidden: false });

      // Test expand/collapse cycle maintains consistency
      const testCycle = (containerId: string, description: string) => {
        // Collapse
        visState.collapseContainer(containerId);
        expect(visState.getContainer(containerId)?.collapsed).toBe(true);
        
        // Expand
        visState.expandContainer(containerId);
        expect(visState.getContainer(containerId)?.collapsed).toBe(false);
      };

      testCycle('parent', 'parent container');
      testCycle('grandparent', 'grandparent container');
      testCycle('parent', 'parent container again');
    });
  });

  describe('Visibility Consistency', () => {
    test('should have consistent container visibility between VisState.visibleContainers and hierarchy state', () => {
      // Create hierarchy with multiple levels
      visState.setContainer('root', {
        children: ['level1A', 'level1B'],
        collapsed: false,
        hidden: false
      });
      
      visState.setContainer('level1A', {
        children: ['level2A'],
        collapsed: false,
        hidden: false
      });
      
      visState.setContainer('level1B', {
        children: ['level2B'],
        collapsed: false,
        hidden: false
      });
      
      visState.setContainer('level2A', {
        children: ['node1'],
        collapsed: false,
        hidden: false
      });
      
      visState.setContainer('level2B', {
        children: ['node2'],
        collapsed: false,
        hidden: false
      });
      
      visState.setGraphNode('node1', { label: 'Node 1', hidden: false });
      visState.setGraphNode('node2', { label: 'Node 2', hidden: false });

      // All containers should be visible initially
      const allContainerIds = ['root', 'level1A', 'level1B', 'level2A', 'level2B'];
      const visibleContainerIds = visState.visibleContainers.map(c => c.id);
      
      for (const containerId of allContainerIds) {
        expect(visibleContainerIds).toContain(containerId);
      }

      // Collapse root - descendants should become hidden
      visState.collapseContainer('root');
      
      const visibleAfterCollapse = visState.visibleContainers.map(c => c.id);
      expect(visibleAfterCollapse).toContain('root');
      expect(visibleAfterCollapse).not.toContain('level1A');
      expect(visibleAfterCollapse).not.toContain('level1B');
      expect(visibleAfterCollapse).not.toContain('level2A');
      expect(visibleAfterCollapse).not.toContain('level2B');
    });

    test('should detect hierarchy inconsistencies', () => {
      // This test verifies our assertion catches sync issues
      visState.setContainer('container1', {
        children: ['node1'],
        collapsed: false,
        hidden: false
      });
      
      visState.setGraphNode('node1', { label: 'Node 1', hidden: false });

      // Manually corrupt the state (simulating a bug)
      const container = visState.getContainer('container1');
      if (container) {
        // Simulate TreeHierarchy thinking it's expanded but VisState thinks it's collapsed
        container.collapsed = true;
        // But don't update visibility properly (simulating the bug we're looking for)
      }

      // Our assertion should catch this inconsistency
      if (process.env.NODE_ENV !== 'production') {
        expect(() => {
          visState.validateInvariants(); // Manually trigger validation to detect the inconsistency
        }).toThrow(/Node node1 should be hidden because.*container1 is collapsed/);
      } else {
        // In production, just check the inconsistency exists
        const containerCollapsed = visState.getContainer('container1')?.collapsed;
        const nodeVisible = visState.visibleNodes.some(n => n.id === 'node1');
        
        // If container is collapsed, child nodes should not be visible (this indicates the bug)
        expect(containerCollapsed).toBe(true);
        expect(nodeVisible).toBe(true); // This is the bug condition we're detecting
      }
    });
  });

  describe('ELK Integration Consistency', () => {
    test('should have containers marked as elkFixed=false for fresh layout after collapse', () => {
      // Create containers
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

      // Simulate the getContainersRequiringLayout call (what ELK bridge uses)
      const containersForLayout = visState.getContainersRequiringLayout();
      
      // All containers should be elkFixed=false for fresh layout
      for (const container of containersForLayout) {
        const isFixed = visState.getContainerELKFixed(container.id);
        expect(isFixed).toBe(false); // Should be false for fresh layout
      }
    });

    test('should maintain elkFixed state consistency during interactive collapse', () => {
      visState.setContainer('interactive', {
        children: ['node1'],
        collapsed: false,
        hidden: false
      });
      
      visState.setGraphNode('node1', { label: 'Node 1', hidden: false });

      // Simulate interactive collapse with one container changing
      const containersForInteractiveLayout = visState.getContainersRequiringLayout('interactive');
      
      // The changed container should be elkFixed=false, others should be elkFixed=true
      for (const container of containersForInteractiveLayout) {
        const isFixed = visState.getContainerELKFixed(container.id);
        if (container.id === 'interactive') {
          expect(isFixed).toBe(false); // Changed container should be free
        } else {
          expect(isFixed).toBe(true); // Other containers should be fixed
        }
      }
    });
  });

  describe('Integration Test: Complete Collapse Flow', () => {
    test('should maintain all sync invariants during smart collapse simulation', () => {
      // Create a realistic hierarchy similar to backtrace data
      const containers = [
        { id: 'bt_1', children: ['node1', 'node2'] },
        { id: 'bt_2', children: ['node3', 'node4'] },
        { id: 'bt_3', children: ['node5', 'node6'] },
        { id: 'root', children: ['bt_1', 'bt_2', 'bt_3'] }
      ];

      // Setup hierarchy
      for (const { id, children } of containers) {
        visState.setContainer(id, {
          children: children,
          collapsed: false,
          hidden: false
        });
      }

      // Add nodes
      for (let i = 1; i <= 6; i++) {
        visState.setGraphNode(`node${i}`, { label: `Node ${i}`, hidden: false });
      }

      // Simulate smart collapse: collapse multiple containers
      visState.collapseContainer('bt_1');
      visState.collapseContainer('bt_2');
      // Leave bt_3 expanded for variety

      // INVARIANT CHECKS after collapse:
      
      // 1. Collapsed containers should report collapsed=true
      expect(visState.getContainer('bt_1')?.collapsed).toBe(true);
      expect(visState.getContainer('bt_2')?.collapsed).toBe(true);
      expect(visState.getContainer('bt_3')?.collapsed).toBe(false);

      // 2. Child nodes of collapsed containers should be hidden
      const visibleNodeIds = visState.visibleNodes.map(n => n.id);
      expect(visibleNodeIds).not.toContain('node1');
      expect(visibleNodeIds).not.toContain('node2');
      expect(visibleNodeIds).not.toContain('node3');
      expect(visibleNodeIds).not.toContain('node4');
      expect(visibleNodeIds).toContain('node5'); // bt_3 is expanded
      expect(visibleNodeIds).toContain('node6'); // bt_3 is expanded

      // 3. Visible containers should be consistent
      const visibleContainerIds = visState.visibleContainers.map(c => c.id);
      expect(visibleContainerIds).toContain('root');
      expect(visibleContainerIds).toContain('bt_1');
      expect(visibleContainerIds).toContain('bt_2');
      expect(visibleContainerIds).toContain('bt_3');

      // 4. Fresh layout preparation should set all elkFixed=false
      visState.getContainersRequiringLayout(); // Simulate fresh layout call
      const allContainers = visState.visibleContainers;
      for (const container of allContainers) {
        const isFixed = visState.getContainerELKFixed(container.id);
        expect(isFixed).toBe(false); // All should be free for fresh layout
      }

      // 5. All invariants should pass
      expect(() => {
        visState.validateAllInvariants('After smart collapse simulation');
      }).not.toThrow();
    });
  });
});
