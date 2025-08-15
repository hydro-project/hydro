/**
 * @fileoverview Visual Configuration Panel Demo
 * 
 * Demonstrates the Visual Configuration Panel integration with a visualization system.
 * This serves as both documentation and a working example.
 */

import React, { useState, useCallback } from 'react';
import { VisualConfigPanel, VisualConfigState } from '../VisualConfigPanel';
import { COMPONENT_COLORS, TYPOGRAPHY } from '../../shared/config';

// Mock visualization state for demonstration
interface MockVisualizationState {
  nodeStyle: string;
  edgeStyle: string;
  containerStyle: string;
  edgeType: string;
  colorPalette: string;
  typographyScale: number;
  infoPanelFontSize: number;
  shadowIntensity: string;
  borderRadius: number;
  collapsedContainerWidth: number;
  collapsedContainerHeight: number;
}

export function VisualConfigPanelDemo() {
  const [visualConfig, setVisualConfig] = useState<VisualConfigState>({
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
  });

  // Handle configuration changes from the panel
  const handleConfigChange = useCallback((newConfig: VisualConfigState) => {
    setVisualConfig(newConfig);
    
    // In a real implementation, you would:
    // 1. Update the VisualizationState with new configuration
    // 2. Trigger re-rendering of the visualization
    // 3. Apply new styles to ReactFlow components
    console.log('Visual configuration updated:', newConfig);
  }, []);

  // Sample node to show visual effects
  const sampleNodeStyle: React.CSSProperties = {
    padding: '12px',
    border: `2px solid ${COMPONENT_COLORS.BORDER_MEDIUM}`,
    borderRadius: `${visualConfig.borderRadius}px`,
    backgroundColor: COMPONENT_COLORS.BACKGROUND_PRIMARY,
    fontSize: `${visualConfig.infoPanelFontSize * visualConfig.typographyScale}px`,
    boxShadow: visualConfig.shadowIntensity === 'LIGHT' 
      ? '0 1px 3px 0 rgba(0, 0, 0, 0.1)'
      : visualConfig.shadowIntensity === 'MEDIUM'
      ? '0 4px 6px -1px rgba(0, 0, 0, 0.1)'
      : '0 10px 15px -3px rgba(0, 0, 0, 0.1)',
    margin: '8px',
    minWidth: '120px',
    textAlign: 'center' as const,
    transition: 'all 0.2s ease'
  };

  const containerStyle: React.CSSProperties = {
    width: `${visualConfig.collapsedContainerWidth}px`,
    height: `${visualConfig.collapsedContainerHeight}px`,
    border: `1px solid ${COMPONENT_COLORS.BORDER_LIGHT}`,
    borderRadius: `${visualConfig.borderRadius}px`,
    backgroundColor: COMPONENT_COLORS.BACKGROUND_SECONDARY,
    padding: '8px',
    margin: '8px',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    fontSize: `${14 * visualConfig.typographyScale}px`,
    boxShadow: visualConfig.shadowIntensity === 'LIGHT' 
      ? '0 1px 3px 0 rgba(0, 0, 0, 0.1)'
      : visualConfig.shadowIntensity === 'MEDIUM'
      ? '0 4px 6px -1px rgba(0, 0, 0, 0.1)'
      : '0 10px 15px -3px rgba(0, 0, 0, 0.1)',
  };

  const edgeStyle: React.CSSProperties = {
    width: '100px',
    height: '2px',
    backgroundColor: COMPONENT_COLORS.BORDER_MEDIUM,
    margin: '16px',
    borderRadius: visualConfig.edgeStyle === 'thick' ? '2px' : '1px',
    opacity: visualConfig.edgeStyle === 'dashed' ? 0.7 : 1,
    background: visualConfig.edgeStyle === 'dashed' 
      ? `repeating-linear-gradient(to right, ${COMPONENT_COLORS.BORDER_MEDIUM} 0px, ${COMPONENT_COLORS.BORDER_MEDIUM} 5px, transparent 5px, transparent 10px)`
      : COMPONENT_COLORS.BORDER_MEDIUM
  };

  return (
    <div style={{ 
      display: 'flex', 
      height: '100vh',
      backgroundColor: COMPONENT_COLORS.BACKGROUND_SECONDARY,
      fontFamily: 'system-ui, -apple-system, sans-serif'
    }}>
      {/* Main Content Area */}
      <div style={{ 
        flex: 1, 
        padding: '20px',
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'center'
      }}>
        <h1 style={{ 
          fontSize: TYPOGRAPHY.PAGE_TITLE,
          color: COMPONENT_COLORS.TEXT_PRIMARY,
          marginBottom: '20px'
        }}>
          Visual Configuration Panel Demo
        </h1>
        
        <p style={{
          fontSize: TYPOGRAPHY.UI_MEDIUM,
          color: COMPONENT_COLORS.TEXT_SECONDARY,
          textAlign: 'center',
          maxWidth: '600px',
          marginBottom: '40px'
        }}>
          This demo shows how the Visual Configuration Panel provides real-time control over 
          visual constants. Adjust the settings in the panel to see live updates to the sample 
          visualization elements below.
        </p>

        {/* Sample Visualization Elements */}
        <div style={{ 
          display: 'flex', 
          flexDirection: 'column', 
          alignItems: 'center',
          gap: '20px'
        }}>
          {/* Sample Nodes */}
          <div style={{ display: 'flex', gap: '12px', alignItems: 'center' }}>
            <div style={{
              ...sampleNodeStyle,
              border: visualConfig.nodeStyle === 'highlighted' 
                ? `2px solid #4ade80` 
                : visualConfig.nodeStyle === 'selected'
                ? `2px solid #3b82f6`
                : visualConfig.nodeStyle === 'warning'
                ? `2px solid #f59e0b`
                : visualConfig.nodeStyle === 'error'
                ? `2px solid #ef4444`
                : `2px solid ${COMPONENT_COLORS.BORDER_MEDIUM}`
            }}>
              Sample Node<br/>
              <small>Style: {visualConfig.nodeStyle}</small>
            </div>

            {/* Sample Edge */}
            <div style={edgeStyle} title={`Edge Style: ${visualConfig.edgeStyle}, Type: ${visualConfig.edgeType}`} />

            <div style={sampleNodeStyle}>
              Target Node<br/>
              <small>Palette: {visualConfig.colorPalette}</small>
            </div>
          </div>

          {/* Sample Container */}
          <div style={containerStyle}>
            <div style={{ textAlign: 'center' }}>
              Sample Container<br/>
              <small style={{ 
                fontSize: `${12 * visualConfig.typographyScale}px`,
                color: COMPONENT_COLORS.TEXT_TERTIARY
              }}>
                {visualConfig.collapsedContainerWidth}×{visualConfig.collapsedContainerHeight}px
              </small>
            </div>
          </div>

          {/* Configuration Display */}
          <div style={{
            backgroundColor: COMPONENT_COLORS.BACKGROUND_PRIMARY,
            border: `1px solid ${COMPONENT_COLORS.BORDER_LIGHT}`,
            borderRadius: `${visualConfig.borderRadius}px`,
            padding: '16px',
            fontSize: `${12 * visualConfig.typographyScale}px`,
            fontFamily: 'monospace',
            maxWidth: '400px',
            overflowX: 'auto'
          }}>
            <strong>Current Configuration:</strong>
            <pre style={{ margin: '8px 0 0 0', fontSize: 'inherit' }}>
              {JSON.stringify(visualConfig, null, 2)}
            </pre>
          </div>
        </div>
      </div>

      {/* Visual Configuration Panel */}
      <div style={{ position: 'relative' }}>
        <VisualConfigPanel
          id="demo-visual-config"
          title="Visual Configuration"
          defaultConfig={visualConfig}
          onConfigChange={handleConfigChange}
        />
      </div>
    </div>
  );
}