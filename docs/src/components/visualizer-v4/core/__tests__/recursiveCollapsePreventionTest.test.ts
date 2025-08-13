import { VisualizationState } from '../VisualizationState';

describe('Recursive Collapse Prevention', () => {
  describe('Container Hierarchy Collapse Prevention', () => {
    test('should prevent recursive collapse of already-collapsed containers', () => {
      const state = new VisualizationState();
      
      // Create hierarchical container structure like chat.json backtraces
      state.addContainer('bt_1', { x: 0, y: 0, width: 200, height: 100 });
      state.addContainer('bt_5', { x: 250, y: 0, width: 200, height: 100 });
      state.addContainer('bt_6', { x: 500, y: 0, width: 200, height: 100, children: ['bt_5'] });
      
      // Create nodes within containers
      state.addGraphNode('node_in_bt_1', { 
        x: 25, y: 25, width: 50, height: 20, 
        containerId: 'bt_1' 
      });
      state.addGraphNode('node_in_bt_5', { 
        x: 25, y: 25, width: 50, height: 20, 
        containerId: 'bt_5' 
      });
      state.addGraphNode('node_in_bt_6', {
        x: 25, y: 25, width: 50, height: 20,
        containerId: 'bt_6'
      });
      
      // Create edges between nodes in different containers
      state.addGraphEdge('edge_1', { source: 'node_in_bt_1', target: 'node_in_bt_5' });
      state.addGraphEdge('edge_1_to_5', { source: 'node_in_bt_1', target: 'node_in_bt_5' });
      state.addGraphEdge('edge_5_to_1', { source: 'node_in_bt_5', target: 'node_in_bt_1' });
      
      // Step 1: Collapse bt_1
      console.log('=== STEP 1: Collapsing bt_1 ===');
      state.setContainerState('bt_1', { collapsed: true });
      
      // Step 2: Collapse bt_5 (which is inside bt_6) 
      console.log('=== STEP 2: Collapsing bt_5 ===');
      state.setContainerState('bt_5', { collapsed: true });
      
      // Step 3: Collapse bt_6 (parent container) - this should NOT try to re-collapse bt_5
      console.log('=== STEP 3: Collapsing bt_6 (parent container) ===');
      try {
        state.setContainerState('bt_6', { collapsed: true });
        console.log('✅ bt_6 collapsed successfully without recursive collapse error');
      } catch (error) {
        console.error('❌ Recursive collapse error:', error.message);
        throw error;
      }
      
      // Validate no duplicate hyperEdges exist
      state.validateInvariants();
      
      console.log('✅ Test passed - recursive collapse prevented successfully');
    });

    test('should handle complex nested hierarchy without recursive collapse errors', () => {
      const state = new VisualizationState();
      
      // Create a deeply nested container hierarchy: bt_1 > bt_2 > bt_3 > bt_4
      state.addContainer('bt_1', { x: 0, y: 0, width: 400, height: 300 });
      state.addContainer('bt_2', { x: 50, y: 50, width: 300, height: 200 });
      state.addContainer('bt_3', { x: 100, y: 100, width: 200, height: 100 });
      state.addContainer('bt_4', { x: 150, y: 150, width: 100, height: 50 });
      
      // Add nodes in each container
      state.addGraphNode('node1', { x: 25, y: 25, width: 20, height: 20, containerId: 'bt_1' });
      state.addGraphNode('node2', { x: 75, y: 75, width: 20, height: 20, containerId: 'bt_2' });
      state.addGraphNode('node3', { x: 125, y: 125, width: 20, height: 20, containerId: 'bt_3' });
      state.addGraphNode('node4', { x: 150, y: 150, width: 20, height: 20, containerId: 'bt_4' });
      
      // Set up hierarchy: bt_2 inside bt_1, bt_3 inside bt_2, bt_4 inside bt_3
      state.addContainer('bt_1', { x: 0, y: 0, width: 400, height: 300, children: ['bt_2'] });
      state.addContainer('bt_2', { x: 50, y: 50, width: 300, height: 200, children: ['bt_3'] });
      state.addContainer('bt_3', { x: 100, y: 100, width: 200, height: 100, children: ['bt_4'] });
      
      // Add cross-hierarchy edges
      state.addGraphEdge('edge1', { source: 'node1', target: 'node4' });
      state.addGraphEdge('edge2', { source: 'node2', target: 'node3' });
      
      // Collapse from deepest to shallowest
      console.log('=== Collapsing nested containers ===');
      
      // This should work fine - collapse bt_4 first
      state.setContainerState('bt_4', { collapsed: true });
      console.log('✅ bt_4 collapsed');
      
      // Collapse bt_3 - should not try to re-collapse bt_4
      state.setContainerState('bt_3', { collapsed: true });
      console.log('✅ bt_3 collapsed');
      
      // Collapse bt_2 - should not try to re-collapse bt_3 or bt_4  
      state.setContainerState('bt_2', { collapsed: true });
      console.log('✅ bt_2 collapsed');
      
      // Collapse bt_1 - should not try to re-collapse any children
      state.setContainerState('bt_1', { collapsed: true });
      console.log('✅ bt_1 collapsed');
      
      // Validate state integrity
      state.validateInvariants();
      console.log('✅ Complex nested hierarchy handled correctly');
    });
  });
});
