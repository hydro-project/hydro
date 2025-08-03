/**
 * @fileoverview ReactFlowBridge Unit Tests
 * 
 * Tests for the ReactFlow bridge that converts VisState to ReactFlow format
 */

import assert from 'assert';
import { ReactFlowBridge } from './ReactFlowBridge';
import type { ReactFlowData } from './ReactFlowBridge';

console.log('Running ReactFlowBridge tests...');

// ============ Mock VisState for Testing ============

function createMockVisState() {
  const mockNodes = [
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
  ];
  
  const mockContainers = [
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
    },
    {
      id: 'collapsed_container',
      collapsed: true,
      hidden: false,
      children: new Set(['node3']),
      layout: {
        position: { x: 200, y: 300 },
        dimensions: { width: 200, height: 60 }
      },
      style: 'minimized'
    }
  ];
  
  const mockEdges = [
    { id: 'edge1', source: 'node1', target: 'node2', hidden: false, style: 'default' },
    { id: 'edge2', source: 'container1', target: 'node2', hidden: false, style: 'thick' }
  ];
  
  const mockHyperEdges = [
    { 
      id: 'hyper_container1_to_node2', 
      source: 'container1', 
      target: 'node2', 
      style: 'dashed', 
      aggregatedEdges: [
        { id: 'inner1', source: 'node1', target: 'node2' }
      ]
    }
  ];
  
  return {
    visibleNodes: mockNodes,
    visibleContainers: mockContainers,
    expandedContainers: mockContainers.filter(c => !c.collapsed),
    visibleEdges: mockEdges,
    allHyperEdges: mockHyperEdges
  };
}

// ============ ReactFlowBridge Tests ============

function testVisStateToReactFlowConversion(): void {
  console.log('  Testing complete VisState to ReactFlow conversion...');
  
  const bridge = new ReactFlowBridge();
  const mockVisState = createMockVisState();
  
  const result: ReactFlowData = bridge.visStateToReactFlow(mockVisState as any);
  
  assert.ok(result, 'Should return ReactFlow data');
  assert.ok(Array.isArray(result.nodes), 'Should have nodes array');
  assert.ok(Array.isArray(result.edges), 'Should have edges array');
  
  // Should have containers + regular nodes
  assert.ok(result.nodes.length >= 4, 'Should have containers and nodes'); // 2 containers + 2 nodes
  
  // Should have regular edges + hyperedges
  assert.strictEqual(result.edges.length, 3, 'Should have all edges including hyperedges'); // 2 regular + 1 hyper
  
  console.log('    ✅ Complete conversion works correctly');
}

function testParentChildMapping(): void {
  console.log('  Testing parent-child relationship mapping...');
  
  const bridge = new ReactFlowBridge();
  const mockVisState = createMockVisState();
  
  // Access private method for testing
  const parentMap = (bridge as any).buildParentMap(mockVisState);
  
  assert.ok(parentMap instanceof Map, 'Should return a Map');
  assert.strictEqual(parentMap.get('node1'), 'container1', 'Node1 should be child of container1');
  assert.strictEqual(parentMap.get('node2'), undefined, 'Node2 should be top-level (no parent)');
  
  console.log('    ✅ Parent-child mapping works correctly');
}

function testContainerConversion(): void {
  console.log('  Testing container to ReactFlow node conversion...');
  
  const bridge = new ReactFlowBridge();
  const mockVisState = createMockVisState();
  
  const result = bridge.visStateToReactFlow(mockVisState as any);
  const containerNodes = result.nodes.filter(n => n.type === 'container');
  
  assert.strictEqual(containerNodes.length, 2, 'Should convert both containers');
  
  // Test expanded container
  const expandedContainer = containerNodes.find(n => n.id === 'container1');
  assert.ok(expandedContainer, 'Should include expanded container');
  assert.strictEqual(expandedContainer.type, 'container', 'Should have container type');
  assert.strictEqual(expandedContainer.position.x, 50, 'Should preserve container x position');
  assert.strictEqual(expandedContainer.position.y, 75, 'Should preserve container y position');
  assert.strictEqual(expandedContainer.data.collapsed, false, 'Should mark as not collapsed');
  assert.strictEqual(expandedContainer.data.width, 350, 'Should include width');
  assert.strictEqual(expandedContainer.data.height, 250, 'Should include height');
  
  // Test collapsed container
  const collapsedContainer = containerNodes.find(n => n.id === 'collapsed_container');
  assert.ok(collapsedContainer, 'Should include collapsed container');
  assert.strictEqual(collapsedContainer.data.collapsed, true, 'Should mark as collapsed');
  assert.strictEqual(collapsedContainer.position.x, 200, 'Should preserve collapsed container position');
  
  console.log('    ✅ Container conversion works correctly');
}

function testNodeConversion(): void {
  console.log('  Testing regular node conversion...');
  
  const bridge = new ReactFlowBridge();
  const mockVisState = createMockVisState();
  
  const result = bridge.visStateToReactFlow(mockVisState as any);
  const standardNodes = result.nodes.filter(n => n.type === 'standard');
  
  assert.strictEqual(standardNodes.length, 2, 'Should convert both regular nodes');
  
  const node1 = standardNodes.find(n => n.id === 'node1');
  assert.ok(node1, 'Should include node1');
  assert.strictEqual(node1.type, 'standard', 'Should have standard type');
  assert.strictEqual(node1.data.label, 'Node 1', 'Should preserve label');
  assert.strictEqual(node1.data.style, 'default', 'Should preserve style');
  
  // Test coordinate translation for child node
  // Node1 is inside container1, so coordinates should be relative
  // ELK coords: node(120, 180), container(50, 75) → ReactFlow relative: (70, 105)
  assert.strictEqual(node1.position.x, 70, 'Child node x should be relative to container: 120-50=70');
  assert.strictEqual(node1.position.y, 105, 'Child node y should be relative to container: 180-75=105');
  assert.strictEqual(node1.parentId, 'container1', 'Should have correct parent');
  assert.strictEqual(node1.extent, 'parent', 'Should be constrained to parent');
  
  // Test top-level node
  const node2 = standardNodes.find(n => n.id === 'node2');
  assert.ok(node2, 'Should include node2');
  assert.strictEqual(node2.position.x, 450, 'Top-level node x should be absolute');
  assert.strictEqual(node2.position.y, 100, 'Top-level node y should be absolute');
  assert.strictEqual(node2.parentId, undefined, 'Should not have parent');
  assert.strictEqual(node2.extent, undefined, 'Should not be constrained');
  
  console.log('    ✅ Node conversion with coordinate translation works correctly');
}

function testEdgeConversion(): void {
  console.log('  Testing edge conversion...');
  
  const bridge = new ReactFlowBridge();
  const mockVisState = createMockVisState();
  
  const result = bridge.visStateToReactFlow(mockVisState as any);
  
  // Test regular edges
  const regularEdges = result.edges.filter(e => e.type === 'standard');
  assert.strictEqual(regularEdges.length, 2, 'Should convert regular edges');
  
  const edge1 = regularEdges.find(e => e.id === 'edge1');
  assert.ok(edge1, 'Should include edge1');
  assert.strictEqual(edge1.source, 'node1', 'Should preserve source');
  assert.strictEqual(edge1.target, 'node2', 'Should preserve target');
  assert.strictEqual(edge1.data.style, 'default', 'Should preserve style');
  assert.ok(edge1.markerEnd, 'Should have arrow marker');
  
  console.log('    ✅ Regular edge conversion works correctly');
}

function testHyperEdgeConversion(): void {
  console.log('  Testing hyperedge conversion...');
  
  const bridge = new ReactFlowBridge();
  const mockVisState = createMockVisState();
  
  const result = bridge.visStateToReactFlow(mockVisState as any);
  
  // Test hyperedges
  const hyperEdges = result.edges.filter(e => e.type === 'hyper');
  assert.strictEqual(hyperEdges.length, 1, 'Should convert hyperedges');
  
  const hyperEdge = hyperEdges.find(e => e.id === 'hyper_container1_to_node2');
  assert.ok(hyperEdge, 'Should include hyperedge');
  assert.strictEqual(hyperEdge.type, 'hyper', 'Should have hyper type');
  assert.strictEqual(hyperEdge.source, 'container1', 'Should preserve hyperedge source');
  assert.strictEqual(hyperEdge.target, 'node2', 'Should preserve hyperedge target');
  assert.strictEqual(hyperEdge.data.style, 'dashed', 'Should preserve hyperedge style');
  
  console.log('    ✅ Hyperedge conversion works correctly');
}

function testCustomPropertyExtraction(): void {
  console.log('  Testing custom property extraction...');
  
  const bridge = new ReactFlowBridge();
  const mockVisState = createMockVisState();
  
  const result = bridge.visStateToReactFlow(mockVisState as any);
  const node1 = result.nodes.find(n => n.id === 'node1');
  
  assert.ok(node1, 'Should find node1');
  assert.strictEqual(node1.data.customProp, 'test-value', 'Should preserve custom properties');
  
  // Should not include known properties as custom
  assert.strictEqual(node1.data.id, undefined, 'Should not include id as custom property');
  assert.strictEqual(node1.data.x, undefined, 'Should not include x as custom property');
  assert.strictEqual(node1.data.hidden, undefined, 'Should not include hidden as custom property');
  
  console.log('    ✅ Custom property extraction works correctly');
}

function testCoordinateTranslationIntegration(): void {
  console.log('  Testing coordinate translation integration...');
  
  const bridge = new ReactFlowBridge();
  
  // Create test data with known coordinates
  const testVisState = {
    visibleNodes: [
      { id: 'child_node', label: 'Child', x: 170, y: 225, width: 180, height: 60, style: 'default', hidden: false }
    ],
    visibleContainers: [
      {
        id: 'parent_container',
        collapsed: false,
        hidden: false,
        children: new Set(['child_node']),
        layout: {
          position: { x: 100, y: 150 },
          dimensions: { width: 300, height: 200 }
        },
        style: 'default'
      }
    ],
    expandedContainers: [
      {
        id: 'parent_container',
        collapsed: false,
        hidden: false,
        children: new Set(['child_node']),
        layout: {
          position: { x: 100, y: 150 },
          dimensions: { width: 300, height: 200 }
        },
        style: 'default'
      }
    ],
    visibleEdges: [],
    allHyperEdges: []
  };
  
  const result = bridge.visStateToReactFlow(testVisState as any);
  const childNode = result.nodes.find(n => n.id === 'child_node');
  
  assert.ok(childNode, 'Should find child node');
  // ELK absolute: (170, 225), Container: (100, 150) → ReactFlow relative: (70, 75)
  assert.strictEqual(childNode.position.x, 70, 'Child x should be relative: 170-100=70');
  assert.strictEqual(childNode.position.y, 75, 'Child y should be relative: 225-150=75');
  assert.strictEqual(childNode.parentId, 'parent_container', 'Should have correct parent');
  
  console.log('    ✅ Coordinate translation integration works correctly');
}

// ============ Run All Tests ============

export function runReactFlowBridgeTests(): void {
  console.log('🧪 ReactFlowBridge Tests:');
  
  try {
    testVisStateToReactFlowConversion();
    testParentChildMapping();
    testContainerConversion();
    testNodeConversion();
    testEdgeConversion();
    testHyperEdgeConversion();
    testCustomPropertyExtraction();
    testCoordinateTranslationIntegration();
    
    console.log('✅ All ReactFlowBridge tests passed!');
  } catch (error) {
    console.error('❌ ReactFlowBridge test failed:', error);
    throw error;
  }
}

// Run tests if this file is executed directly
if (require.main === module) {
  runReactFlowBridgeTests();
}
