/**
 * Integration Tests for Vis Components
 * 
 * Tests the complete flow from JSON parsing through state management
 * using real Hydro graph data files.
 */

import assert from 'assert';
import { readFile } from 'fs/promises';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { parseHydroGraphJSON, validateHydroGraphJSON, getAvailableGroupings } from '../dist/core/JSONParser.js';
import { runFuzzTest, InvariantChecker } from './fuzzTest.js';

const __dirname = dirname(fileURLToPath(import.meta.url));

console.log('Running Integration tests...');

/**
 * Test basic JSON parsing with real data
 */
async function testRealDataParsing() {
  console.log('Testing real data parsing...');
  
  const testFiles = ['chat.json', 'paxos.json'];
  
  for (const filename of testFiles) {
    console.log(`  📁 Testing ${filename}...`);
    
    const filePath = join(__dirname, '../test-data', filename);
    const jsonData = await readFile(filePath, 'utf-8');
    
    // Validate
    const validation = validateHydroGraphJSON(jsonData);
    assert(validation.isValid, `${filename} should be valid: ${validation.errors.join(', ')}`);
    assert(validation.nodeCount > 0, `${filename} should have nodes`);
    
    // Get groupings
    const groupings = getAvailableGroupings(jsonData);
    console.log(`    📊 Found ${groupings.length} groupings: ${groupings.map(g => g.name).join(', ')}`);
    
    // Test each grouping
    for (const grouping of groupings) {
      console.log(`    🔧 Testing grouping: ${grouping.name}`);
      
      const result = parseHydroGraphJSON(jsonData, grouping.id);
      const { state, metadata } = result;
      
      // Basic structure checks
      assert(state.visibleNodes.length > 0, 'Should have visible nodes');
      assert.strictEqual(metadata.selectedGrouping, grouping.id, 'Should track selected grouping');
      
      // Check invariants
      const checker = new InvariantChecker(state);
      checker.checkAll(`${filename} - ${grouping.name} initial state`);
      
      // Test basic operations if containers exist
      const containers = state.visibleContainers;
      if (containers.length > 0) {
        console.log(`      📦 Testing with ${containers.length} containers`);
        
        // Try collapsing the first container
        const container = containers[0];
        const beforeNodes = state.visibleNodes.length;
        const beforeEdges = state.visibleEdges.length;
        
        state.collapseContainer(container.id);
        checker.checkAll(`${filename} - ${grouping.name} after collapse`);
        
        const afterNodes = state.visibleNodes.length;
        const afterEdges = state.visibleEdges.length;
        const hyperEdges = state.allHyperEdges.length;
        
        console.log(`        📉 Collapse: ${beforeNodes}→${afterNodes} nodes, ${beforeEdges}→${afterEdges} edges, +${hyperEdges} hyperEdges`);
        
        // Expand it back
        state.expandContainer(container.id);
        checker.checkAll(`${filename} - ${grouping.name} after expand`);
        
        const finalNodes = state.visibleNodes.length;
        const finalEdges = state.visibleEdges.length;
        const finalHyperEdges = state.allHyperEdges.length;
        
        console.log(`        📈 Expand: ${afterNodes}→${finalNodes} nodes, ${afterEdges}→${finalEdges} edges, ${finalHyperEdges} hyperEdges`);
        
        // Should be back to original state
        assert.strictEqual(finalNodes, beforeNodes, 'Should restore original node count');
        assert.strictEqual(finalEdges, beforeEdges, 'Should restore original edge count');
        assert.strictEqual(finalHyperEdges, 0, 'Should remove all hyperEdges');
      }
    }
    
    console.log(`  ✅ ${filename} passed all tests`);
  }
  
  console.log('✓ Real data parsing tests passed');
}

/**
 * Test edge cases and error handling
 */
async function testEdgeCases() {
  console.log('Testing edge cases...');
  
  // Test empty data
  try {
    parseHydroGraphJSON({ nodes: [], edges: [] });
    assert.fail('Should reject empty node data');
  } catch (error) {
    assert(error.message.includes('Invalid graph data'), 'Should give helpful error message');
  }
  
  // Test malformed JSON
  try {
    validateHydroGraphJSON('invalid json');
    // Should not throw, but should return invalid result
  } catch (error) {
    // JSON parsing errors are expected here
  }
  
  // Test data with no hierarchy
  const flatData = {
    nodes: [{ id: '1' }, { id: '2' }],
    edges: [{ id: 'e1', source: '1', target: '2' }]
  };
  
  const result = parseHydroGraphJSON(flatData);
  assert.strictEqual(result.state.visibleNodes.length, 2, 'Should handle flat data');
  assert.strictEqual(result.state.visibleContainers.length, 0, 'Should have no containers');
  assert.strictEqual(result.metadata.selectedGrouping, null, 'Should have no grouping');
  
  console.log('✓ Edge cases tests passed');
}

/**
 * Test performance with larger datasets
 */
async function testPerformance() {
  console.log('Testing performance...');
  
  const filePath = join(__dirname, '../test-data/chat.json');
  const jsonData = await readFile(filePath, 'utf-8');
  
  const startTime = Date.now();
  const result = parseHydroGraphJSON(jsonData);
  const parseTime = Date.now() - startTime;
  
  console.log(`  ⏱️  Parsing time: ${parseTime}ms`);
  assert(parseTime < 1000, 'Parsing should be fast (< 1s)');
  
  // Test state operations performance
  const state = result.state;
  const containers = state.visibleContainers;
  
  if (containers.length > 0) {
    const operationStart = Date.now();
    
    // Perform multiple operations
    for (let i = 0; i < Math.min(10, containers.length); i++) {
      state.collapseContainer(containers[i].id);
      state.expandContainer(containers[i].id);
    }
    
    const operationTime = Date.now() - operationStart;
    console.log(`  ⚡ Operation time (20 collapse/expand): ${operationTime}ms`);
    assert(operationTime < 500, 'Operations should be fast (< 500ms for 20 ops)');
  }
  
  console.log('✓ Performance tests passed');
}

/**
 * Test state consistency after complex operations
 */
async function testStateConsistency() {
  console.log('Testing state consistency...');
  
  const filePath = join(__dirname, '../test-data/chat.json');
  const jsonData = await readFile(filePath, 'utf-8');
  const result = parseHydroGraphJSON(jsonData);
  const state = result.state;
  const checker = new InvariantChecker(state);
  
  // Get initial counts
  const initialNodes = state.visibleNodes.length;
  const initialEdges = state.visibleEdges.length;
  const containers = state.visibleContainers;
  
  if (containers.length === 0) {
    console.log('  ⚠️  No containers, skipping consistency test');
    return;
  }
  
  console.log(`  📊 Initial: ${initialNodes} nodes, ${initialEdges} edges, ${containers.length} containers`);
  
  // Collapse all containers
  for (const container of containers) {
    state.collapseContainer(container.id);
    checker.checkAll(`After collapsing ${container.id}`);
  }
  
  const allCollapsedNodes = state.visibleNodes.length;
  const allCollapsedEdges = state.visibleEdges.length;
  const hyperEdges = state.allHyperEdges.length;
  
  console.log(`  📉 All collapsed: ${allCollapsedNodes} nodes, ${allCollapsedEdges} edges, ${hyperEdges} hyperEdges`);
  
  // Expand all containers
  for (const container of containers) {
    state.expandContainer(container.id);
    checker.checkAll(`After expanding ${container.id}`);
  }
  
  const finalNodes = state.visibleNodes.length;
  const finalEdges = state.visibleEdges.length;
  const finalHyperEdges = state.allHyperEdges.length;
  
  console.log(`  📈 All expanded: ${finalNodes} nodes, ${finalEdges} edges, ${finalHyperEdges} hyperEdges`);
  
  // Should be back to initial state
  assert.strictEqual(finalNodes, initialNodes, 'Should restore all nodes');
  assert.strictEqual(finalEdges, initialEdges, 'Should restore all edges');
  assert.strictEqual(finalHyperEdges, 0, 'Should have no hyperEdges');
  
  console.log('✓ State consistency tests passed');
}

/**
 * Run a quick fuzz test on real data
 */
async function testFuzzIntegration() {
  console.log('Testing fuzz integration...');
  
  const filePath = join(__dirname, '../test-data/chat.json');
  const jsonData = JSON.parse(await readFile(filePath, 'utf-8'));
  
  // Run a short fuzz test
  await runFuzzTest(jsonData, 'chat.json', null, 20); // 20 iterations instead of 100
  
  console.log('✓ Fuzz integration test passed');
}

/**
 * Run all integration tests
 */
async function runAllTests() {
  try {
    await testRealDataParsing();
    await testEdgeCases();
    await testPerformance();
    await testStateConsistency();
    await testFuzzIntegration();
    
    console.log('\n🎉 All integration tests passed! System is working correctly with real data.');
  } catch (error) {
    console.error('\n❌ Integration test failed:', error.message);
    console.error(error.stack);
    process.exit(1);
  }
}

// Export for use in other test files
export {
  testRealDataParsing,
  testEdgeCases,
  testPerformance,
  testStateConsistency,
  testFuzzIntegration,
  runAllTests
};

// Run tests if executed directly
if (import.meta.url === `file://${process.argv[1]}`) {
  runAllTests();
}
