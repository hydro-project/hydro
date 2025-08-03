/**
 * @fileoverview ReactFlowBridge Unit Tests
 * 
 * Tests for the ReactFlow bridge that converts VisState to ReactFlow format
 */

import { describe, it, expect } from 'vitest';
import { ReactFlowBridge } from './ReactFlowBridge';
import type { ReactFlowData } from './ReactFlowBridge';

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
      }
    ],
    allHyperEdges: [
      {
        id: 'hyper1',
        sources: ['container1'],
        targets: ['node2'],
        style: 'default'
      }
    ]
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
      
      const regularEdges = result.edges.filter(e => e.type === 'standard');
      expect(regularEdges.length).toBe(1);
      
      const edge1 = regularEdges.find(e => e.id === 'edge1');
      expect(edge1).toBeDefined();
      expect(edge1!.source).toBe('node1');
      expect(edge1!.target).toBe('node2');
    });

    it('should convert hyperedges correctly', () => {
      const mockVisState = createMockVisState();
      const result = bridge.visStateToReactFlow(mockVisState as any);
      
      const hyperEdges = result.edges.filter(e => e.type === 'hyper');
      expect(hyperEdges.length).toBe(1);
      
      const hyperEdge = hyperEdges[0];
      expect(hyperEdge.id).toContain('hyper_container1_to_node2');
      expect(hyperEdge.source).toBe('container1');
      expect(hyperEdge.target).toBe('node2');
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
        visibleEdges: [],
        allHyperEdges: []
      };
      
      const result = bridge.visStateToReactFlow(testVisState as any);
      const childNode = result.nodes.find(n => n.id === 'child_node');
      
      expect(childNode).toBeDefined();
      // ELK absolute: (170, 225), Container: (100, 150) → ReactFlow relative: (70, 75)
      expect(childNode!.position.x).toBe(70);
      expect(childNode!.position.y).toBe(75);
      expect(childNode!.parentId).toBe('parent_container');
      expect(childNode!.extent).toBe('parent');
    });
  });
});
