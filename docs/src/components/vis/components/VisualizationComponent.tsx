/**
 * @fileoverview Visualization Component - Clean React integration
 * 
 * Demonstrates the new bridge architecture:
 * - Uses VisualizationEngine for orchestration
 * - Handles loading and error states
 * - Provides clean interface for ReactFlow integration
 */

import React from 'react';
import { ReactFlow, Background, Controls, MiniMap } from '@xyflow/react';
import '@xyflow/react/dist/style.css';

import { useVisualization } from '../hooks/useVisualization';
import type { VisualizationState } from '../core/VisState';
import type { UseVisualizationConfig } from '../hooks/useVisualization';

export interface VisualizationComponentProps {
  visState: VisualizationState;
  config?: UseVisualizationConfig;
  className?: string;
  style?: React.CSSProperties;
}

export function VisualizationComponent({
  visState,
  config,
  className = '',
  style = {}
}: VisualizationComponentProps): JSX.Element {
  const {
    reactFlowData,
    engineState,
    runLayout,
    visualize,
    onDataChanged,
    isLoading,
    isReady,
    hasError,
    error
  } = useVisualization(visState, config);

  // Loading state
  if (isLoading) {
    return (
      <div className={`visualization-loading ${className}`} style={{ 
        display: 'flex', 
        alignItems: 'center', 
        justifyContent: 'center',
        minHeight: '400px',
        background: '#f5f5f5',
        border: '1px solid #ddd',
        borderRadius: '8px',
        ...style 
      }}>
        <div style={{ textAlign: 'center' }}>
          <div style={{ fontSize: '24px', marginBottom: '16px' }}>🔄</div>
          <div style={{ fontSize: '16px', color: '#666' }}>
            {engineState.phase === 'laying_out' && 'Running layout...'}
            {engineState.phase === 'rendering' && 'Generating visualization...'}
            {engineState.phase === 'initial' && 'Initializing...'}
          </div>
          <div style={{ fontSize: '12px', color: '#999', marginTop: '8px' }}>
            Phase: {engineState.phase} | Layouts: {engineState.layoutCount}
          </div>
        </div>
      </div>
    );
  }

  // Error state
  if (hasError) {
    return (
      <div className={`visualization-error ${className}`} style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        minHeight: '400px',
        background: '#ffe6e6',
        border: '1px solid #ff9999',
        borderRadius: '8px',
        ...style
      }}>
        <div style={{ textAlign: 'center', maxWidth: '500px' }}>
          <div style={{ fontSize: '24px', marginBottom: '16px' }}>❌</div>
          <div style={{ fontSize: '16px', color: '#cc0000', marginBottom: '12px' }}>
            Visualization Error
          </div>
          <div style={{ fontSize: '14px', color: '#666', marginBottom: '16px' }}>
            {error}
          </div>
          <button 
            onClick={visualize}
            style={{
              padding: '8px 16px',
              background: '#007bff',
              color: 'white',
              border: 'none',
              borderRadius: '4px',
              cursor: 'pointer'
            }}
          >
            Retry
          </button>
        </div>
      </div>
    );
  }

  // No data state
  if (!reactFlowData) {
    return (
      <div className={`visualization-empty ${className}`} style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        minHeight: '400px',
        background: '#f9f9f9',
        border: '1px solid #ddd',
        borderRadius: '8px',
        ...style
      }}>
        <div style={{ textAlign: 'center' }}>
          <div style={{ fontSize: '24px', marginBottom: '16px' }}>📊</div>
          <div style={{ fontSize: '16px', color: '#666', marginBottom: '12px' }}>
            Ready to visualize
          </div>
          <button 
            onClick={visualize}
            style={{
              padding: '8px 16px',
              background: '#28a745',
              color: 'white',
              border: 'none',
              borderRadius: '4px',
              cursor: 'pointer'
            }}
          >
            Generate Visualization
          </button>
        </div>
      </div>
    );
  }

  // Success state - render ReactFlow
  return (
    <div className={`visualization-display ${className}`} style={{ 
      height: '600px', 
      border: '1px solid #ddd',
      borderRadius: '8px',
      overflow: 'hidden',
      ...style 
    }}>
      {/* Header with controls */}
      <div style={{
        padding: '12px 16px',
        background: '#f8f9fa',
        borderBottom: '1px solid #ddd',
        display: 'flex',
        justifyContent: 'space-between',
        alignItems: 'center'
      }}>
        <div style={{ fontSize: '14px', color: '#666' }}>
          Nodes: {reactFlowData.nodes.length} | 
          Edges: {reactFlowData.edges.length} | 
          Layouts: {engineState.layoutCount}
        </div>
        <div style={{ display: 'flex', gap: '8px' }}>
          <button 
            onClick={runLayout}
            style={{
              padding: '4px 12px',
              background: '#007bff',
              color: 'white',
              border: 'none',
              borderRadius: '4px',
              cursor: 'pointer',
              fontSize: '12px'
            }}
          >
            Re-layout
          </button>
          <button 
            onClick={onDataChanged}
            style={{
              padding: '4px 12px',
              background: '#6c757d',
              color: 'white',
              border: 'none',
              borderRadius: '4px',
              cursor: 'pointer',
              fontSize: '12px'
            }}
          >
            Refresh
          </button>
        </div>
      </div>

      {/* ReactFlow visualization */}
      <div style={{ height: 'calc(100% - 57px)' }}>
        <ReactFlow
          nodes={reactFlowData.nodes}
          edges={reactFlowData.edges}
          fitView
          attributionPosition="bottom-left"
        >
          <Background />
          <Controls />
          <MiniMap />
        </ReactFlow>
      </div>
    </div>
  );
}

/**
 * Example usage component
 */
export interface ExampleVisualizationProps {
  visState: VisualizationState;
}

export function ExampleVisualization({ visState }: ExampleVisualizationProps): JSX.Element {
  return (
    <div style={{ padding: '20px' }}>
      <h2 style={{ marginBottom: '20px' }}>🌉 New Bridge Architecture Demo</h2>
      
      <div style={{ marginBottom: '16px', padding: '16px', background: '#e8f4fd', borderRadius: '8px' }}>
        <h3 style={{ margin: '0 0 8px 0', color: '#0056b3' }}>✨ Features Demonstrated:</h3>
        <ul style={{ margin: 0, paddingLeft: '20px', color: '#666' }}>
          <li>🔧 <strong>ELKBridge</strong>: Includes ALL edges (regular + hyperedges)</li>
          <li>🎨 <strong>ReactFlowBridge</strong>: Clean coordinate translation</li>
          <li>⚡ <strong>VisualizationEngine</strong>: State machine orchestration</li>
          <li>🎯 <strong>React Integration</strong>: Hooks and error handling</li>
        </ul>
      </div>

      <VisualizationComponent 
        visState={visState}
        config={{
          autoLayout: true,
          autoVisualize: true,
          enableLogging: true
        }}
        style={{ height: '700px' }}
      />
      
      <div style={{ marginTop: '16px', fontSize: '12px', color: '#666' }}>
        💡 The hyperedge layout bug is now fixed! Collapsed containers and external nodes 
        will no longer overlap because hyperedges are included in ELK calculations.
      </div>
    </div>
  );
}
