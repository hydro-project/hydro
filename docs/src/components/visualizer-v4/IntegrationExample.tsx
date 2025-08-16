/**
 * @fileoverview Integration Example
 * 
 * Shows how to integrate the Visual Configuration Panel with a visualization system.
 * This demonstrates the complete workflow and serves as documentation.
 */

import React, { useState, useCallback, useEffect } from 'react';
import { VisualConfigPanel } from '../components/VisualConfigPanel';
import type { VisualConfigState } from '../components/types';
import { COMPONENT_COLORS, TYPOGRAPHY, SHADOWS } from '../shared/config';

// Example showing how the panel would integrate with VisualizationState
interface ExampleVisualizationState {
  // This would be the actual VisualizationState in real implementation
  nodes: Array<{ id: string; style: any; type: string }>;
  edges: Array<{ id: string; style: any; type: string }>;
  containers: Array<{ id: string; style: any; collapsed: boolean }>;
  
  // Visual configuration applied to the visualization
  visualConfig: VisualConfigState;
}

export function IntegrationExample() {
  // Simulated visualization state
  const [visState, setVisState] = useState<ExampleVisualizationState>({
    nodes: [
      { id: 'node1', style: {}, type: 'Source' },
      { id: 'node2', style: {}, type: 'Transform' },
      { id: 'node3', style: {}, type: 'Sink' }
    ],
    edges: [
      { id: 'edge1', style: {}, type: 'default' },
      { id: 'edge2', style: {}, type: 'default' }
    ],
    containers: [
      { id: 'container1', style: {}, collapsed: false }
    ],
    visualConfig: {
      nodeStyle: 'default',
      edgeStyle: 'default',
      containerStyle: 'default',
      edgeType: 'default',
      colorPalette: 'Set3',
      typographyScale: 1.0,
      infoPanelFontSize: 14,
      shadowIntensity: 'MEDIUM',
      borderRadius: 6,
      collapsedContainerWidth: 200,
      collapsedContainerHeight: 150
    }
  });

  // Handle visual configuration changes from the panel
  const handleVisualConfigChange = useCallback((newConfig: VisualConfigState) => {
    console.log('Visual configuration updated:', newConfig);
    
    // Update the visualization state with new configuration
    setVisState(prevState => ({
      ...prevState,
      visualConfig: newConfig,
      // In real implementation, you would update the actual visualization here:
      // - Apply new styles to ReactFlow nodes/edges
      // - Update container dimensions
      // - Change color palettes
      // - Update typography scaling
      nodes: prevState.nodes.map(node => ({
        ...node,
        style: {
          ...node.style,
          // Apply node style based on configuration
          border: newConfig.nodeStyle === 'highlighted' ? '2px solid #4ade80' :
                  newConfig.nodeStyle === 'selected' ? '2px solid #3b82f6' :
                  newConfig.nodeStyle === 'warning' ? '2px solid #f59e0b' :
                  newConfig.nodeStyle === 'error' ? '2px solid #ef4444' :
                  `1px solid ${COMPONENT_COLORS.BORDER_MEDIUM}`,
          borderRadius: `${newConfig.borderRadius}px`,
          boxShadow: SHADOWS[newConfig.shadowIntensity as keyof typeof SHADOWS],
          fontSize: `${newConfig.infoPanelFontSize * newConfig.typographyScale}px`
        }
      })),
      edges: prevState.edges.map(edge => ({
        ...edge,
        style: {
          ...edge.style,
          strokeWidth: newConfig.edgeStyle === 'thick' ? 3 : 
                      newConfig.edgeStyle === 'dashed' ? 2 : 1,
          strokeDasharray: newConfig.edgeStyle === 'dashed' ? '5,5' : 'none',
          opacity: newConfig.edgeStyle === 'warning' ? 0.7 : 1
        }
      })),
      containers: prevState.containers.map(container => ({
        ...container,
        style: {
          ...container.style,
          width: container.collapsed ? `${newConfig.collapsedContainerWidth}px` : 'auto',
          height: container.collapsed ? `${newConfig.collapsedContainerHeight}px` : 'auto',
          borderRadius: `${newConfig.borderRadius}px`,
          boxShadow: SHADOWS[newConfig.shadowIntensity as keyof typeof SHADOWS]
        }
      }))
    }));
  }, []);

  // Apply visual changes immediately when configuration changes
  useEffect(() => {
    // In a real implementation, this would trigger:
    // 1. VisualizationEngine re-layout if needed
    // 2. ReactFlow component updates
    // 3. Style updates to all visual elements
    console.log('Applying visual configuration:', visState.visualConfig);
  }, [visState.visualConfig]);

  return (
    <div style={{
      display: 'flex',
      height: '100vh',
      backgroundColor: COMPONENT_COLORS.BACKGROUND_SECONDARY,
      fontFamily: 'system-ui, -apple-system, sans-serif'
    }}>
      {/* Main Visualization Area */}
      <div style={{
        flex: 1,
        padding: '20px',
        display: 'flex',
        flexDirection: 'column'
      }}>
        <h1 style={{
          fontSize: TYPOGRAPHY.PAGE_TITLE,
          color: COMPONENT_COLORS.TEXT_PRIMARY,
          marginBottom: '20px'
        }}>
          Visual Configuration Panel Integration
        </h1>

        <p style={{
          fontSize: TYPOGRAPHY.UI_MEDIUM,
          color: COMPONENT_COLORS.TEXT_SECONDARY,
          marginBottom: '20px',
          maxWidth: '600px'
        }}>
          This example demonstrates how the Visual Configuration Panel integrates with a visualization system.
          The panel provides real-time control over visual constants from shared/config.ts.
        </p>

        {/* Simulated Visualization Elements */}
        <div style={{
          flex: 1,
          border: `1px solid ${COMPONENT_COLORS.BORDER_LIGHT}`,
          borderRadius: '8px',
          padding: '20px',
          backgroundColor: COMPONENT_COLORS.BACKGROUND_PRIMARY,
          position: 'relative'
        }}>
          <h3 style={{
            fontSize: TYPOGRAPHY.UI_LARGE,
            color: COMPONENT_COLORS.TEXT_PRIMARY,
            marginBottom: '16px'
          }}>
            Simulated ReactFlow Visualization
          </h3>

          {/* Sample nodes with applied styles */}
          <div style={{
            display: 'flex',
            gap: '20px',
            alignItems: 'center',
            marginBottom: '20px',
            flexWrap: 'wrap'
          }}>
            {visState.nodes.map((node) => (
              <div
                key={node.id}
                style={{
                  ...node.style,
                  padding: '12px 16px',
                  backgroundColor: COMPONENT_COLORS.BACKGROUND_SECONDARY,
                  minWidth: '80px',
                  textAlign: 'center',
                  transition: 'all 0.2s ease'
                }}
              >
                <div style={{ fontWeight: 'bold' }}>{node.type}</div>
                <div style={{ fontSize: '12px', opacity: 0.7 }}>{node.id}</div>
              </div>
            ))}
          </div>

          {/* Sample edges */}
          <div style={{ marginBottom: '20px' }}>
            <h4 style={{ fontSize: TYPOGRAPHY.UI_MEDIUM, marginBottom: '8px' }}>
              Edges (Style: {visState.visualConfig.edgeStyle}, Type: {visState.visualConfig.edgeType})
            </h4>
            {visState.edges.map((edge) => (
              <div
                key={edge.id}
                style={{
                  ...edge.style,
                  height: '2px',
                  backgroundColor: COMPONENT_COLORS.BORDER_MEDIUM,
                  width: '100px',
                  margin: '4px 0'
                }}
              />
            ))}
          </div>

          {/* Sample containers */}
          <div style={{ marginBottom: '20px' }}>
            <h4 style={{ fontSize: TYPOGRAPHY.UI_MEDIUM, marginBottom: '8px' }}>
              Containers (Style: {visState.visualConfig.containerStyle})
            </h4>
            {visState.containers.map((container) => (
              <div
                key={container.id}
                style={{
                  ...container.style,
                  border: `1px solid ${COMPONENT_COLORS.BORDER_LIGHT}`,
                  backgroundColor: COMPONENT_COLORS.BACKGROUND_SECONDARY,
                  padding: '12px',
                  margin: '8px 0',
                  display: 'inline-block'
                }}
              >
                <div>Container {container.id}</div>
                <div style={{ fontSize: '12px', opacity: 0.7 }}>
                  {container.collapsed ? 'Collapsed' : 'Expanded'}
                </div>
              </div>
            ))}
          </div>

          {/* Configuration display */}
          <div style={{
            position: 'absolute',
            bottom: '20px',
            right: '20px',
            backgroundColor: COMPONENT_COLORS.BACKGROUND_SECONDARY,
            padding: '12px',
            borderRadius: '6px',
            fontSize: '12px',
            fontFamily: 'monospace',
            maxWidth: '300px',
            overflow: 'auto'
          }}>
            <strong>Current Configuration:</strong>
            <pre style={{ margin: '8px 0 0 0', fontSize: '10px' }}>
              {JSON.stringify(visState.visualConfig, null, 2)}
            </pre>
          </div>
        </div>
      </div>

      {/* Visual Configuration Panel */}
      <div style={{ position: 'relative', width: '320px', flexShrink: 0 }}>
        <VisualConfigPanel
          id="integration-visual-config"
          title="Visual Configuration"
          defaultConfig={visState.visualConfig}
          onConfigChange={handleVisualConfigChange}
        />
      </div>
    </div>
  );
}

// Export for demonstration purposes
export default IntegrationExample;