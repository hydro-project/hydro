/**
 * @fileoverview Test edge style selection and marker color functionality
 * 
 * Tests that the edge appearance API correctly updates both edge paths and arrowhead colors.
 */

import { describe, it, expect, beforeEach } from 'vitest';
import { ReactFlowBridge } from '../bridges/ReactFlowBridge';
import { ReactFlowConverter } from '../render/ReactFlowConverter';
import { createVisualizationState } from '../core/VisualizationState';
import { NODE_STYLES, EDGE_STYLES } from '../shared/config';

describe('Edge Style and Marker Color', () => {
  let bridge: ReactFlowBridge;
  let converter: ReactFlowConverter;
  let visState: any;

  beforeEach(() => {
    bridge = new ReactFlowBridge();
    converter = new ReactFlowConverter();
    visState = createVisualizationState();

    // Create a simple test graph
    visState.setGraphNode('node1', { 
      label: 'Source', 
      style: NODE_STYLES.DEFAULT, 
      x: 0, 
      y: 0 
    });
    visState.setGraphNode('node2', { 
      label: 'Target', 
      style: NODE_STYLES.DEFAULT, 
      x: 200, 
      y: 0 
    });
    visState.setGraphEdge('edge1', {
      source: 'node1',
      target: 'node2',
      style: EDGE_STYLES.DEFAULT
    });
  });

  it('should set default marker color to #999 when no edge appearance is configured', () => {
    const result = bridge.visStateToReactFlow(visState);
    
    expect(result.edges).toHaveLength(1);
    expect(result.edges[0].markerEnd?.color).toBe('#999');
  });

  it('should update marker color when edge appearance is set', () => {
    const testColor = '#ff0000';
    bridge.setEdgeAppearance({ color: testColor });
    
    const result = bridge.visStateToReactFlow(visState);
    
    expect(result.edges).toHaveLength(1);
    expect(result.edges[0].markerEnd?.color).toBe(testColor);
  });

  it('should propagate edge appearance through ReactFlowConverter', () => {
    const testColor = '#00ff00';
    converter.setEdgeAppearance({ color: testColor });
    
    const result = converter.convert(visState);
    
    expect(result.edges).toHaveLength(1);
    expect(result.edges[0].markerEnd?.color).toBe(testColor);
  });

  it('should maintain marker properties while updating color', () => {
    const testColor = '#0000ff';
    bridge.setEdgeAppearance({ color: testColor });
    
    const result = bridge.visStateToReactFlow(visState);
    const edge = result.edges[0];
    
    expect(edge.markerEnd).toEqual({
      type: expect.any(String), // MarkerType.ArrowClosed
      width: 15,
      height: 15,
      color: testColor
    });
  });

  it('should handle edge appearance updates with multiple calls', () => {
    // Set initial color
    bridge.setEdgeAppearance({ color: '#111111' });
    let result = bridge.visStateToReactFlow(visState);
    expect(result.edges[0].markerEnd?.color).toBe('#111111');

    // Update color
    bridge.setEdgeAppearance({ color: '#222222' });
    result = bridge.visStateToReactFlow(visState);
    expect(result.edges[0].markerEnd?.color).toBe('#222222');
  });

  it('should fall back to default color when edge appearance color is cleared', () => {
    // Set a color first
    bridge.setEdgeAppearance({ color: '#333333' });
    let result = bridge.visStateToReactFlow(visState);
    expect(result.edges[0].markerEnd?.color).toBe('#333333');

    // Clear the color (set to undefined)
    bridge.setEdgeAppearance({ color: undefined });
    result = bridge.visStateToReactFlow(visState);
    expect(result.edges[0].markerEnd?.color).toBe('#999');
  });
});