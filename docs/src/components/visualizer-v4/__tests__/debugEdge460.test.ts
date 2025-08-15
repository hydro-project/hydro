/**
 * Debug Edge e460 - Investigate the specific failing edge
 */

import { describe, test, expect } from 'vitest';
import { parseGraphJSON } from '../core/JSONParser';
import { VisualizationEngine } from '../core/VisualizationEngine';

describe('Debug Edge e460', () => {
  test('should investigate edge e460 and its relationship to containers bt_29 and bt_187', async () => {
    // Load the paxos data
    const mockJsonData = require('../test-data/paxos-flipped.json');
    const result = parseGraphJSON(mockJsonData, null);
    const state = result.state;
    
    // Create engine and run smart collapse
    const engine = new VisualizationEngine(state, {
      enableLogging: false,
      layoutConfig: {
        enableSmartCollapse: true,
        algorithm: 'mrtree',
        direction: 'DOWN'
      }
    });
    
    await engine.runLayout();
    
    console.log('\n=== INVESTIGATING EDGE e460 ===');
    
    // Find edge e460 in all collections
    let edge460 = null;
    const allNodes = state.getVisibleNodes();
    const allEdges = state.getVisibleEdges();
    const allContainers = state.getVisibleContainers();
    const allHyperEdges = state.visibleHyperEdges;
    
    // Try to find e460 in visible edges
    edge460 = allEdges.find(e => e.id === 'e460');
    if (!edge460) {
      console.log('❌ Edge e460 not found in visible edges');
      console.log('Checking all edges using internal accessor...');
      // This is hacky but needed for debugging
      const stateAny = state as any;
      for (const [id, edge] of stateAny._collections.graphEdges) {
        if (id === 'e460') {
          edge460 = edge;
          break;
        }
      }
    }
    
    if (edge460) {
      console.log(`✅ Found edge e460: ${edge460.source} -> ${edge460.target}, hidden=${edge460.hidden}`);
    } else {
      console.log('❌ Edge e460 not found anywhere!');
      return;
    }
    
    // Check containers bt_29 and bt_187
    const containers = [
      { id: 'bt_29', container: allContainers.find(c => c.id === 'bt_29') },
      { id: 'bt_187', container: allContainers.find(c => c.id === 'bt_187') }
    ];
    
    console.log('\n=== CHECKING PROBLEM CONTAINERS ===');
    for (const { id, container } of containers) {
      if (container) {
        console.log(`✅ Container ${id}: collapsed=${container.collapsed}, hidden=${container.hidden}`);
      } else {
        console.log(`❌ Container ${id}: not found in visible containers`);
        // Try to find in all containers
        const stateAny = state as any;
        const hiddenContainer = stateAny._collections.containers.get(id);
        if (hiddenContainer) {
          console.log(`   Found in hidden containers: collapsed=${hiddenContainer.collapsed}, hidden=${hiddenContainer.hidden}`);
        }
      }
    }
    
    // Check for hyperEdges that might represent this connectivity
    console.log('\n=== CHECKING HYPEREDGES FOR e460 ===');
    
    const relevantHyperEdges = allHyperEdges.filter(he => {
      // Check if this hyperEdge has e460 in its aggregatedEdges
      if (he.aggregatedEdges && he.aggregatedEdges.has('e460')) {
        return true;
      }
      
      // Check if endpoints match
      const matchesSource = he.source === edge460.source || he.target === edge460.source;
      const matchesTarget = he.source === edge460.target || he.target === edge460.target;
      const matchesContainers = he.source === 'bt_29' || he.target === 'bt_29' || 
                               he.source === 'bt_187' || he.target === 'bt_187';
      
      return (matchesSource || matchesTarget) && matchesContainers;
    });
    
    console.log(`Found ${relevantHyperEdges.length} potentially relevant hyperEdges:`);
    for (const he of relevantHyperEdges) {
      const hasE460 = he.aggregatedEdges && he.aggregatedEdges.has('e460');
      console.log(`  ${he.id}: ${he.source} -> ${he.target}, contains e460: ${hasE460}, aggregated: ${he.aggregatedEdges?.size || 0}`);
      
      if (he.aggregatedEdges) {
        const edgeIds = Array.from(he.aggregatedEdges.keys()).slice(0, 5);
        console.log(`    First few aggregated edges: ${edgeIds.join(', ')}`);
      }
    }
    
    // Try to understand the node hierarchy
    console.log('\n=== NODE HIERARCHY ANALYSIS ===');
    const sourceNode = edge460.source;
    const targetNode = edge460.target;
    
    console.log(`Edge e460 connects: ${sourceNode} -> ${targetNode}`);
    
    // Check if these nodes are visible
    const sourceVisible = allNodes.some(n => n.id === sourceNode);
    const targetVisible = allNodes.some(n => n.id === targetNode);
    console.log(`Source ${sourceNode} visible: ${sourceVisible}`);
    console.log(`Target ${targetNode} visible: ${targetVisible}`);
    
    // If nodes are not visible, they might be inside containers
    if (!sourceVisible || !targetVisible) {
      console.log('One or both endpoints are hidden - likely inside collapsed containers');
    }
  });
});
