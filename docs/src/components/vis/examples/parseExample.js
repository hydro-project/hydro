/**
 * Example: Using the JSON Parser with Real Hydro Data
 * 
 * Demonstrates how to parse Hydro graph JSON and work with the resulting VisualizationState
 */

import { parseHydroGraphJSON, getAvailableGroupings, validateHydroGraphJSON } from '../JSONParser.js';
import { readFile } from 'fs/promises';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));

async function loadAndParseExample() {
  try {
    console.log('🔄 Loading example data...');
    
    // Load the chat.json test data
    const chatJsonPath = join(__dirname, '../../visualizer/test-data/chat.json');
    const chatData = await readFile(chatJsonPath, 'utf-8');
    
    console.log('✅ Data loaded successfully');
    
    // Validate the JSON first
    console.log('\n📋 Validating JSON...');
    const validation = validateHydroGraphJSON(chatData);
    
    if (!validation.isValid) {
      console.error('❌ Validation failed:', validation.errors);
      return;
    }
    
    console.log(`✅ Validation passed:`);
    console.log(`   - Nodes: ${validation.nodeCount}`);
    console.log(`   - Edges: ${validation.edgeCount}`);
    console.log(`   - Hierarchies: ${validation.hierarchyCount}`);
    
    if (validation.warnings.length > 0) {
      console.log(`⚠️  Warnings:`, validation.warnings);
    }
    
    // Show available groupings
    console.log('\n📊 Available groupings:');
    const groupings = getAvailableGroupings(chatData);
    groupings.forEach((grouping, index) => {
      console.log(`   ${index + 1}. ${grouping.name} (${grouping.id})`);
    });
    
    // Parse with different groupings
    for (const grouping of groupings) {
      console.log(`\n🔧 Parsing with "${grouping.name}" grouping...`);
      
      const result = parseHydroGraphJSON(chatData, grouping.id);
      const { state, metadata } = result;
      
      console.log(`✅ Parsed successfully:`);
      console.log(`   - Visible nodes: ${state.getVisibleNodes().length}`);
      console.log(`   - Visible edges: ${state.getVisibleEdges().length}`);
      console.log(`   - Containers: ${state.getVisibleContainers().length}`);
      console.log(`   - Selected grouping: ${metadata.selectedGrouping}`);
      
      // Show container details
      const containers = state.getVisibleContainers();
      if (containers.length > 0) {
        console.log('   📦 Containers:');
        containers.forEach(container => {
          const childCount = state.getContainerChildren(container.id).size;
          console.log(`      - ${container.label} (${container.id}): ${childCount} children`);
        });
      }
      
      // Show sample node assignments
      console.log('   🎯 Sample node assignments:');
      const sampleNodes = state.getVisibleNodes().slice(0, 3);
      sampleNodes.forEach(node => {
        const containerId = state.getNodeContainer(node.id);
        const containerName = containerId ? state.getContainer(containerId)?.label : 'None';
        console.log(`      - Node "${node.label}" (${node.id}) → ${containerName || 'No container'}`);
      });
      
      // Test collapse/expand
      if (containers.length > 0) {
        console.log(`\n🔄 Testing collapse/expand with container: ${containers[0].label}`);
        
        const beforeNodes = state.getVisibleNodes().length;
        const beforeEdges = state.getVisibleEdges().length;
        
        // Collapse
        state.collapseContainer(containers[0].id);
        const afterCollapseNodes = state.getVisibleNodes().length;
        const afterCollapseEdges = state.getVisibleEdges().length;
        const hyperEdges = state.getHyperEdges().length;
        
        console.log(`   📉 After collapse:`);
        console.log(`      - Visible nodes: ${beforeNodes} → ${afterCollapseNodes}`);
        console.log(`      - Visible edges: ${beforeEdges} → ${afterCollapseEdges}`);
        console.log(`      - HyperEdges created: ${hyperEdges}`);
        
        // Expand
        state.expandContainer(containers[0].id);
        const afterExpandNodes = state.getVisibleNodes().length;
        const afterExpandEdges = state.getVisibleEdges().length;
        const afterExpandHyperEdges = state.getHyperEdges().length;
        
        console.log(`   📈 After expand:`);
        console.log(`      - Visible nodes: ${afterCollapseNodes} → ${afterExpandNodes}`);
        console.log(`      - Visible edges: ${afterCollapseEdges} → ${afterExpandEdges}`);
        console.log(`      - HyperEdges remaining: ${afterExpandHyperEdges}`);
      }
    }
    
    console.log('\n🎉 Example completed successfully!');
    
  } catch (error) {
    console.error('❌ Error in example:', error.message);
    console.error(error.stack);
  }
}

// Run the example
if (import.meta.url === `file://${process.argv[1]}`) {
  loadAndParseExample();
}

export { loadAndParseExample };
