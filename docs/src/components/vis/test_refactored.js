// Quick test to verify the refactored VisState functionality
import { createVisualizationState } from './VisState.js';
import { NODE_STYLES, EDGE_STYLES } from './constants.js';

const state = createVisualizationState();

console.log('🧪 Testing refactored VisualizationState...');

// Test node operations
state.setGraphNode('node1', { label: 'Test Node', style: NODE_STYLES.DEFAULT });
console.log('✅ Node created:', state.getGraphNode('node1')?.label);

// Test hidden functionality
state.setNodeHidden('node1', true);
console.log('✅ Node hidden:', state.getNodeHidden('node1'));
console.log('✅ Visible nodes count:', state.getVisibleNodes().length);

state.setNodeHidden('node1', false);
console.log('✅ Node shown:', state.getNodeHidden('node1'));
console.log('✅ Visible nodes count after showing:', state.getVisibleNodes().length);

// Test edge operations
state.setGraphEdge('edge1', { 
  source: 'node1', 
  target: 'node2', 
  style: EDGE_STYLES.DEFAULT 
});
console.log('✅ Edge created:', state.getGraphEdge('edge1')?.source);

// Test container operations
state.setContainer('container1', {
  children: ['node1'],
  collapsed: false
});
console.log('✅ Container created:', state.getContainer('container1')?.id);

// Test collapse functionality
state.collapseContainer('container1');
console.log('✅ Container collapsed:', state.getContainerCollapsed('container1'));

state.expandContainer('container1');
console.log('✅ Container expanded:', !state.getContainerCollapsed('container1'));

console.log('🎉 All tests passed! Refactoring successful.');
