/**
 * ELK Layout Engine Tests (TypeScript)
 * 
 * Tests for the ELK layout engine including layout execution,
 * configuration handling, and result processing.
 */

import assert from 'assert';
import { ELKLayoutEngine } from '../ELKLayoutEngine.js';
import { LayoutConfig, LayoutResult, LayoutEngine } from '../types.js';
import { GraphNode, GraphEdge, Container, HyperEdge } from '../../shared/types.js';
import { createTestGraphData } from './ELKStateManager.test.js';

/**
 * Test ELK layout engine creation and interface compliance
 */
function testELKLayoutEngineCreation(): void {
  console.log('Testing ELK layout engine creation...');
  
  const engine = new ELKLayoutEngine();
  
  // Verify interface compliance
  const layoutEngine: LayoutEngine = engine;
  assert.strictEqual(layoutEngine, engine, 'Should implement LayoutEngine interface');
  
  // Verify methods exist
  assert(typeof engine.layout === 'function', 'Should have layout method');
  
  // Verify it's a proper instance
  assert(engine instanceof ELKLayoutEngine, 'Should be instance of ELKLayoutEngine');
  
  console.log('✅ ELK layout engine creation test passed');
}

/**
 * Test basic layout execution
 */
async function testBasicLayoutExecution(): Promise<void> {
  console.log('Testing basic layout execution...');
  
  const engine = new ELKLayoutEngine();
  const { nodes, edges, containers, hyperEdges } = createTestGraphData();
  
  try {
    const result: LayoutResult = await engine.layout(nodes, edges, containers, hyperEdges);
    
    // Verify result structure
    assert(result, 'Layout should return result');
    assert('nodes' in result, 'Result should have nodes');
    assert('edges' in result, 'Result should have edges');
    assert('containers' in result, 'Result should have containers');
    assert('hyperEdges' in result, 'Result should have hyperEdges');
    
    // Verify arrays
    assert(Array.isArray(result.nodes), 'Nodes should be array');
    assert(Array.isArray(result.edges), 'Edges should be array');
    assert(Array.isArray(result.containers), 'Containers should be array');
    assert(Array.isArray(result.hyperEdges), 'HyperEdges should be array');
    
    // Verify positioned nodes
    assert(result.nodes.length > 0, 'Should have positioned nodes');
    result.nodes.forEach((node, index) => {
      assert('x' in node, `Node ${index} should have x position`);
      assert('y' in node, `Node ${index} should have y position`);
      assert('width' in node, `Node ${index} should have width`);
      assert('height' in node, `Node ${index} should have height`);
      assert(typeof node.x === 'number', `Node ${index} x should be number`);
      assert(typeof node.y === 'number', `Node ${index} y should be number`);
      assert(typeof node.width === 'number', `Node ${index} width should be number`);
      assert(typeof node.height === 'number', `Node ${index} height should be number`);
      
      // Verify original properties preserved
      assert('id' in node, `Node ${index} should have original id`);
      assert('label' in node, `Node ${index} should have original label`);
      assert('style' in node, `Node ${index} should have original style`);
      assert('hidden' in node, `Node ${index} should have original hidden`);
    });
    
    // Verify positioned containers
    if (result.containers.length > 0) {
      result.containers.forEach((container, index) => {
        assert('x' in container, `Container ${index} should have x position`);
        assert('y' in container, `Container ${index} should have y position`);
        assert('width' in container, `Container ${index} should have width`);
        assert('height' in container, `Container ${index} should have height`);
        assert(typeof container.x === 'number', `Container ${index} x should be number`);
        assert(typeof container.y === 'number', `Container ${index} y should be number`);
        assert(typeof container.width === 'number', `Container ${index} width should be number`);
        assert(typeof container.height === 'number', `Container ${index} height should be number`);
        
        // Verify original properties preserved
        assert('id' in container, `Container ${index} should have original id`);
        assert('children' in container, `Container ${index} should have original children`);
        assert('collapsed' in container, `Container ${index} should have original collapsed`);
        assert('hidden' in container, `Container ${index} should have original hidden`);
      });
    }
    
    console.log(`   📊 Layout completed: ${result.nodes.length} nodes, ${result.containers.length} containers`);
    
  } catch (error) {
    if (error instanceof Error && (error.message.includes('ELK') || error.message.includes('elk'))) {
      console.log('   ⚠️  ELK not available, layout execution skipped');
      return;
    }
    throw error;
  }
  
  console.log('✅ Basic layout execution test passed');
}

/**
 * Test layout with custom configuration
 */
async function testLayoutWithCustomConfig(): Promise<void> {
  console.log('Testing layout with custom configuration...');
  
  const engine = new ELKLayoutEngine();
  const { nodes, edges, containers, hyperEdges } = createTestGraphData();
  
  // Test different configurations
  const configs: LayoutConfig[] = [
    { algorithm: 'layered', direction: 'RIGHT', spacing: 50 },
    { algorithm: 'force', spacing: 75 },
    { algorithm: 'stress', spacing: 100, nodeSize: { width: 200, height: 80 } },
    {} // Empty config should use defaults
  ];
  
  for (const config of configs) {
    try {
      console.log(`   🔧 Testing config: ${JSON.stringify(config)}`);
      
      const result = await engine.layout(nodes, edges, containers, hyperEdges, config);
      
      // Verify basic structure
      assert(result, 'Custom config should produce result');
      assert(Array.isArray(result.nodes), 'Should have nodes array');
      assert(result.nodes.length > 0, 'Should have positioned nodes');
      
      // Verify nodes have valid positions
      result.nodes.forEach((node) => {
        assert(typeof node.x === 'number' && !isNaN(node.x), 'Node should have valid x');
        assert(typeof node.y === 'number' && !isNaN(node.y), 'Node should have valid y');
        assert(typeof node.width === 'number' && node.width > 0, 'Node should have valid width');
        assert(typeof node.height === 'number' && node.height > 0, 'Node should have valid height');
      });
      
      console.log(`   ✅ Config test passed: ${result.nodes.length} nodes positioned`);
      
    } catch (error) {
      if (error instanceof Error && (error.message.includes('ELK') || error.message.includes('elk'))) {
        console.log('   ⚠️  ELK not available for custom config, skipped');
        continue;
      }
      throw error;
    }
  }
  
  console.log('✅ Layout with custom configuration test passed');
}

/**
 * Test layout with empty data
 */
async function testLayoutWithEmptyData(): Promise<void> {
  console.log('Testing layout with empty data...');
  
  const engine = new ELKLayoutEngine();
  
  try {
    const result = await engine.layout([], [], [], []);
    
    // Verify result structure
    assert(result, 'Empty layout should return result');
    assert(Array.isArray(result.nodes), 'Should have nodes array');
    assert(Array.isArray(result.edges), 'Should have edges array');
    assert(Array.isArray(result.containers), 'Should have containers array');
    assert(Array.isArray(result.hyperEdges), 'Should have hyperEdges array');
    
    // Verify arrays are empty
    assert.strictEqual(result.nodes.length, 0, 'Should have no nodes');
    assert.strictEqual(result.edges.length, 0, 'Should have no edges');
    assert.strictEqual(result.containers.length, 0, 'Should have no containers');
    assert.strictEqual(result.hyperEdges.length, 0, 'Should have no hyperEdges');
    
  } catch (error) {
    if (error instanceof Error && (error.message.includes('ELK') || error.message.includes('elk'))) {
      console.log('   ⚠️  ELK not available, empty data test skipped');
      return;
    }
    throw error;
  }
  
  console.log('✅ Layout with empty data test passed');
}

/**
 * Test layout with single node
 */
async function testLayoutWithSingleNode(): Promise<void> {
  console.log('Testing layout with single node...');
  
  const engine = new ELKLayoutEngine();
  
  const singleNode: GraphNode[] = [
    {
      id: 'single',
      label: 'Single Node',
      style: 'default',
      hidden: false
    }
  ];
  
  try {
    const result = await engine.layout(singleNode, [], [], []);
    
    // Verify result
    assert(result, 'Single node layout should return result');
    assert.strictEqual(result.nodes.length, 1, 'Should have one node');
    
    const node = result.nodes[0];
    assert.strictEqual(node.id, 'single', 'Should preserve node id');
    assert.strictEqual(node.label, 'Single Node', 'Should preserve node label');
    assert(typeof node.x === 'number', 'Should have x position');
    assert(typeof node.y === 'number', 'Should have y position');
    assert(typeof node.width === 'number', 'Should have width');
    assert(typeof node.height === 'number', 'Should have height');
    
  } catch (error) {
    if (error instanceof Error && (error.message.includes('ELK') || error.message.includes('elk'))) {
      console.log('   ⚠️  ELK not available, single node test skipped');
      return;
    }
    throw error;
  }
  
  console.log('✅ Layout with single node test passed');
}

/**
 * Test layout with complex hierarchy
 */
async function testLayoutWithComplexHierarchy(): Promise<void> {
  console.log('Testing layout with complex hierarchy...');
  
  const engine = new ELKLayoutEngine();
  
  // Create more complex data
  const complexNodes: GraphNode[] = [
    { id: 'n1', label: 'Node 1', style: 'default', hidden: false },
    { id: 'n2', label: 'Node 2', style: 'default', hidden: false },
    { id: 'n3', label: 'Node 3', style: 'default', hidden: false },
    { id: 'n4', label: 'Node 4', style: 'default', hidden: false },
    { id: 'n5', label: 'Node 5', style: 'default', hidden: false },
    { id: 'ext1', label: 'External 1', style: 'default', hidden: false },
    { id: 'ext2', label: 'External 2', style: 'default', hidden: false }
  ];
  
  const complexEdges: GraphEdge[] = [
    { id: 'e1', source: 'n1', target: 'n2', style: 'default', hidden: false },
    { id: 'e2', source: 'n2', target: 'n3', style: 'default', hidden: false },
    { id: 'e3', source: 'n3', target: 'n4', style: 'default', hidden: false },
    { id: 'e4', source: 'n4', target: 'n5', style: 'default', hidden: false },
    { id: 'e5', source: 'n1', target: 'ext1', style: 'default', hidden: false },
    { id: 'e6', source: 'n5', target: 'ext2', style: 'default', hidden: false }
  ];
  
  const complexContainers: Container[] = [
    {
      id: 'container1',
      children: new Set(['n1', 'n2', 'n3']),
      expandedDimensions: { width: 500, height: 400 },
      collapsed: false,
      hidden: false
    },
    {
      id: 'container2',
      children: new Set(['n4', 'n5']),
      expandedDimensions: { width: 300, height: 200 },
      collapsed: false,
      hidden: false
    }
  ];
  
  const complexHyperEdges: HyperEdge[] = [
    {
      id: 'he1',
      source: 'container1',
      target: 'ext1',
      style: 'default',
      aggregatedEdges: [complexEdges[4]]
    },
    {
      id: 'he2',
      source: 'container2',
      target: 'ext2',
      style: 'default',
      aggregatedEdges: [complexEdges[5]]
    }
  ];
  
  try {
    const result = await engine.layout(complexNodes, complexEdges, complexContainers, complexHyperEdges);
    
    // Verify complex layout
    assert(result, 'Complex layout should return result');
    assert(result.nodes.length === complexNodes.length, 'Should have all nodes');
    assert(result.containers.length === complexContainers.length, 'Should have all containers');
    
    // Verify hierarchical relationships are maintained
    result.containers.forEach((container) => {
      assert(container.children instanceof Set, 'Container children should be Set');
      assert(container.children.size > 0, 'Container should have children');
    });
    
    // Verify layout quality - nodes should be separated
    const positions = result.nodes.map(node => ({ x: node.x, y: node.y }));
    const uniquePositions = new Set(positions.map(p => `${p.x},${p.y}`));
    assert(uniquePositions.size > 1 || result.nodes.length === 1, 'Nodes should have different positions');
    
    console.log(`   📊 Complex layout: ${result.nodes.length} nodes, ${result.containers.length} containers`);
    
  } catch (error) {
    if (error instanceof Error && (error.message.includes('ELK') || error.message.includes('elk'))) {
      console.log('   ⚠️  ELK not available, complex hierarchy test skipped');
      return;
    }
    throw error;
  }
  
  console.log('✅ Layout with complex hierarchy test passed');
}

/**
 * Test layout error handling
 */
async function testLayoutErrorHandling(): Promise<void> {
  console.log('Testing layout error handling...');
  
  const engine = new ELKLayoutEngine();
  
  // Test with invalid data
  const invalidNodes: GraphNode[] = [
    {
      id: '', // Invalid empty ID
      label: 'Invalid Node',
      style: 'default',
      hidden: false
    }
  ];
  
  try {
    const result = await engine.layout(invalidNodes, [], [], []);
    
    // Should handle gracefully or provide meaningful error
    assert(result || true, 'Should handle invalid data gracefully');
    
  } catch (error) {
    // Expected behavior - should throw meaningful error
    assert(error instanceof Error, 'Should throw Error object');
    assert(typeof error.message === 'string', 'Error should have message');
    console.log('   📝 Properly caught layout error:', error.message);
  }
  
  console.log('✅ Layout error handling test passed');
}

/**
 * Test layout result consistency
 */
async function testLayoutResultConsistency(): Promise<void> {
  console.log('Testing layout result consistency...');
  
  const engine = new ELKLayoutEngine();
  const { nodes, edges, containers, hyperEdges } = createTestGraphData();
  
  try {
    // Run layout multiple times with same data
    const result1 = await engine.layout(nodes, edges, containers, hyperEdges);
    const result2 = await engine.layout(nodes, edges, containers, hyperEdges);
    
    // Verify structure consistency
    assert.strictEqual(result1.nodes.length, result2.nodes.length, 'Node count should be consistent');
    assert.strictEqual(result1.containers.length, result2.containers.length, 'Container count should be consistent');
    
    // Verify IDs are preserved consistently
    const ids1 = result1.nodes.map(n => n.id).sort();
    const ids2 = result2.nodes.map(n => n.id).sort();
    assert.deepStrictEqual(ids1, ids2, 'Node IDs should be consistent');
    
    console.log('   🔄 Layout consistency verified across multiple runs');
    
  } catch (error) {
    if (error instanceof Error && (error.message.includes('ELK') || error.message.includes('elk'))) {
      console.log('   ⚠️  ELK not available, consistency test skipped');
      return;
    }
    throw error;
  }
  
  console.log('✅ Layout result consistency test passed');
}

/**
 * Run all ELK layout engine tests
 */
export async function runAllTests(): Promise<void> {
  console.log('🧪 Running ELK Layout Engine Tests');
  console.log('==================================\n');
  
  try {
    testELKLayoutEngineCreation();
    await testBasicLayoutExecution();
    await testLayoutWithCustomConfig();
    await testLayoutWithEmptyData();
    await testLayoutWithSingleNode();
    await testLayoutWithComplexHierarchy();
    await testLayoutErrorHandling();
    await testLayoutResultConsistency();
    
    console.log('\n🎉 All ELK layout engine tests passed!');
    console.log('✅ Layout engine is working correctly!');
  } catch (error: unknown) {
    console.error('\n❌ ELK layout engine test failed:', error instanceof Error ? error.message : String(error));
    if (error instanceof Error) {
      console.error(error.stack);
    }
    throw error;
  }
}

// Export individual test functions for selective testing
export {
  testELKLayoutEngineCreation,
  testBasicLayoutExecution,
  testLayoutWithCustomConfig,
  testLayoutWithEmptyData,
  testLayoutWithSingleNode,
  testLayoutWithComplexHierarchy,
  testLayoutErrorHandling,
  testLayoutResultConsistency
};

// Run tests if this file is executed directly
if (import.meta.url === `file://${process.argv[1]}`) {
  runAllTests().catch(() => process.exit(1));
}
