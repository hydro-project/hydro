/**
 * @fileoverview ELKBridge Unit Tests
 * 
 * Tests for the ELK bridge that handles VisState ↔ ELK conversion and layout
 */

import assert from 'assert';
import { ELKBridge } from './ELKBridge';
import type { ElkGraph, ElkNode } from './elk-types';

console.log('Running ELKBridge tests...');

// ============ Mock VisState for Testing ============

function createMockVisState() {
  const mockNodes = [
    { id: 'node1', label: 'Node 1', x: 0, y: 0, width: 180, height: 60, hidden: false, style: 'default' },
    { id: 'node2', label: 'Node 2', x: 0, y: 0, width: 180, height: 60, hidden: false, style: 'default' }
  ];
  
  const mockContainers = [
    {
      id: 'container1',
      collapsed: false,
      hidden: false,
      children: new Set(['node1']),
      layout: {
        position: { x: 0, y: 0 },
        dimensions: { width: 300, height: 200 }
      },
      style: 'default'
    }
  ];
  
  const mockEdges = [
    { id: 'edge1', source: 'node1', target: 'node2', hidden: false, style: 'default' }
  ];
  
  const mockHyperEdges = [
    { id: 'hyper_container1_to_node2', source: 'container1', target: 'node2', style: 'default', aggregatedEdges: [] }
  ];
  
  return {
    visibleNodes: mockNodes,
    visibleContainers: mockContainers,
    expandedContainers: mockContainers.filter(c => !c.collapsed),
    visibleEdges: mockEdges,
    allHyperEdges: mockHyperEdges,
    
    getGraphNode: (id: string) => mockNodes.find(n => n.id === id) || null,
    getContainer: (id: string) => mockContainers.find(c => c.id === id) || null
  };
}

// ============ ELK Bridge Tests ============

function testVisStateToELKExtraction(): void {
  console.log('  Testing VisState data extraction...');
  
  const bridge = new ELKBridge();
  const mockVisState = createMockVisState();
  
  // Access private method for testing (using bracket notation to bypass TypeScript)
  const elkGraph = (bridge as any).visStateToELK(mockVisState);
  
  assert.ok(elkGraph, 'Should generate ELK graph');
  assert.strictEqual(elkGraph.id, 'root', 'Root graph should have correct ID');
  assert.ok(elkGraph.children, 'Should have children array');
  assert.ok(elkGraph.edges, 'Should have edges array');
  
  // Should include both regular edges and hyperedges
  assert.strictEqual(elkGraph.edges.length, 2, 'Should include both regular edge and hyperedge');
  
  const edgeIds = elkGraph.edges.map(e => e.id);
  assert.ok(edgeIds.includes('edge1'), 'Should include regular edge');
  assert.ok(edgeIds.includes('hyper_container1_to_node2'), 'Should include hyperedge');
  
  console.log('    ✅ VisState data extraction works correctly');
}

function testExtractVisibleNodes(): void {
  console.log('  Testing visible node extraction...');
  
  const bridge = new ELKBridge();
  const mockVisState = createMockVisState();
  
  // Add a collapsed container to test
  mockVisState.visibleContainers.push({
    id: 'collapsed_container',
    collapsed: true,
    hidden: false,
    children: new Set(['node3']),
    layout: {
      position: { x: 100, y: 100 },
      dimensions: { width: 200, height: 60 }
    },
    style: 'default'
  });
  
  const visibleNodes = (bridge as any).extractVisibleNodes(mockVisState);
  
  assert.ok(Array.isArray(visibleNodes), 'Should return array of nodes');
  assert.strictEqual(visibleNodes.length, 3, 'Should include 2 regular nodes + 1 collapsed container as node');
  
  // Check that collapsed container is treated as a node
  const collapsedAsNode = visibleNodes.find(n => n.id === 'collapsed_container');
  assert.ok(collapsedAsNode, 'Collapsed container should be included as a node');
  assert.strictEqual(collapsedAsNode.x, 100, 'Should preserve collapsed container position');
  assert.strictEqual(collapsedAsNode.y, 100, 'Should preserve collapsed container position');
  
  console.log('    ✅ Visible node extraction works correctly');
}

function testExtractAllEdges(): void {
  console.log('  Testing edge extraction (regular + hyperedges)...');
  
  const bridge = new ELKBridge();
  const mockVisState = createMockVisState();
  
  const allEdges = (bridge as any).extractAllEdges(mockVisState);
  
  assert.ok(Array.isArray(allEdges), 'Should return array of edges');
  assert.strictEqual(allEdges.length, 2, 'Should include both regular edges and hyperedges');
  
  const regularEdge = allEdges.find(e => e.id === 'edge1');
  const hyperEdge = allEdges.find(e => e.id === 'hyper_container1_to_node2');
  
  assert.ok(regularEdge, 'Should include regular edge');
  assert.ok(hyperEdge, 'Should include hyperedge');
  
  // Verify edge structure
  assert.strictEqual(regularEdge.source, 'node1', 'Regular edge should have correct source');
  assert.strictEqual(regularEdge.target, 'node2', 'Regular edge should have correct target');
  assert.strictEqual(hyperEdge.source, 'container1', 'Hyperedge should have correct source');
  assert.strictEqual(hyperEdge.target, 'node2', 'Hyperedge should have correct target');
  
  console.log('    ✅ Edge extraction (including hyperedges) works correctly');
}

function testBuildELKGraph(): void {
  console.log('  Testing ELK graph construction...');
  
  const bridge = new ELKBridge();
  const mockVisState = createMockVisState();
  
  const nodes = (bridge as any).extractVisibleNodes(mockVisState);
  const containers = (bridge as any).extractVisibleContainers(mockVisState);
  const edges = (bridge as any).extractAllEdges(mockVisState);
  
  const elkGraph = (bridge as any).buildELKGraph(nodes, containers, edges);
  
  assert.strictEqual(elkGraph.id, 'root', 'Should have correct root ID');
  assert.ok(elkGraph.layoutOptions, 'Should have layout options');
  assert.strictEqual(elkGraph.layoutOptions['elk.algorithm'], 'layered', 'Should use layered algorithm');
  
  // Check children (containers + top-level nodes)
  assert.ok(elkGraph.children, 'Should have children');
  assert.ok(elkGraph.children.length > 0, 'Should have at least one child');
  
  // Check edges
  assert.ok(elkGraph.edges, 'Should have edges');
  assert.strictEqual(elkGraph.edges.length, 2, 'Should have both regular and hyperedges');
  
  console.log('    ✅ ELK graph construction works correctly');
}

function testELKResultApplication(): void {
  console.log('  Testing ELK result application to VisState...');
  
  const bridge = new ELKBridge();
  const mockVisState = createMockVisState();
  
  // Mock ELK result
  const elkResult: ElkGraph = {
    id: 'root',
    children: [
      {
        id: 'container1',
        x: 50,
        y: 75,
        width: 350,
        height: 250,
        children: [
          { id: 'node1', x: 20, y: 30, width: 180, height: 60 }
        ]
      },
      {
        id: 'node2',
        x: 450,
        y: 100,
        width: 180,
        height: 60
      }
    ]
  };
  
  // Apply ELK results
  (bridge as any).elkToVisState(elkResult, mockVisState);
  
  // Check that container was updated
  const container = mockVisState.getContainer('container1');
  assert.ok(container, 'Container should exist');
  assert.strictEqual(container.layout.position.x, 50, 'Container x should be updated');
  assert.strictEqual(container.layout.position.y, 75, 'Container y should be updated');
  assert.strictEqual(container.layout.dimensions.width, 350, 'Container width should be updated');
  assert.strictEqual(container.layout.dimensions.height, 250, 'Container height should be updated');
  
  // Check that nodes were updated
  const node1 = mockVisState.getGraphNode('node1');
  const node2 = mockVisState.getGraphNode('node2');
  assert.ok(node1, 'Node1 should exist');
  assert.ok(node2, 'Node2 should exist');
  assert.strictEqual(node1.x, 20, 'Node1 x should be updated');
  assert.strictEqual(node1.y, 30, 'Node1 y should be updated');
  assert.strictEqual(node2.x, 450, 'Node2 x should be updated');
  assert.strictEqual(node2.y, 100, 'Node2 y should be updated');
  
  console.log('    ✅ ELK result application works correctly');
}

function testContainerHierarchy(): void {
  console.log('  Testing container hierarchy handling...');
  
  const bridge = new ELKBridge();
  const mockVisState = createMockVisState();
  
  // Test isNodeInContainer helper
  const container = mockVisState.visibleContainers[0];
  const isInContainer = (bridge as any).isNodeInContainer('node1', 'container1', container);
  const isNotInContainer = (bridge as any).isNodeInContainer('node2', 'container1', container);
  
  assert.strictEqual(isInContainer, true, 'Should correctly identify node in container');
  assert.strictEqual(isNotInContainer, false, 'Should correctly identify node not in container');
  
  console.log('    ✅ Container hierarchy handling works correctly');
}

// ============ Run All Tests ============

export function runELKBridgeTests(): void {
  console.log('🧪 ELKBridge Tests:');
  
  try {
    testVisStateToELKExtraction();
    testExtractVisibleNodes();
    testExtractAllEdges();
    testBuildELKGraph();
    testELKResultApplication();
    testContainerHierarchy();
    
    console.log('✅ All ELKBridge tests passed!');
  } catch (error) {
    console.error('❌ ELKBridge test failed:', error);
    throw error;
  }
}

// Run tests if this file is executed directly
if (require.main === module) {
  runELKBridgeTests();
}
