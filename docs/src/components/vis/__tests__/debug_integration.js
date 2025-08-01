/**
 * Debug script to reproduce the integration test failure
 */

import assert from 'assert';
import { readFile } from 'fs/promises';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { parseHydroGraphJSON } from '../dist/JSONParser.js';
import { InvariantChecker } from './fuzzTest.js';

const __dirname = dirname(fileURLToPath(import.meta.url));

async function debugIntegrationFailure() {
  console.log('🔍 Debugging integration test failure...');
  
  // Load the chat.json test data
  const filePath = join(__dirname, '../test-data', 'chat.json');
  const jsonData = await readFile(filePath, 'utf-8');
  
  // Parse with Location grouping (the one that seems to be failing)
  const result = parseHydroGraphJSON(jsonData, 'Location');
  const { state } = result;
  
  console.log('Initial state:');
  console.log(`  Nodes: ${state.getVisibleNodes().length}`);
  console.log(`  Edges: ${state.getVisibleEdges().length}`);
  console.log(`  Containers: ${state.getVisibleContainers().length}`);
  console.log(`  HyperEdges: ${state.getHyperEdges().length}`);
  
  // Get containers to collapse
  const containers = state.getVisibleContainers();
  console.log('\nContainers:');
  containers.forEach(container => {
    const children = Array.from(container.children);
    console.log(`  ${container.id}: ${container.children.size} children - [${children.join(', ')}]`);
  });
  
  // Try collapsing each container and check the state
  for (let i = 0; i < containers.length; i++) {
    const container = containers[i];
    console.log(`\n🔽 Collapsing container: ${container.id}`);
    
    state.collapseContainer(container.id);
    
    console.log('After collapse:');
    console.log(`  Nodes: ${state.getVisibleNodes().length}`);
    console.log(`  Edges: ${state.getVisibleEdges().length}`);
    console.log(`  Containers: ${state.getVisibleContainers().length}`);
    console.log(`  HyperEdges: ${state.getHyperEdges().length}`);
    
    // Run the invariant checker
    console.log('\n🔍 Running invariant checker after collapse...');
    const checker = new InvariantChecker(state);
    try {
      checker.checkAll(`After collapsing ${container.id}`);
      console.log('✅ All invariants passed');
    } catch (error) {
      console.log('❌ Invariant check failed:');
      console.log(error.message);
      break; // Stop on failure
    }
  }
  
  // Now test expanding
  console.log('\n🔼 Testing expansion...');
  for (let i = 0; i < containers.length; i++) {
    const container = containers[i];
    console.log(`\n🔼 Expanding container: ${container.id}`);
    
    state.expandContainer(container.id);
    
    console.log('After expansion:');
    console.log(`  Nodes: ${state.getVisibleNodes().length}`);
    console.log(`  Edges: ${state.getVisibleEdges().length}`);
    console.log(`  Containers: ${state.getVisibleContainers().length}`);
    console.log(`  HyperEdges: ${state.getHyperEdges().length}`);
    
    // Run the invariant checker
    console.log('\n🔍 Running invariant checker after expansion...');
    const checker = new InvariantChecker(state);
    try {
      checker.checkAll(`After expanding ${container.id}`);
      console.log('✅ All invariants passed');
    } catch (error) {
      console.log('❌ Invariant check failed:');
      console.log(error.message);
      
      // Let's examine the problematic edge
      const visibleEdges = state.getVisibleEdges();
      const visibleNodes = state.getVisibleNodes();
      console.log('\nVisible edges:');
      visibleEdges.forEach(edge => {
        const sourceNode = state.getGraphNode(edge.source);
        const targetNode = state.getGraphNode(edge.target);
        console.log(`  ${edge.id}: ${edge.source} -> ${edge.target}, source visible: ${sourceNode && !sourceNode.hidden}, target visible: ${targetNode && !targetNode.hidden}`);
      });
      
      break; // Stop on failure
    }
  }
}

// Run the debug
debugIntegrationFailure().catch(console.error);
