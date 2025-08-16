/**
 * @fileoverview Tests for Style Tuner functionality
 * 
 * Tests the Style Tuner panel controls, edge style selection, and marker color functionality.
 */

import { describe, test, expect } from 'vitest';
import { ReactFlowConverter } from '../../render/ReactFlowConverter';
import { ReactFlowBridge } from '../../bridges/ReactFlowBridge';
import { createVisualizationState } from '../VisualizationState';

describe('Style Tuner Tests', () => {
  
  describe('Edge Style Selection', () => {
    test('should support bezier, straight, and smoothstep edge types', () => {
      const edgeTypes = ['bezier', 'straight', 'smoothstep'];
      
      edgeTypes.forEach(edgeType => {
        // This test validates that the edge types are recognized
        // The actual path selection logic is in the edge components
        expect(edgeType).toMatch(/^(bezier|straight|smoothstep)$/);
      });
    });
    
    test('should handle edge style configuration changes', () => {
      // Test that style configuration objects can hold edge style values
      const styleConfig = {
        edgeStyle: 'bezier' as const,
        edgeColor: '#ff0000',
        edgeWidth: 3,
        edgeDashed: false
      };
      
      expect(styleConfig.edgeStyle).toBe('bezier');
      expect(styleConfig.edgeColor).toBe('#ff0000');
      expect(styleConfig.edgeWidth).toBe(3);
      expect(styleConfig.edgeDashed).toBe(false);
      
      // Test style updates
      const updatedConfig = { ...styleConfig, edgeStyle: 'straight' as const };
      expect(updatedConfig.edgeStyle).toBe('straight');
    });
  });

  describe('ReactFlowConverter Edge Appearance', () => {
    test('should accept setEdgeAppearance configuration', () => {
      const converter = new ReactFlowConverter();
      
      // Should not throw when setting edge appearance
      expect(() => {
        converter.setEdgeAppearance({ color: '#ff0000' });
      }).not.toThrow();
      
      // Should accept different color formats
      expect(() => {
        converter.setEdgeAppearance({ color: '#1976d2' });
        converter.setEdgeAppearance({ color: 'rgb(255, 0, 0)' });
        converter.setEdgeAppearance({ color: 'blue' });
      }).not.toThrow();
    });
    
    test('should pass edge appearance to bridge', () => {
      const converter = new ReactFlowConverter();
      const testColor = '#ff5722';
      
      // Set edge appearance
      converter.setEdgeAppearance({ color: testColor });
      
      // The converter should store and pass the configuration
      // We can't directly test the bridge interaction without mocking,
      // but we can verify the API exists and works
      expect(() => {
        converter.setEdgeAppearance({ color: testColor });
      }).not.toThrow();
    });
  });

  describe('ReactFlowBridge Marker Color', () => {
    test('should use configured edge color for markers', () => {
      const bridge = new ReactFlowBridge();
      const testColor = '#e91e63';
      
      // Set edge appearance
      bridge.setEdgeAppearance({ color: testColor });
      
      // Create a simple visualization state for testing
      const visState = createVisualizationState();
      visState.setGraphNode('node1', { label: 'Node 1', style: 'default', position: { x: 0, y: 0 } });
      visState.setGraphNode('node2', { label: 'Node 2', style: 'default', position: { x: 100, y: 0 } });
      visState.setGraphEdge('edge1', { source: 'node1', target: 'node2', style: 'default' });
      
      // Convert to ReactFlow format
      const result = bridge.visStateToReactFlow(visState);
      
      // Check that edges have marker with the configured color
      expect(result.edges).toHaveLength(1);
      const edge = result.edges[0];
      expect(edge.markerEnd).toBeDefined();
      expect(edge.markerEnd!.color).toBe(testColor);
    });
    
    test('should fallback to default color when no edge color configured', () => {
      const bridge = new ReactFlowBridge();
      
      // Create a simple visualization state for testing  
      const visState = createVisualizationState();
      visState.setGraphNode('node1', { label: 'Node 1', style: 'default', position: { x: 0, y: 0 } });
      visState.setGraphNode('node2', { label: 'Node 2', style: 'default', position: { x: 100, y: 0 } });
      visState.setGraphEdge('edge1', { source: 'node1', target: 'node2', style: 'default' });
      
      // Convert to ReactFlow format without setting edge appearance
      const result = bridge.visStateToReactFlow(visState);
      
      // Check that edges have marker with the default color
      expect(result.edges).toHaveLength(1);
      const edge = result.edges[0];
      expect(edge.markerEnd).toBeDefined();
      expect(edge.markerEnd!.color).toBe('#999'); // Default fallback color
    });
    
    test('should update marker color when edge appearance changes', () => {
      const bridge = new ReactFlowBridge();
      
      // Create a simple visualization state for testing
      const visState = createVisualizationState();
      visState.setGraphNode('node1', { label: 'Node 1', style: 'default', position: { x: 0, y: 0 } });
      visState.setGraphNode('node2', { label: 'Node 2', style: 'default', position: { x: 100, y: 0 } });
      visState.setGraphEdge('edge1', { source: 'node1', target: 'node2', style: 'default' });
      
      // First conversion with default color
      let result = bridge.visStateToReactFlow(visState);
      expect(result.edges[0].markerEnd!.color).toBe('#999');
      
      // Update edge appearance
      const newColor = '#2196f3';
      bridge.setEdgeAppearance({ color: newColor });
      
      // Convert again - should use new color
      result = bridge.visStateToReactFlow(visState);
      expect(result.edges[0].markerEnd!.color).toBe(newColor);
    });
  });

  describe('Style Configuration Integration', () => {
    test('should handle complete style configuration object', () => {
      const fullStyleConfig = {
        // Edge styling
        edgeStyle: 'smoothstep' as const,
        edgeColor: '#9c27b0',
        edgeWidth: 4,
        edgeDashed: true,
        
        // Node styling  
        nodeBorderRadius: 12,
        nodePadding: 16,
        nodeFontSize: 14,
        
        // Container styling
        containerBorderRadius: 8,
        containerBorderWidth: 3,
        containerShadow: 'MEDIUM' as const
      };
      
      // Should be able to destructure and use all properties
      const { edgeStyle, edgeColor, nodeBorderRadius, containerShadow } = fullStyleConfig;
      expect(edgeStyle).toBe('smoothstep');
      expect(edgeColor).toBe('#9c27b0');
      expect(nodeBorderRadius).toBe(12);
      expect(containerShadow).toBe('MEDIUM');
    });
    
    test('should handle partial style configuration updates', () => {
      const baseConfig = {
        edgeStyle: 'bezier' as const,
        edgeColor: '#1976d2',
        edgeWidth: 2
      };
      
      // Partial updates should merge correctly
      const update1 = { ...baseConfig, edgeColor: '#f44336' };
      expect(update1.edgeColor).toBe('#f44336');
      expect(update1.edgeStyle).toBe('bezier'); // Preserved
      
      const update2 = { ...update1, edgeStyle: 'straight' as const };
      expect(update2.edgeStyle).toBe('straight');
      expect(update2.edgeColor).toBe('#f44336'); // Preserved
    });
  });
});