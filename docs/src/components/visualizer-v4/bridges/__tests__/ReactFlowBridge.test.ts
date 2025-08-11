/**
 * @fileoverview ReactFlowBridge Unit Tests
 * 
 * Tests for the ReactFlow bridge that converts VisState to ReactFlow format
 */

import { describe, it, expect } from 'vitest';
import { ReactFlowBridge } from '../ReactFlowBridge';
import type { ReactFlowData } from '../ReactFlowBridge';

describe('ReactFlowBridge', () => {
  const bridge = new ReactFlowBridge();

  // Helper to create a simple mock VisState
  const createMockVisState = () => ({
    visibleNodes: [
      { 
        id: 'node1', 
        label: 'Node 1', 
        x: 120, 
        y: 180, 
        width: 180, 
        height: 60, 
        hidden: false, 
        style: 'default',
        customProp: 'test-value'
      },
      { 
        id: 'node2', 
        label: 'Node 2', 
        x: 450, 
        y: 100, 
        width: 180, 
        height: 60, 
        hidden: false, 
        style: 'highlighted'
      }
    ],
    visibleContainers: [
      {
        id: 'container1',
        collapsed: false,
        hidden: false,
        children: new Set(['node1']),
        x: 50,
        y: 75,
        width: 350,
        height: 250,
        layout: {
          position: { x: 50, y: 75 },
          dimensions: { width: 350, height: 250 }
        },
        style: 'default'
      }
    ],
    expandedContainers: [{ id: 'container1' }],
    visibleEdges: [
      { 
        id: 'edge1', 
        source: 'node1', 
        target: 'node2', 
        style: 'default' 
      },
      // Hyperedge included transparently in visibleEdges
      {
        id: 'hyper1',
        source: 'container1',
        target: 'node2',
        style: 'default'
      }
    ],
    // Add the getContainer method that the bridge expects
    getContainer: (id: string) => {
      if (id === 'container1') {
        return {
          layout: {
            position: { x: 50, y: 75 },
            dimensions: { width: 350, height: 250 }
          }
        };
      }
      return null;
    },
    // Add the getNodeLayout method that the bridge expects
    getNodeLayout: (id: string) => {
      // Mock node layout data based on the mock nodes
      const nodeLayoutMap = {
        'node1': { position: { x: 120, y: 180 }, dimensions: { width: 180, height: 60 } },
        'node2': { position: { x: 300, y: 240 }, dimensions: { width: 180, height: 60 } }
      };
      return nodeLayoutMap[id] || null;
    },
    // Add the getContainerAdjustedDimensions method that the bridge expects
    getContainerAdjustedDimensions: (id: string) => {
      if (id === 'container1') {
        return { width: 350, height: 250 };
      }
      return { width: 200, height: 150 }; // Default dimensions
    },
    // Add the getGraphEdge method that the bridge expects
    getGraphEdge: (id: string) => {
      const edgeLayoutMap = {
        'edge1': { 
          id: 'edge1', 
          source: 'node1', 
          target: 'node2', 
          style: 'default',
          layout: { sections: [{ startPoint: { x: 300, y: 210 }, endPoint: { x: 450, y: 130 } }] }
        },
        'hyper1': { 
          id: 'hyper1', 
          source: 'container1', 
          target: 'node2', 
          style: 'default',
          layout: { sections: [{ startPoint: { x: 400, y: 200 }, endPoint: { x: 450, y: 130 } }] }
        }
      };
      return edgeLayoutMap[id] || null;
    }
  });

  describe('visStateToReactFlow', () => {
    it('should convert VisState to ReactFlow format', () => {
      const mockVisState = createMockVisState();
      const result: ReactFlowData = bridge.visStateToReactFlow(mockVisState as any);
      
      expect(result).toBeDefined();
      expect(Array.isArray(result.nodes)).toBe(true);
      expect(Array.isArray(result.edges)).toBe(true);
    });

    it('should convert nodes correctly', () => {
      const mockVisState = createMockVisState();
      const result = bridge.visStateToReactFlow(mockVisState as any);
      
      const standardNodes = result.nodes.filter(n => n.type === 'standard');
      expect(standardNodes.length).toBe(2);
      
      const node1 = standardNodes.find(n => n.id === 'node1');
      expect(node1).toBeDefined();
      expect(node1!.data.label).toBe('Node 1');
      expect(node1!.data.customProp).toBe('test-value');
    });

    it('should convert containers correctly', () => {
      const mockVisState = createMockVisState();
      const result = bridge.visStateToReactFlow(mockVisState as any);
      
      const containerNodes = result.nodes.filter(n => n.type === 'container');
      expect(containerNodes.length).toBe(1);
      
      const container1 = containerNodes.find(n => n.id === 'container1');
      expect(container1).toBeDefined();
      expect(container1!.position.x).toBe(50);
      expect(container1!.position.y).toBe(75);
    });

    it('should convert edges correctly', () => {
      const mockVisState = createMockVisState();
      const result = bridge.visStateToReactFlow(mockVisState as any);
      
      // All edges (including hyperedges) should be processed as standard edges
      const regularEdges = result.edges.filter(e => e.type === 'standard');
      expect(regularEdges.length).toBe(2); // 1 regular + 1 hyperedge (transparently included)
      
      const edge1 = regularEdges.find(e => e.id === 'edge1');
      expect(edge1).toBeDefined();
      expect(edge1!.source).toBe('node1');
      expect(edge1!.target).toBe('node2');
    });

    it('should process edges transparently (including hyperedges)', () => {
      const mockVisState = createMockVisState();
      const result = bridge.visStateToReactFlow(mockVisState as any);
      
      // All edges should be processed as standard ReactFlow edges
      // Hyperedges are included transparently through visibleEdges
      const allEdges = result.edges;
      expect(allEdges.length).toBeGreaterThan(0);
      
      // All edges should be standard type (hyperedges are encapsulated)
      allEdges.forEach(edge => {
        expect(edge.type).toBe('standard');
        expect(edge.id).toBeDefined();
        expect(edge.source).toBeDefined();
        expect(edge.target).toBeDefined();
      });
    });

    it('should handle child node positioning correctly', () => {
      const testVisState = {
        visibleNodes: [
          {
            id: 'child_node',
            label: 'Child',
            x: 170, // ELK absolute position
            y: 225,
            width: 100,
            height: 40,
            style: 'default',
            hidden: false
          }
        ],
        visibleContainers: [
          {
            id: 'parent_container',
            collapsed: false,
            hidden: false,
            children: new Set(['child_node']),
            layout: {
              position: { x: 100, y: 150 }, // Container position
              dimensions: { width: 200, height: 150 }
            },
            style: 'default'
          }
        ],
        expandedContainers: [{ id: 'parent_container' }],
        visibleEdges: [], // No hyperedges in this test scenario
        getContainer: (id: string) => {
          if (id === 'parent_container') {
            return {
              layout: {
                position: { x: 100, y: 150 },
                dimensions: { width: 200, height: 150 }
              }
            };
          }
          return null;
        },
        getNodeLayout: (id: string) => {
          // Mock layout for child node test
          if (id === 'child_node') {
            return { position: { x: 170, y: 225 }, dimensions: { width: 100, height: 40 } };
          }
          return null;
        },
        getContainerAdjustedDimensions: (id: string) => {
          if (id === 'parent_container') {
            return { width: 200, height: 150 };
          }
          return { width: 200, height: 150 }; // Default dimensions
        }
      };
      
      const result = bridge.visStateToReactFlow(testVisState as any);
      const childNode = result.nodes.find(n => n.id === 'child_node');
      
      expect(childNode).toBeDefined();
      // With the fixed coordinate system, ELK child coordinates are already relative
      // ELK: (170, 225) → ReactFlow: (170, 225) (no longer subtract container position)
      expect(childNode!.position.x).toBe(170);
      expect(childNode!.position.y).toBe(225);
      expect(childNode!.parentId).toBe('parent_container');
      // expect(childNode!.extent).toBe('parent'); // REMOVED: No longer setting extent
    });
  });

  describe('handle strategy', () => {
    it('should respect current handle strategy for edge creation', async () => {
      const mockVisState = createMockVisState();
      
      const result = bridge.visStateToReactFlow(mockVisState as any);
      
      // Import the handle configuration to check current strategy
      const { getHandleConfig } = await import('../../render/handleConfig');
      const handleConfig = getHandleConfig();
      
      for (const edge of result.edges) {
        if (handleConfig.enableContinuousHandles) {
          // For continuous handles (ReactFlow v12), handles should be undefined
          // ReactFlow automatically determines optimal connection points
          expect(edge.sourceHandle).toBeUndefined();
          expect(edge.targetHandle).toBeUndefined();
        } else {
          // For discrete handles, handles should be defined
          expect(edge.sourceHandle).toBeDefined();
          expect(edge.targetHandle).toBeDefined();
          expect(typeof edge.sourceHandle).toBe('string');
          expect(typeof edge.targetHandle).toBe('string');
        }
        
        // Basic edge structure should always be valid
        expect(edge.source).toBeDefined();
        expect(edge.target).toBeDefined();
        expect(edge.id).toBeDefined();
        expect(edge.type).toBe('standard');
      }
    });
  });
});
