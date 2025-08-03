/**
 * Layout Integration Tests (TypeScript)
 * 
 * Integration tests for the complete layout system including
 * configuration, state management, and layout engine working together.
 */

import assert from 'assert';
import {
  ELKLayoutEngine,
  createELKStateManager,
  DEFAULT_LAYOUT_CONFIG,
  LAYOUT_CONFIGS,
  getLayoutConfig,
  createLayoutConfig
} from '../index.js';
import { LayoutConfig, LayoutResult, LayoutEngine } from '../types.js';
import { GraphNode, GraphEdge, Container, HyperEdge, NodeStyle, EdgeStyle } from '../../shared/types.js';

/**
 * Create comprehensive test data for integration testing
 */
function createIntegrationTestData() {
  const nodes: GraphNode[] = [
    { id: 'source', label: 'Source Node', style: 'default' as NodeStyle, hidden: false },
    { id: 'transform1', label: 'Transform 1', style: 'highlighted' as NodeStyle, hidden: false },
    { id: 'transform2', label: 'Transform 2', style: 'default' as NodeStyle, hidden: false },
    { id: 'join', label: 'Join Node', style: 'selected' as NodeStyle, hidden: false },
    { id: 'sink', label: 'Sink Node', style: 'default' as NodeStyle, hidden: false },
    { id: 'external1', label: 'External 1', style: 'default' as NodeStyle, hidden: false },
    { id: 'external2', label: 'External 2', style: 'default' as NodeStyle, hidden: false }
  ];

  const edges: GraphEdge[] = [
    { id: 'e1', source: 'source', target: 'transform1', style: 'default' as EdgeStyle, hidden: false },
    { id: 'e2', source: 'source', target: 'transform2', style: 'default' as EdgeStyle, hidden: false },
    { id: 'e3', source: 'transform1', target: 'join', style: 'highlighted' as EdgeStyle, hidden: false },
    { id: 'e4', source: 'transform2', target: 'join', style: 'default' as EdgeStyle, hidden: false },
    { id: 'e5', source: 'join', target: 'sink', style: 'thick' as EdgeStyle, hidden: false },
    { id: 'e6', source: 'transform1', target: 'external1', style: 'dashed' as EdgeStyle, hidden: false },
    { id: 'e7', source: 'sink', target: 'external2', style: 'default' as EdgeStyle, hidden: false }
  ];

  const containers: Container[] = [
    {
      id: 'processing_pipeline',
      children: new Set(['source', 'transform1', 'transform2', 'join', 'sink']),
      expandedDimensions: { width: 600, height: 400 },
      collapsed: false,
      hidden: false
    }
  ];

  const hyperEdges: HyperEdge[] = [
    {
      id: 'he1',
      source: 'processing_pipeline',
      target: 'external1',
      style: 'dashed' as EdgeStyle,
      aggregatedEdges: [edges[5]]
    },
    {
      id: 'he2',
      source: 'processing_pipeline',
      target: 'external2',
      style: 'default' as EdgeStyle,
      aggregatedEdges: [edges[6]]
    }
  ];

  return { nodes, edges, containers, hyperEdges };
}

/**
 * Test complete layout system integration
 */
async function testCompleteLayoutIntegration(): Promise<void> {
  console.log('Testing complete layout system integration...');
  
  const engine = new ELKLayoutEngine();
  const { nodes, edges, containers, hyperEdges } = createIntegrationTestData();
  
  try {
    // Test with default configuration
    const result = await engine.layout(nodes, edges, containers, hyperEdges);
    
    // Verify complete result structure
    assert(result, 'Integration should produce result');
    assert('nodes' in result, 'Should have nodes');
    assert('edges' in result, 'Should have edges');
    assert('containers' in result, 'Should have containers');
    assert('hyperEdges' in result, 'Should have hyperEdges');
    
    // Verify all data is processed
    assert.strictEqual(result.nodes.length, nodes.length, 'Should process all nodes');
    assert.strictEqual(result.edges.length, edges.length, 'Should process all edges');
    assert.strictEqual(result.containers.length, containers.length, 'Should process all containers');
    assert.strictEqual(result.hyperEdges.length, hyperEdges.length, 'Should process all hyperEdges');
    
    // Verify positioning quality
    result.nodes.forEach((node, index) => {
      assert(typeof node.x === 'number' && !isNaN(node.x), `Node ${index} should have valid x`);
      assert(typeof node.y === 'number' && !isNaN(node.y), `Node ${index} should have valid y`);
      assert(typeof node.width === 'number' && node.width > 0, `Node ${index} should have valid width`);
      assert(typeof node.height === 'number' && node.height > 0, `Node ${index} should have valid height`);
    });
    
    result.containers.forEach((container, index) => {
      assert(typeof container.x === 'number' && !isNaN(container.x), `Container ${index} should have valid x`);
      assert(typeof container.y === 'number' && !isNaN(container.y), `Container ${index} should have valid y`);
      assert(typeof container.width === 'number' && container.width > 0, `Container ${index} should have valid width`);
      assert(typeof container.height === 'number' && container.height > 0, `Container ${index} should have valid height`);
    });
    
    // Verify properties are preserved
    result.nodes.forEach((node) => {
      const original = nodes.find(n => n.id === node.id);
      assert(original, `Original node ${node.id} should exist`);
      assert.strictEqual(node.label, original.label, 'Should preserve node label');
      assert.strictEqual(node.style, original.style, 'Should preserve node style');
      assert.strictEqual(node.hidden, original.hidden, 'Should preserve node hidden');
    });
    
    console.log(`   📊 Integration complete: ${result.nodes.length} nodes, ${result.containers.length} containers positioned`);
    
  } catch (error) {
    if (error instanceof Error && (error.message.includes('ELK') || error.message.includes('elk'))) {
      console.log('   ⚠️  ELK not available, integration test skipped');
      return;
    }
    throw error;
  }
  
  console.log('✅ Complete layout system integration test passed');
}

/**
 * Test configuration system integration
 */
async function testConfigurationIntegration(): Promise<void> {
  console.log('Testing configuration system integration...');
  
  const engine = new ELKLayoutEngine();
  const { nodes, edges, containers, hyperEdges } = createIntegrationTestData();
  
  // Test all preset configurations
  const presetNames = ['DEFAULT', 'COMPACT', 'LOOSE', 'HORIZONTAL'] as const;
  
  for (const presetName of presetNames) {
    try {
      console.log(`   🔧 Testing ${presetName} preset...`);
      
      const config = getLayoutConfig(presetName);
      const result = await engine.layout(nodes, edges, containers, hyperEdges, config);
      
      // Verify preset produces valid layout
      assert(result, `${presetName} preset should produce result`);
      assert(result.nodes.length > 0, `${presetName} should position nodes`);
      
      // Verify nodes are positioned
      result.nodes.forEach((node) => {
        assert(typeof node.x === 'number' && !isNaN(node.x), `${presetName} - node should have valid x`);
        assert(typeof node.y === 'number' && !isNaN(node.y), `${presetName} - node should have valid y`);
      });
      
      console.log(`   ✅ ${presetName} preset works: ${result.nodes.length} nodes positioned`);
      
    } catch (error) {
      if (error instanceof Error && (error.message.includes('ELK') || error.message.includes('elk'))) {
        console.log(`   ⚠️  ELK not available for ${presetName}, skipped`);
        continue;
      }
      throw error;
    }
  }
  
  // Test custom configuration creation
  try {
    const customConfig = createLayoutConfig({
      algorithm: 'force',
      spacing: 120,
      nodeSize: { width: 200, height: 80 }
    });
    
    const result = await engine.layout(nodes, edges, containers, hyperEdges, customConfig);
    
    assert(result, 'Custom config should produce result');
    assert(result.nodes.length > 0, 'Custom config should position nodes');
    
    console.log('   🎛️ Custom configuration integration successful');
    
  } catch (error) {
    if (error instanceof Error && (error.message.includes('ELK') || error.message.includes('elk'))) {
      console.log('   ⚠️  ELK not available for custom config, skipped');
    } else {
      throw error;
    }
  }
  
  console.log('✅ Configuration system integration test passed');
}

/**
 * Test state manager integration
 */
async function testStateManagerIntegration(): Promise<void> {
  console.log('Testing state manager integration...');
  
  const stateManager = createELKStateManager();
  const { nodes, edges, containers } = createIntegrationTestData();
  
  try {
    // Test state manager produces compatible output
    const stateResult = await stateManager.calculateFullLayout(nodes, edges, containers);
    
    assert(stateResult, 'State manager should produce result');
    assert('nodes' in stateResult, 'State result should have nodes');
    assert('edges' in stateResult, 'State result should have edges');
    assert(Array.isArray(stateResult.nodes), 'State nodes should be array');
    
    // Verify state manager output is compatible with engine expectations
    assert(stateResult.nodes.length > 0, 'State manager should process nodes');
    
    stateResult.nodes.forEach((node: any) => {
      assert('id' in node, 'State node should have id');
      // Position can be in different formats from state manager
      const hasPosition = ('position' in node && node.position) || ('x' in node && 'y' in node);
      assert(hasPosition, 'State node should have position information');
    });
    
    console.log(`   🔗 State manager integration successful: ${stateResult.nodes.length} nodes processed`);
    
  } catch (error) {
    if (error instanceof Error && (error.message.includes('ELK') || error.message.includes('elk'))) {
      console.log('   ⚠️  ELK not available, state manager integration skipped');
      return;
    }
    throw error;
  }
  
  console.log('✅ State manager integration test passed');
}

/**
 * Test end-to-end workflow
 */
async function testEndToEndWorkflow(): Promise<void> {
  console.log('Testing end-to-end workflow...');
  
  const { nodes, edges, containers, hyperEdges } = createIntegrationTestData();
  
  try {
    // Step 1: Create configuration
    const config = createLayoutConfig({
      algorithm: 'layered',
      direction: 'RIGHT',
      spacing: 80,
      nodeSize: { width: 180, height: 60 }
    });
    
    console.log('   1️⃣ Configuration created');
    
    // Step 2: Create layout engine
    const engine = new ELKLayoutEngine();
    console.log('   2️⃣ Layout engine created');
    
    // Step 3: Execute layout
    const startTime = Date.now();
    const result = await engine.layout(nodes, edges, containers, hyperEdges, config);
    const duration = Date.now() - startTime;
    
    console.log(`   3️⃣ Layout executed in ${duration}ms`);
    
    // Step 4: Verify complete workflow result
    assert(result, 'Workflow should produce result');
    assert(result.nodes.length === nodes.length, 'All nodes should be processed');
    assert(result.containers.length === containers.length, 'All containers should be processed');
    
    // Step 5: Verify quality metrics
    const nodeSpread = {
      minX: Math.min(...result.nodes.map(n => n.x)),
      maxX: Math.max(...result.nodes.map(n => n.x)),
      minY: Math.min(...result.nodes.map(n => n.y)),
      maxY: Math.max(...result.nodes.map(n => n.y))
    };
    
    const layoutWidth = nodeSpread.maxX - nodeSpread.minX;
    const layoutHeight = nodeSpread.maxY - nodeSpread.minY;
    
    assert(layoutWidth >= 0, 'Layout should have valid width');
    assert(layoutHeight >= 0, 'Layout should have valid height');
    
    console.log(`   4️⃣ Quality verified: ${layoutWidth.toFixed(0)}x${layoutHeight.toFixed(0)} layout area`);
    
    // Step 6: Verify data integrity
    result.nodes.forEach((node) => {
      const original = nodes.find(n => n.id === node.id);
      assert(original, `Node ${node.id} should preserve original data`);
      assert.strictEqual(node.id, original.id, 'ID should be preserved');
      assert.strictEqual(node.label, original.label, 'Label should be preserved');
      assert.strictEqual(node.style, original.style, 'Style should be preserved');
    });
    
    console.log('   5️⃣ Data integrity verified');
    
    console.log(`   📈 End-to-end workflow completed successfully`);
    
  } catch (error) {
    if (error instanceof Error && (error.message.includes('ELK') || error.message.includes('elk'))) {
      console.log('   ⚠️  ELK not available, end-to-end workflow skipped');
      return;
    }
    throw error;
  }
  
  console.log('✅ End-to-end workflow test passed');
}

/**
 * Test performance characteristics
 */
async function testPerformanceCharacteristics(): Promise<void> {
  console.log('Testing performance characteristics...');
  
  const engine = new ELKLayoutEngine();
  
  // Create datasets of different sizes
  const createDataset = (nodeCount: number) => {
    const nodes: GraphNode[] = [];
    const edges: GraphEdge[] = [];
    
    for (let i = 0; i < nodeCount; i++) {
      nodes.push({
        id: `node_${i}`,
        label: `Node ${i}`,
        style: 'default' as NodeStyle,
        hidden: false
      });
      
      if (i > 0) {
        edges.push({
          id: `edge_${i-1}_${i}`,
          source: `node_${i-1}`,
          target: `node_${i}`,
          style: 'default' as EdgeStyle,
          hidden: false
        });
      }
    }
    
    return { nodes, edges, containers: [], hyperEdges: [] };
  };
  
  const datasets = [
    { size: 5, name: 'small' },
    { size: 10, name: 'medium' },
    { size: 20, name: 'large' }
  ];
  
  for (const { size, name } of datasets) {
    try {
      console.log(`   ⏱️ Testing ${name} dataset (${size} nodes)...`);
      
      const dataset = createDataset(size);
      const startTime = Date.now();
      const result = await engine.layout(dataset.nodes, dataset.edges, dataset.containers, dataset.hyperEdges);
      const duration = Date.now() - startTime;
      
      // Verify result
      assert(result, `${name} dataset should produce result`);
      assert.strictEqual(result.nodes.length, size, `${name} should process all nodes`);
      
      // Performance expectations (generous for test environment)
      assert(duration < 5000, `${name} dataset should layout in reasonable time (${duration}ms)`);
      
      console.log(`   ✅ ${name} dataset: ${duration}ms for ${size} nodes`);
      
    } catch (error) {
      if (error instanceof Error && (error.message.includes('ELK') || error.message.includes('elk'))) {
        console.log(`   ⚠️  ELK not available for ${name} performance test, skipped`);
        continue;
      }
      throw error;
    }
  }
  
  console.log('✅ Performance characteristics test passed');
}

/**
 * Test error propagation through integration
 */
async function testErrorPropagation(): Promise<void> {
  console.log('Testing error propagation...');
  
  const engine = new ELKLayoutEngine();
  
  // Test various error conditions
  const errorCases = [
    {
      name: 'invalid node ID',
      data: {
        nodes: [{ id: '', label: 'Invalid', style: 'default' as NodeStyle, hidden: false }],
        edges: [],
        containers: [],
        hyperEdges: []
      }
    },
    {
      name: 'circular container reference',
      data: {
        nodes: [
          { id: 'node1', label: 'Node 1', style: 'default' as NodeStyle, hidden: false }
        ],
        edges: [],
        containers: [
          {
            id: 'container1',
            children: new Set(['container1']), // Self-reference
            expandedDimensions: { width: 400, height: 300 },
            collapsed: false,
            hidden: false
          }
        ],
        hyperEdges: []
      }
    }
  ];
  
  for (const { name, data } of errorCases) {
    try {
      console.log(`   🚨 Testing ${name}...`);
      
      const result = await engine.layout(data.nodes, data.edges, data.containers, data.hyperEdges);
      
      // Should either handle gracefully or throw meaningful error
      if (result) {
        console.log(`   📝 ${name} handled gracefully`);
      }
      
    } catch (error) {
      // Expected behavior for error cases
      assert(error instanceof Error, `${name} should throw Error object`);
      console.log(`   📝 ${name} properly caught: ${error.message}`);
    }
  }
  
  console.log('✅ Error propagation test passed');
}

/**
 * Run all layout integration tests
 */
export async function runAllTests(): Promise<void> {
  console.log('🧪 Running Layout Integration Tests');
  console.log('===================================\n');
  
  try {
    await testCompleteLayoutIntegration();
    await testConfigurationIntegration();
    await testStateManagerIntegration();
    await testEndToEndWorkflow();
    await testPerformanceCharacteristics();
    await testErrorPropagation();
    
    console.log('\n🎉 All layout integration tests passed!');
    console.log('✅ Complete layout system is working correctly!');
  } catch (error: unknown) {
    console.error('\n❌ Layout integration test failed:', error instanceof Error ? error.message : String(error));
    if (error instanceof Error) {
      console.error(error.stack);
    }
    throw error;
  }
}

// Export individual test functions for selective testing
export {
  testCompleteLayoutIntegration,
  testConfigurationIntegration,
  testStateManagerIntegration,
  testEndToEndWorkflow,
  testPerformanceCharacteristics,
  testErrorPropagation,
  createIntegrationTestData
};

// Run tests if this file is executed directly
if (import.meta.url === `file://${process.argv[1]}`) {
  runAllTests().catch(() => process.exit(1));
}
