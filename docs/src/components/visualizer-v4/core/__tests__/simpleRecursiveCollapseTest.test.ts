import { VisualizationState } from '../VisualizationState';

describe('Simple Recursive Collapse Test', () => {
  test('should prevent recursive collapse errors', () => {
    const state = new VisualizationState();
    
    // Create simple container structure
    state.addContainer('parent', { x: 0, y: 0, width: 200, height: 200 });
    state.addContainer('child', { x: 50, y: 50, width: 100, height: 100 });
    
    // Set up parent-child relationship
    state.addContainer('parent', { x: 0, y: 0, width: 200, height: 200, children: ['child'] });
    
    // Add a node to child container
    state.addGraphNode('node1', { x: 75, y: 75, width: 20, height: 20, containerId: 'child' });
    
    // Collapse child first
    state.setContainerState('child', { collapsed: true });
    console.log('✅ Child collapsed');
    
    // Collapse parent - should not try to re-collapse child
    try {
      state.setContainerState('parent', { collapsed: true });
      console.log('✅ Parent collapsed without recursive error');
    } catch (error) {
      console.error('❌ Recursive collapse error:', error.message);
      throw error;
    }
    
    // Validate state
    state.validateInvariants();
    expect(true).toBe(true); // Test passes if no errors thrown
  });
});
