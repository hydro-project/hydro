/**
 * Test for ValidationWrapper API boundary validation
 * 
 * Verifies that validation happens at public API boundaries
 * and not during internal operations.
 */

import { VisualizationState } from '../VisualizationState';

describe('Validation API Boundary', () => {
  let visState: VisualizationState;

  beforeEach(() => {
    visState = new VisualizationState();
  });

  test('should validate at public API boundary for addGraphEdge', () => {
    // This should trigger validation and fail because source/target don't exist
    expect(() => {
      visState.addGraphEdge('edge1', {
        id: 'edge1',
        source: 'nonexistent_source',
        target: 'nonexistent_target'
      });
    }).toThrow('VisualizationState invariant violations detected');
  });

  test('should allow valid operations to complete successfully', () => {
    // Add nodes first
    visState.addGraphNode('node1', { id: 'node1', x: 0, y: 0 });
    visState.addGraphNode('node2', { id: 'node2', x: 100, y: 100 });
    
    // This should work fine with validation
    expect(() => {
      visState.addGraphEdge('edge1', {
        id: 'edge1',
        source: 'node1',
        target: 'node2'
      });
    }).not.toThrow();

    // Verify the edge was added
    const edges = visState.getVisibleEdges();
    expect(edges).toHaveLength(1);
    expect(edges[0].id).toBe('edge1');
  });

  test('should validate container operations at API boundary', () => {
    // Add a container with children
    visState.addGraphNode('child1', { id: 'child1', x: 0, y: 0 });
    visState.addContainer('container1', {
      id: 'container1',
      children: ['child1'],
      collapsed: false
    });

    // Container operations should validate after completion
    expect(() => {
      visState.setContainerCollapsed('container1', true);
    }).not.toThrow();

    // Verify the container was collapsed
    const collapsed = visState.getContainerCollapsed('container1');
    expect(collapsed).toBe(true);
  });

  test('should skip validation for internal operations', () => {
    // Add nodes
    visState.addGraphNode('node1', { id: 'node1', x: 0, y: 0 });
    
    // Internal setNodeVisibility should not trigger validation
    // (this is called internally during container operations)
    expect(() => {
      visState.setNodeVisibility('node1', false);
    }).not.toThrow();
  });
});
