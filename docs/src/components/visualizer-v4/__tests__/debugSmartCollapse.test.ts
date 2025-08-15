/**
 * Debug Smart Collapse - Debug the missing hyperedge issue
 */

import { describe, test, expect } from 'vitest';
import { parseGraphJSON } from '../core/JSONParser';
import { VisualizationEngine } from '../core/VisualizationEngine';

describe('Debug Smart Collapse', () => {
  test('should debug missing hyperedges during smart collapse', async () => {
    // Load the paxos data that's causing issues
    const mockJsonData = require('../test-data/paxos-flipped.json');
    const result = parseGraphJSON(mockJsonData, null);
    const state = result.state;
    
    console.log('\n=== INITIAL STATE ===');
    console.log('Total nodes:', state.getVisibleNodes().length);
    console.log('Total edges:', state.getVisibleEdges().length);
    console.log('Total containers:', state.getVisibleContainers().length);
    console.log('Total hyperEdges:', state.visibleHyperEdges.length);
    
    // Print first few containers
    console.log('\n=== FIRST 5 CONTAINERS ===');
    const containers = state.getVisibleContainers();
    for (let i = 0; i < Math.min(5, containers.length); i++) {
      const container = containers[i];
      console.log(`Container ${container.id}: collapsed=${container.collapsed}, hidden=${container.hidden}`);
    }
    
    // Sample a few edges to see structure
    console.log('\n=== FIRST 5 EDGES ===');
    const edges = state.getVisibleEdges();
    for (let i = 0; i < Math.min(5, edges.length); i++) {
      const edge = edges[i];
      console.log(`Edge ${edge.id}: ${edge.source} -> ${edge.target}, hidden=${edge.hidden}`);
    }
    
    // Create engine to run smart collapse
    const engine = new VisualizationEngine(state, {
      enableLogging: true,
      layoutConfig: {
        enableSmartCollapse: true,
        algorithm: 'mrtree',
        direction: 'DOWN'
      }
    });
    
    console.log('\n=== RUNNING SMART COLLAPSE VIA ENGINE ===');
    await engine.runLayout();
    
    console.log('\n=== POST-SMART-COLLAPSE STATE ===');
    console.log('Total hyperEdges:', state.visibleHyperEdges.length);
    
    // Count how many containers are collapsed
    const collapsedContainers = state.getVisibleContainers().filter(c => c.collapsed);
    console.log(`Collapsed containers: ${collapsedContainers.length}`);
    
    // Try validation - we expect this to fail now
    try {
      state.validateInvariants();
      console.log('✅ Validation passed - no missing hyperEdges!');
    } catch (error) {
      const message = error.message;
      const missingCount = (message.match(/Edge \w+ crosses collapsed container/g) || []).length;
      console.log(`❌ Validation failed - ${missingCount} missing hyperEdges`);
      
      // Show first few error messages for analysis
      const errorLines = message.split(';').slice(0, 3);
      console.log('First few errors:');
      errorLines.forEach(line => console.log('  ', line.trim()));
    }
  });
});
