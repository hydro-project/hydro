/**
 * Debug test to check current Y-axis coordinates
 */

import { describe, test } from 'vitest';
import { ELKBridge } from '../bridges/ELKBridge';
import { VisualizationState } from '../core/VisState';

describe('Y-Axis Debug', () => {
  test('should output current Y coordinates from ELK', async () => {
    // Create a simple test visualization
    const visState = new VisualizationState();
    
    // Add some test nodes
    visState.setGraphNode('node1', { x: 0, y: 0, width: 100, height: 50, label: 'Node 1' });
    visState.setGraphNode('node2', { x: 200, y: 100, width: 100, height: 50, label: 'Node 2' });
    
    // Add an edge
    visState.setGraphEdge('edge1', { source: 'node1', target: 'node2' });
    
    console.log('=== Y-AXIS DEBUG TEST ===');
    
    // Get initial node positions
    console.log('\n📍 Initial node positions:');
    visState.visibleNodes.forEach(node => {
      console.log(`  ${node.id}: (${node.x || 0}, ${node.y || 0})`);
    });
    
    // Run ELK layout
    const elkBridge = new ELKBridge();
    await elkBridge.layoutVisState(visState);
    
    // Get ELK layout results
    console.log('\n🔧 ELK layout results:');
    visState.visibleNodes.forEach(node => {
      const layout = visState.getNodeLayout(node.id);
      console.log(`  ${node.id}: ELK=(${layout?.position?.x || 0}, ${layout?.position?.y || 0})`);
    });
    
    // Get edge routing if available
    console.log('\n🔗 Edge routing:');
    visState.visibleEdges.forEach(edge => {
      const layout = visState.getEdgeLayout(edge.id);
      if (layout?.routing && layout.routing.length > 0) {
        const section = layout.routing[0];
        console.log(`  ${edge.id}: start=(${section.startPoint?.x || 0}, ${section.startPoint?.y || 0}), end=(${section.endPoint?.x || 0}, ${section.endPoint?.y || 0})`);
      } else {
        console.log(`  ${edge.id}: no routing data`);
      }
    });
    
    console.log('\n=== END Y-AXIS DEBUG ===');
  });
});
