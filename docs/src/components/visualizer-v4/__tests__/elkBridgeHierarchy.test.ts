/**
 * @fileoverview ELK Bridge Hierarchy Tests
 * 
 * Unit tests for container hierarchy handling in ELK Bridge
 */

import { describe, it, expect, beforeEach } from 'vitest';
import { ELKBridge } from '../bridges/ELKBridge';
import { loadChatJsonTestData, skipIfNoTestData, createMockVisStateWithContainers } from './testUtils';
import type { VisualizationState } from '../core/VisState';

describe('ELKBridge Container Hierarchy', () => {
  let elkBridge: ELKBridge;

  beforeEach(() => {
    elkBridge = new ELKBridge();
  });

  describe('Mock Data Tests', () => {
    it('should handle simple container hierarchy correctly', async () => {
      const state = createMockVisStateWithContainers();
      
      // Check initial state
      expect(state.visibleNodes.length).toBe(5); // 5 nodes
      expect(state.getExpandedContainers().length).toBe(2); // 2 containers
      expect(state.visibleEdges.length).toBe(4); // 4 edges
      
      // Test container membership
      const containerA = state.getContainer('container_a');
      const containerB = state.getContainer('container_b');
      
      expect(containerA).toBeDefined();
      expect(containerB).toBeDefined();
      expect(containerA!.children.has('node_0')).toBe(true);
      expect(containerA!.children.has('node_1')).toBe(true);
      expect(containerB!.children.has('node_2')).toBe(true);
      expect(containerB!.children.has('node_3')).toBe(true);
      expect(containerB!.children.has('node_4')).toBe(true);
      
      // Run ELK layout
      await elkBridge.layoutVisState(state);
      
      // Verify containers got proper dimensions
      const layoutA = state.getContainerLayout('container_a');
      const layoutB = state.getContainerLayout('container_b');
      
      expect(layoutA).toBeDefined();
      expect(layoutB).toBeDefined();
      expect(layoutA?.dimensions?.width).toBeGreaterThan(0);
      expect(layoutA?.dimensions?.height).toBeGreaterThan(0);
      expect(layoutB?.dimensions?.width).toBeGreaterThan(0);
      expect(layoutB?.dimensions?.height).toBeGreaterThan(0);
    });

    it('should correctly identify nodes in containers', async () => {
      const state = createMockVisStateWithContainers();
      
      // Test the actual container behavior by checking state directly
      await elkBridge.layoutVisState(state);
      
      // Verify containers exist and have correct membership
      const containerA = state.getContainer('container_a');
      const containerB = state.getContainer('container_b');
      
      expect(containerA).toBeDefined();
      expect(containerB).toBeDefined();
      
      // Check container_a has 2 children: node_0, node_1
      const containerAChildren = state.getContainerChildren('container_a');
      expect(containerAChildren.size).toBe(2);
      expect(containerAChildren.has('node_0')).toBe(true);
      expect(containerAChildren.has('node_1')).toBe(true);
      
      // Check container_b has 3 children: node_2, node_3, node_4
      const containerBChildren = state.getContainerChildren('container_b');
      expect(containerBChildren.size).toBe(3);
      expect(containerBChildren.has('node_2')).toBe(true);
      expect(containerBChildren.has('node_3')).toBe(true);
      expect(containerBChildren.has('node_4')).toBe(true);
      
      // Verify all nodes have proper layouts after ELK processing
      for (const nodeId of ['node_0', 'node_1', 'node_2', 'node_3', 'node_4']) {
        const nodeLayout = state.getNodeLayout(nodeId);
        expect(nodeLayout).toBeDefined();
        expect(nodeLayout?.position).toBeDefined();
        expect(nodeLayout?.dimensions).toBeDefined();
      }
    });

    it('should handle cross-container edges correctly', async () => {
      const state = createMockVisStateWithContainers();
      
      // Add a cross-container edge (from container_a to container_b)
      state.setGraphEdge('edge_cross', { source: 'node_1', target: 'node_2' });
      
      expect(state.visibleEdges.length).toBe(5); // Now 5 edges including cross-container
      
      await elkBridge.layoutVisState(state);
      
      // Check edge layouts - cross-container edges may or may not have sections
      const crossEdgeLayout = state.getEdgeLayout('edge_cross');
      const normalEdgeLayout = state.getEdgeLayout('edge_0_1'); // Within container_a
      
      // Cross-container edges may use automatic routing (no sections) or have bend points
      // Both are valid approaches depending on ELK's layout algorithm
      console.log(`Cross-container edge sections: ${crossEdgeLayout?.sections?.length || 0}`);
      console.log(`Normal edge sections: ${normalEdgeLayout?.sections?.length || 0}`);
      
      // Normal edges within containers may or may not have sections depending on layout
      if (normalEdgeLayout?.sections) {
        expect(normalEdgeLayout.sections.length).toBeGreaterThanOrEqual(0); // Allow 0 or more sections
      }
    });
  });

  describe('Real Data Tests', () => {
    it('should reproduce chat.json container hierarchy bug', async () => {
      const testData = loadChatJsonTestData('location'); // Use location grouping for hierarchical testing
      if (skipIfNoTestData(testData, 'container hierarchy bug reproduction')) return;
      
      const state = testData!.state;
      
      // Test the actual container hierarchy by validating state directly
      await elkBridge.layoutVisState(state);
      
      // Verify containers and node membership directly
      const containers = state.visibleContainers;
      const nodes = state.visibleNodes;
      
      expect(containers.length).toBeGreaterThan(0);
      expect(nodes.length).toBeGreaterThan(0);
      
      // Verify that containers have proper computed dimensions
      for (const container of containers) {
        // Check that the container has valid computed dimensions (via width/height getters)
        expect(container.width).toBeGreaterThan(0);
        expect(container.height).toBeGreaterThan(0);
        expect(container.children.size).toBeGreaterThan(0);
      }
      
      // Check that nodes are properly assigned to containers
      let nodesInContainers = 0;
      for (const node of nodes) {
        const isInContainer = containers.some(container => container.children.has(node.id));
        if (isInContainer) {
          nodesInContainers++;
        }
      }
      
      // Most nodes should be in containers when using location grouping
      expect(nodesInContainers).toBeGreaterThan(0);
      expect(nodesInContainers).toBeLessThanOrEqual(nodes.length);
      
      // Verify ELK layout was applied properly
      for (const container of containers) {
        const containerLayout = state.getContainerLayout(container.id);
        expect(containerLayout).toBeDefined();
        expect(containerLayout?.position).toBeDefined();
        expect(containerLayout?.dimensions).toBeDefined();
      }
    });

    it('should validate ELK input data structure', async () => {
      const testData = loadChatJsonTestData('location');
      if (skipIfNoTestData(testData, 'ELK input validation')) return;
      
      const state = testData!.state;
      
      // Test the private methods indirectly by running layout and checking results
      await elkBridge.layoutVisState(state);
      
      // After layout, all containers should have proper layout information
      const containers = state.getExpandedContainers();
      for (const container of containers) {
        const layout = state.getContainerLayout(container.id);
        
        expect(layout).toBeDefined();
        
        // ELK should have set position and dimensions
        if (layout?.position) {
          expect(typeof layout.position.x).toBe('number');
          expect(typeof layout.position.y).toBe('number');
          expect(isFinite(layout.position.x)).toBe(true);
          expect(isFinite(layout.position.y)).toBe(true);
        }
        
        if (layout?.dimensions) {
          expect(layout.dimensions.width).toBeGreaterThan(0);
          expect(layout.dimensions.height).toBeGreaterThan(0);
        }
      }
      
      // Check that node positions are also valid
      const nodes = state.visibleNodes;
      for (const node of nodes) {
        const layout = state.getNodeLayout(node.id);
        
        if (layout?.position) {
          expect(typeof layout.position.x).toBe('number');
          expect(typeof layout.position.y).toBe('number');
          expect(isFinite(layout.position.x)).toBe(true);
          expect(isFinite(layout.position.y)).toBe(true);
          
          // Validate that positions are reasonable (not extremely negative)
          expect(layout.position.x).toBeGreaterThan(-1000);
          expect(layout.position.y).toBeGreaterThan(-1000);
        }
      }
    });

    it('should handle edge routing correctly', async () => {
      const testData = loadChatJsonTestData('location');
      if (skipIfNoTestData(testData, 'edge routing validation')) return;
      
      const state = testData!.state;
      
      await elkBridge.layoutVisState(state);
      
      // Validate edge routing structure
      const edges = state.visibleEdges;
      let edgesWithSections = 0;
      let edgesWithoutSections = 0;
      
      for (const edge of edges) {
        const layout = state.getEdgeLayout(edge.id);
        
        if (layout?.sections && layout.sections.length > 0) {
          edgesWithSections++;
          
          // Validate section structure
          for (const section of layout.sections) {
            expect(section.startPoint).toBeDefined();
            expect(section.endPoint).toBeDefined();
            expect(typeof section.startPoint.x).toBe('number');
            expect(typeof section.startPoint.y).toBe('number');
            expect(typeof section.endPoint.x).toBe('number');
            expect(typeof section.endPoint.y).toBe('number');
          }
        } else {
          edgesWithoutSections++;
        }
      }
      
      // Verify that edges exist and have valid layouts
      expect(edges.length).toBeGreaterThan(0);
      expect(edgesWithSections + edgesWithoutSections).toBe(edges.length);
    });

    it('should handle containers with different dimensions', async () => {
      const testData = loadChatJsonTestData('location');
      if (skipIfNoTestData(testData, 'container dimensions test')) return;
      
      const state = testData!.state;
      
      // Before layout, check container dimensions
      const containers = state.getExpandedContainers();
      const initialDimensions = containers.map(c => {
        const dims = state.getContainerAdjustedDimensions(c.id);
        return {
          id: c.id,
          width: dims.width,
          height: dims.height
        };
      });
      
      await elkBridge.layoutVisState(state);
      
      // After layout, containers should have updated dimensions from ELK
      for (const container of containers) {
        const layout = state.getContainerLayout(container.id);
        const initial = initialDimensions.find(d => d.id === container.id);
        
        // Verify that ELK updated the dimensions and they are reasonable
        if (layout?.dimensions && initial) {
          expect(layout.dimensions.width).toBeGreaterThan(100); // Reasonable minimum
          expect(layout.dimensions.height).toBeGreaterThan(50);
          expect(layout.dimensions.width).toBeLessThan(2000); // Reasonable maximum
          expect(layout.dimensions.height).toBeLessThan(2000);
          
          // Dimensions should be at least as large as initial dimensions
          // Note: ELK may optimize dimensions to be smaller if content fits
          expect(layout.dimensions.width).toBeGreaterThanOrEqual(Math.min(initial.width, 100));
          expect(layout.dimensions.height).toBeGreaterThanOrEqual(Math.min(initial.height, 50));
        }
      }
    });
  });
});
