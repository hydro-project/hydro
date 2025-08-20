/**
 * @fileoverview Visualization Component - Clean React integration
 * 
 * Demonstrates the new bridge architecture:
 * - Uses VisualizationEngine for orchestration
 * - Handles loading and error states
 * - Provides clean interface for ReactFlow integration
 */

import React, { useState, useEffect, useCallback } from 'react';
import { ReactFlow, Background, Controls, MiniMap } from '@xyflow/react';
import '@xyflow/react/dist/style.css';

import { VisualizationEngine } from '../core/VisualizationEngine';
import { ReactFlowBridge } from '../bridges/ReactFlowBridge';
import type { VisualizationState } from '../core/VisualizationState';
import type { ReactFlowData } from '../bridges/ReactFlowBridge';
import type { RenderConfig } from '../core/types';

export interface VisualizationComponentProps {
  visState: VisualizationState;
  config?: RenderConfig;
  className?: string;
  style?: React.CSSProperties;
}

export function VisualizationComponent({
  visState,
  config,
  className = '',
  style = {}
}: VisualizationComponentProps): JSX.Element {
  const [reactFlowData, setReactFlowData] = useState<ReactFlowData | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Create visualization engine
  const [engine] = useState(() => new VisualizationEngine(visState));

  // Function to refresh the visualization
  const refreshVisualization = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    
    try {
      // Run ELK layout
      await engine.runLayout();
      
      // Convert to ReactFlow format
      const bridge = new ReactFlowBridge();
      const reactFlowData = bridge.visStateToReactFlow(visState);
      
      setReactFlowData(reactFlowData);
    } catch (err) {
  // eslint-disable-next-line no-console
  console.error('[VisualizationComponent] Failed to generate visualization:', err);
      setError(err instanceof Error ? err.message : 'Unknown error');
    } finally {
      setIsLoading(false);
    }
  }, [visState, engine]);

  // Initial load and refresh when visState changes
  useEffect(() => {
    refreshVisualization();
  }, [refreshVisualization]);

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
            Running layout...
          </div>
        </div>
      </div>
    );
  }

  // Error state
  if (error) {
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
            onClick={refreshVisualization}
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
            onClick={refreshVisualization}
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
          Edges: {reactFlowData.edges.length}
        </div>
        <div style={{ display: 'flex', gap: '8px' }}>
          <button 
            onClick={refreshVisualization}
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
            onClick={refreshVisualization}
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
          onNodeDoubleClick={(event, node) => {
            // Handle container collapse/expand on double-click
            if (node.type === 'container') {
              const container = visState.getContainer(node.id);
              if (container) {
                if (container.collapsed) {
                  console.log(`[VisualizationComponent] 🔓 Expanding container: ${node.id}`);
                  visState.expandContainer(node.id);
                } else {
                  console.log(`[VisualizationComponent] 🔒 Collapsing container: ${node.id}`);
                  visState.collapseContainer(node.id);
                }
                // Force refresh after container state change
                refreshVisualization();
              }
            }
          }}
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
        style={{ height: '700px' }}
      />
    </div>
  );
}
