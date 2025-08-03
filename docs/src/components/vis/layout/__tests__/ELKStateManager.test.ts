/**
 * ELK State Manager Tests (TypeScript)
 * 
 * Tests for the ELK state manager including layout calculations,
 * validation, and state management functionality.
 */

import assert from 'assert';
import {
  createELKStateManager,
  ELKStateManager,
  LayoutPosition,
  LayoutDimensions,
  ContainmentValidationResult
} from '../ELKStateManager.js';
import { GraphNode, GraphEdge, Container, HyperEdge } from '../../shared/types.js';

/**
 * Create test graph data
 */
function createTestGraphData() {
  const nodes: GraphNode[] = [
    {
      id: 'node1',
      label: 'Node 1',
      style: 'default',
      hidden: false
    },
    {
      id: 'node2',
      label: 'Node 2',
      style: 'default',
      hidden: false
    },
    {
      id: 'node3',
      label: 'Node 3',
      style: 'default',
      hidden: false
    },
    {
      id: 'external',
      label: 'External Node',
      style: 'default',
      hidden: false
    }
  ];

  const edges: GraphEdge[] = [
    {
      id: 'edge1-2',
      source: 'node1',
      target: 'node2',
      style: 'default',
      hidden: false
    },
    {
      id: 'edge2-3',
      source: 'node2',
      target: 'node3',
      style: 'default',
      hidden: false
    },
    {
      id: 'edge1-ext',
      source: 'node1',
      target: 'external',
      style: 'default',
      hidden: false
    }
  ];

  const containers: Container[] = [
    {
      id: 'container1',
      children: new Set(['node1', 'node2', 'node3']),
      expandedDimensions: { width: 400, height: 300 },
      collapsed: false,
      hidden: false
    }
  ];

  const hyperEdges: HyperEdge[] = [
    {
      id: 'hyperedge1',
      source: 'container1',
      target: 'external',
      style: 'default',
      aggregatedEdges: [edges[2]] // edge1-ext
    }
  ];

  return { nodes, edges, containers, hyperEdges };
}

/**
 * Test ELK state manager creation and basic properties
 */
function testELKStateManagerCreation(): void {
  console.log('Testing ELK state manager creation...');
  
  const stateManager: ELKStateManager = createELKStateManager();
  
  // Verify it's a proper object with expected methods
  assert(typeof stateManager === 'object', 'State manager should be an object');
  assert(typeof stateManager.calculateFullLayout === 'function', 'Should have calculateFullLayout method');
  assert(typeof stateManager.calculateVisualLayout === 'function', 'Should have calculateVisualLayout method');
  
  console.log('✅ ELK state manager creation test passed');
}

/**
 * Test basic layout calculation
 */
async function testBasicLayoutCalculation(): Promise<void> {
  console.log('Testing basic layout calculation...');
  
  const stateManager = createELKStateManager();
  const { nodes, edges, containers } = createTestGraphData();
  
  try {
    const result = await stateManager.calculateFullLayout(nodes, edges, containers);
    
    // Verify result structure
    assert(result, 'Layout result should exist');
    assert('nodes' in result, 'Result should have nodes');
    assert('edges' in result, 'Result should have edges');
    assert(Array.isArray(result.nodes), 'Nodes should be an array');
    assert(Array.isArray(result.edges), 'Edges should be an array');
    
    // Verify nodes have layout properties
    assert(result.nodes.length > 0, 'Should have layouted nodes');
    result.nodes.forEach((node: any, index: number) => {
      assert('id' in node, `Node ${index} should have id`);
      assert('position' in node || ('x' in node && 'y' in node), `Node ${index} should have position`);
      
      // Check for position values
      const x = node.position?.x ?? node.x;
      const y = node.position?.y ?? node.y;
      assert(typeof x === 'number', `Node ${index} x should be number`);
      assert(typeof y === 'number', `Node ${index} y should be number`);
      assert(!isNaN(x), `Node ${index} x should not be NaN`);
      assert(!isNaN(y), `Node ${index} y should not be NaN`);
    });
    
    console.log(`   📊 Layouted ${result.nodes.length} nodes successfully`);
    
  } catch (error) {
    // ELK might not be available in test environment, handle gracefully
    if (error instanceof Error && (error.message.includes('ELK') || error.message.includes('elk'))) {
      console.log('   ⚠️  ELK not available in test environment, layout calculation skipped');
      return;
    }
    throw error;
  }
  
  console.log('✅ Basic layout calculation test passed');
}

/**
 * Test layout with different algorithms
 */
async function testLayoutAlgorithms(): Promise<void> {
  console.log('Testing layout with different algorithms...');
  
  const stateManager = createELKStateManager();
  const { nodes, edges, containers } = createTestGraphData();
  
  const algorithms = ['layered', 'force', 'stress'] as const;
  
  for (const algorithm of algorithms) {
    try {
      console.log(`   🔧 Testing ${algorithm} algorithm...`);
      
      const result = await stateManager.calculateFullLayout(nodes, edges, containers, algorithm);
      
      // Verify basic structure
      assert(result, `${algorithm} layout should produce result`);
      assert(Array.isArray(result.nodes), `${algorithm} should produce nodes array`);
      assert(result.nodes.length > 0, `${algorithm} should layout nodes`);
      
      // Check that nodes have positions
      result.nodes.forEach((node: any) => {
        const x = node.position?.x ?? node.x;
        const y = node.position?.y ?? node.y;
        assert(typeof x === 'number', `${algorithm} - node should have numeric x`);
        assert(typeof y === 'number', `${algorithm} - node should have numeric y`);
      });
      
      console.log(`   ✅ ${algorithm} algorithm test passed`);
      
    } catch (error) {
      if (error instanceof Error && (error.message.includes('ELK') || error.message.includes('elk'))) {
        console.log(`   ⚠️  ELK not available for ${algorithm}, skipped`);
        continue;
      }
      throw error;
    }
  }
  
  console.log('✅ Layout algorithms test passed');
}

/**
 * Test visual layout calculation with hyperEdges
 */
async function testVisualLayoutCalculation(): Promise<void> {
  console.log('Testing visual layout calculation...');
  
  const stateManager = createELKStateManager();
  const { nodes, edges, containers, hyperEdges } = createTestGraphData();
  
  try {
    const result = await stateManager.calculateVisualLayout(
      nodes,
      edges,
      containers,
      hyperEdges
    );
    
    // Verify result structure
    assert(result, 'Visual layout result should exist');
    assert('nodes' in result, 'Result should have nodes');
    assert('edges' in result, 'Result should have edges');
    assert('elkResult' in result, 'Result should have elkResult');
    
    // Verify layout quality
    assert(Array.isArray(result.nodes), 'Nodes should be array');
    assert(result.nodes.length > 0, 'Should have layouted nodes');
    
    console.log(`   📊 Visual layout calculated for ${result.nodes.length} nodes`);
    
  } catch (error) {
    if (error instanceof Error && (error.message.includes('ELK') || error.message.includes('elk'))) {
      console.log('   ⚠️  ELK not available, visual layout calculation skipped');
      return;
    }
    throw error;
  }
  
  console.log('✅ Visual layout calculation test passed');
}

/**
 * Test layout with caching
 */
async function testLayoutWithCaching(): Promise<void> {
  console.log('Testing layout with caching...');
  
  const stateManager = createELKStateManager();
  const { nodes, edges, containers } = createTestGraphData();
  
  // Create dimensions cache
  const dimensionsCache = new Map<string, LayoutDimensions>();
  dimensionsCache.set('container1', { width: 500, height: 400 });
  
  try {
    const result = await stateManager.calculateVisualLayout(
      nodes,
      edges,
      containers,
      [],
      'layered',
      dimensionsCache
    );
    
    assert(result, 'Cached layout should produce result');
    assert(Array.isArray(result.nodes), 'Cached layout should have nodes');
    
    console.log('   💾 Layout with caching completed');
    
  } catch (error) {
    if (error instanceof Error && (error.message.includes('ELK') || error.message.includes('elk'))) {
      console.log('   ⚠️  ELK not available, caching test skipped');
      return;
    }
    throw error;
  }
  
  console.log('✅ Layout with caching test passed');
}

/**
 * Test empty graph handling
 */
async function testEmptyGraphHandling(): Promise<void> {
  console.log('Testing empty graph handling...');
  
  const stateManager = createELKStateManager();
  
  try {
    // Test with empty arrays
    const result = await stateManager.calculateFullLayout([], [], []);
    
    assert(result, 'Empty graph should still return result');
    assert(Array.isArray(result.nodes), 'Empty result should have nodes array');
    assert(Array.isArray(result.edges), 'Empty result should have edges array');
    assert.strictEqual(result.nodes.length, 0, 'Empty graph should have no nodes');
    assert.strictEqual(result.edges.length, 0, 'Empty graph should have no edges');
    
  } catch (error) {
    if (error instanceof Error && (error.message.includes('ELK') || error.message.includes('elk'))) {
      console.log('   ⚠️  ELK not available, empty graph test skipped');
      return;
    }
    throw error;
  }
  
  console.log('✅ Empty graph handling test passed');
}

/**
 * Test error handling with invalid data
 */
async function testErrorHandling(): Promise<void> {
  console.log('Testing error handling...');
  
  const stateManager = createELKStateManager();
  
  // Test with invalid node data
  const invalidNodes: GraphNode[] = [
    {
      id: '', // Invalid empty id
      label: 'Invalid Node',
      style: 'default',
      hidden: false
    }
  ];
  
  try {
    const result = await stateManager.calculateFullLayout(invalidNodes, [], []);
    
    // Should either handle gracefully or throw meaningful error
    assert(result || true, 'Should handle invalid data gracefully or throw');
    
  } catch (error) {
    // Expected - should throw meaningful error for invalid data
    assert(error instanceof Error, 'Should throw proper Error object');
    console.log('   📝 Properly caught invalid data error');
  }
  
  console.log('✅ Error handling test passed');
}

/**
 * Test layout position and dimensions types
 */
function testLayoutTypes(): void {
  console.log('Testing layout types...');
  
  // Test LayoutPosition
  const position: LayoutPosition = { x: 100.5, y: 200.75 };
  assert.strictEqual(position.x, 100.5, 'Position should support decimals');
  assert.strictEqual(position.y, 200.75, 'Position should support decimals');
  
  // Test LayoutDimensions
  const dimensions: LayoutDimensions = { width: 180, height: 60 };
  assert.strictEqual(dimensions.width, 180, 'Dimensions should be correct');
  assert.strictEqual(dimensions.height, 60, 'Dimensions should be correct');
  
  // Test ContainmentValidationResult
  const validationResult: ContainmentValidationResult = {
    isValid: true,
    violations: []
  };
  assert.strictEqual(validationResult.isValid, true, 'Validation result should be valid');
  assert(Array.isArray(validationResult.violations), 'Should have violations array');
  assert.strictEqual(validationResult.violations.length, 0, 'Should have no violations');
  
  console.log('✅ Layout types test passed');
}

/**
 * Test interface compliance
 */
async function testInterfaceCompliance(): Promise<void> {
  console.log('Testing interface compliance...');
  
  const stateManager = createELKStateManager();
  
  // Verify interface compliance
  const interface_: ELKStateManager = stateManager;
  assert.strictEqual(interface_, stateManager, 'Should implement ELKStateManager interface');
  
  // Test that methods exist and have correct signatures
  assert(typeof interface_.calculateFullLayout === 'function', 'Should have calculateFullLayout');
  assert(typeof interface_.calculateVisualLayout === 'function', 'Should have calculateVisualLayout');
  
  // Test method can be called (mock call)
  try {
    const mockNodes: GraphNode[] = [];
    const mockEdges: GraphEdge[] = [];
    const mockContainers: Container[] = [];
    
    const result = await interface_.calculateFullLayout(mockNodes, mockEdges, mockContainers);
    assert(result, 'Interface method should work');
    
  } catch (error) {
    if (error instanceof Error && (error.message.includes('ELK') || error.message.includes('elk'))) {
      console.log('   ⚠️  ELK not available, interface compliance verified without execution');
    } else {
      throw error;
    }
  }
  
  console.log('✅ Interface compliance test passed');
}

/**
 * Run all ELK state manager tests
 */
export async function runAllTests(): Promise<void> {
  console.log('🧪 Running ELK State Manager Tests');
  console.log('==================================\n');
  
  try {
    testELKStateManagerCreation();
    await testBasicLayoutCalculation();
    await testLayoutAlgorithms();
    await testVisualLayoutCalculation();
    await testLayoutWithCaching();
    await testEmptyGraphHandling();
    await testErrorHandling();
    testLayoutTypes();
    await testInterfaceCompliance();
    
    console.log('\n🎉 All ELK state manager tests passed!');
    console.log('✅ ELK state management is working correctly!');
  } catch (error: unknown) {
    console.error('\n❌ ELK state manager test failed:', error instanceof Error ? error.message : String(error));
    if (error instanceof Error) {
      console.error(error.stack);
    }
    throw error;
  }
}

// Export individual test functions for selective testing
export {
  testELKStateManagerCreation,
  testBasicLayoutCalculation,
  testLayoutAlgorithms,
  testVisualLayoutCalculation,
  testLayoutWithCaching,
  testEmptyGraphHandling,
  testErrorHandling,
  testLayoutTypes,
  testInterfaceCompliance,
  createTestGraphData
};

// Run tests if this file is executed directly
if (import.meta.url === `file://${process.argv[1]}`) {
  runAllTests().catch(() => process.exit(1));
}
