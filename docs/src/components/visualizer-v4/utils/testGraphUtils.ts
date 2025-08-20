/* eslint-disable @typescript-eslint/no-explicit-any */
/**
 * Test graph creation utility for visual tests
 * Separate from test file to avoid importing test framework code in browser
 */

import { VisualizationState } from '../core/VisualizationState';

export function createCollapsedContainersTestGraph(): VisualizationState {
  const visState = new VisualizationState();
  
  // Add containers (both collapsed, not hidden)
  visState.setContainer('loc_0', {
    name: 'Container A (Collapsed)',
    style: 'default',
    collapsed: true,
    hidden: false,
    children: ['0', '1', '7', '8']
  });
  
  visState.setContainer('loc_1', {
    name: 'Container B (Collapsed)', 
    style: 'default',
    collapsed: true,
    hidden: false,
    children: ['2', '3', '4', '5', '6']
  });
  
  // Add nodes (all hidden since they're in collapsed containers)
  const nodeIds = ['0', '1', '2', '3', '4', '5', '6', '7', '8'];
  nodeIds.forEach(id => {
    visState.setGraphNode(id, {
      label: `Node ${id}`,
      style: 'default',
      hidden: true
    });
  });
  
  // Set some basic positions so they don't overlap
  visState.setContainerLayout('loc_0', {
    position: { x: 100, y: 100 },
    dimensions: { width: 200, height: 100 }
  });
  
  visState.setContainerLayout('loc_1', {
    position: { x: 400, y: 100 },
    dimensions: { width: 200, height: 100 }
  });
  
  // Add hyperEdges (the key elements that should be rendered)
  const hyperEdge1 = {
    id: 'hyper_loc_0_to_loc_1',
    source: 'loc_0',
    target: 'loc_1',
    style: 'default',
    hidden: false
  };
  
  const hyperEdge2 = {
    id: 'hyper_loc_1_to_loc_0',
    source: 'loc_1', 
    target: 'loc_0',
    style: 'default',
    hidden: false
  };
  
  // Add hyperEdges to internal collections
  (visState as any)._collections.hyperEdges.set(hyperEdge1.id, hyperEdge1);
  (visState as any)._collections.hyperEdges.set(hyperEdge2.id, hyperEdge2);
  
  return visState;
}
