/**
 * Debug Collapsed Containers - Find which containers are collapsed and why edges are missing hyperEdges
 */

import { describe, test, expect } from 'vitest';
import { parseGraphJSON } from '../core/JSONParser';
import { VisualizationEngine } from '../core/VisualizationEngine';

describe('Debug Collapsed Containers', () => {
  test('should debug which containers are collapsed and analyze missing hyperEdges', async () => {
    // Load the paxos data
    const mockJsonData = require('../test-data/paxos-flipped.json');
    const result = parseGraphJSON(mockJsonData, null);
    const state = result.state;
    
    // Create engine and run smart collapse
    const engine = new VisualizationEngine(state, {
      enableLogging: false, // Reduce noise
      layoutConfig: {
        enableSmartCollapse: true,
        algorithm: 'mrtree',
        direction: 'DOWN'
      }
    });
    
    await engine.runLayout();
    
    console.log('\n=== ANALYZING COLLAPSED CONTAINERS ===');
    
    // Find all collapsed containers
    const allContainers = state.getVisibleContainers();
    const collapsedContainers = allContainers.filter(c => c.collapsed);
    
    console.log(`Total containers: ${allContainers.length}`);
    console.log(`Collapsed containers: ${collapsedContainers.length}`);
    
    // Group containers by their collapse status
    const collapsedIds = collapsedContainers.map(c => c.id);
    console.log(`Collapsed container IDs: ${collapsedIds.join(', ')}`);
    
    // Now let's look at some specific failing edges to understand the issue
    const failedCases = [
      { edgeId: 'e376', containerId: 'bt_25' },
      { edgeId: 'e147', containerId: 'bt_138' },
      { edgeId: 'e7', containerId: 'bt_27' }
    ];
    
    console.log('\n=== ANALYZING SPECIFIC FAILED CASES ===');
    
    for (const { edgeId, containerId } of failedCases) {
      console.log(`\n--- Case: Edge ${edgeId} vs Container ${containerId} ---`);
      
      // Check if this container is actually collapsed
      const container = allContainers.find(c => c.id === containerId);
      if (!container) {
        console.log(`❌ Container ${containerId} not found!`);
        continue;
      }
      
      console.log(`Container ${containerId}: collapsed=${container.collapsed}, hidden=${container.hidden}`);
      
      // Check if this edge exists
      const edge = state.getVisibleEdges().find(e => e.id === edgeId);
      if (!edge) {
        console.log(`❌ Edge ${edgeId} not found in visible edges!`);
        // Try to find it in all edges by looking at the comprehensive test data approach
        console.log(`   Checking if edge exists but is hidden...`);
        continue;
      }
      
      console.log(`Edge ${edgeId}: ${edge.source} -> ${edge.target}, hidden=${edge.hidden}`);
      
      // Find hyperEdges that might represent this connectivity
      const hyperEdges = state.visibleHyperEdges.filter(he => 
        (he.source === containerId || he.target === containerId) ||
        (he.source === edge.source || he.target === edge.source) ||
        (he.source === edge.target || he.target === edge.target)
      );
      
      console.log(`Related hyperEdges: ${hyperEdges.length}`);
      hyperEdges.forEach(he => {
        console.log(`  HyperEdge ${he.id}: ${he.source} -> ${he.target}`);
      });
      
      // Check if the edge endpoints are visible or inside the collapsed container
      const sourceVisible = state.getVisibleNodes().some(n => n.id === edge.source);
      const targetVisible = state.getVisibleNodes().some(n => n.id === edge.target);
      console.log(`Source ${edge.source} visible: ${sourceVisible}, Target ${edge.target} visible: ${targetVisible}`);
    }
    
    console.log('\n=== HYPEREDGE SUMMARY ===');
    console.log(`Total hyperEdges: ${state.visibleHyperEdges.length}`);
    
    // Group hyperEdges by their endpoints to see patterns
    const hyperEdgesByContainer = new Map();
    for (const he of state.visibleHyperEdges) {
      const key = `${he.source}->${he.target}`;
      if (!hyperEdgesByContainer.has(key)) {
        hyperEdgesByContainer.set(key, []);
      }
      hyperEdgesByContainer.get(key).push(he);
    }
    
    console.log('HyperEdge connections:');
    for (const [connection, hyperEdges] of hyperEdgesByContainer) {
      console.log(`  ${connection}: ${hyperEdges.length} hyperEdges`);
    }
  });
});
