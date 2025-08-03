/**
 * @fileoverview Bridge-Based GraphFlow Component
 * 
 * Complete replacement for alpha GraphFlow using our bridge architecture.
 * Maintains identical API while using the new VisualizationEngine internally.
 */

import React, { useEffect, useState, useCallback } from 'react';
import { ReactFlow, Background, Controls, MiniMap } from '@xyflow/react';
import '@xyflow/react/dist/style.css';

import { createVisualizationEngine } from '../core/VisualizationEngine';
import { ReactFlowConverter } from './ReactFlowConverter';
import { DEFAULT_RENDER_CONFIG } from './config';
import { nodeTypes } from './nodes';
import { edgeTypes } from './edges';
import type { VisualizationState } from '../core/VisState';
import type { ReactFlowData } from '../bridges/ReactFlowBridge';
import type { RenderConfig, GraphFlowEventHandlers } from '../core/types';

export interface GraphFlowProps {
  visualizationState: VisualizationState;
  config?: RenderConfig;
  eventHandlers?: GraphFlowEventHandlers;
  className?: string;
  style?: React.CSSProperties;
}

export function GraphFlow({
  visualizationState,
  config = DEFAULT_RENDER_CONFIG,
  eventHandlers,
  className,
  style
}: GraphFlowProps): JSX.Element {
  const [reactFlowData, setReactFlowData] = useState<ReactFlowData | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Create converter and engine
  const [converter] = useState(() => new ReactFlowConverter());
  const [engine] = useState(() => createVisualizationEngine(visualizationState, {
    autoLayout: true, // Always auto-layout for alpha compatibility
    enableLogging: false
  }));

  // Listen to visualization state changes
  useEffect(() => {
    const handleStateChange = async () => {
      try {
        setLoading(true);
        setError(null);

        console.log('[GraphFlow] 🔄 Visualization state changed, updating...');
        
        // Run layout
        await engine.runLayout();
        
        // Convert to ReactFlow format
        const data = converter.convert(visualizationState);
        setReactFlowData(data);
        
        console.log('[GraphFlow] ✅ Updated ReactFlow data:', {
          nodes: data.nodes.length,
          edges: data.edges.length
        });
        
      } catch (err) {
        console.error('[GraphFlow] ❌ Failed to update visualization:', err);
        setError(err instanceof Error ? err.message : String(err));
      } finally {
        setLoading(false);
      }
    };

    // Initial render
    handleStateChange();

    // For alpha compatibility, we just do initial render
    // Real change detection would be implemented with proper state listeners
    
  }, [visualizationState, engine, converter]);

  // Handle node events
  const onNodeClick = useCallback((event: any, node: any) => {
    console.log('[GraphFlow] 🖱️ Node clicked:', node.id);
    eventHandlers?.onNodeClick?.(event, node);
  }, [eventHandlers]);

  const onEdgeClick = useCallback((event: any, edge: any) => {
    console.log('[GraphFlow] 🖱️ Edge clicked:', edge.id);
    eventHandlers?.onEdgeClick?.(event, edge);
  }, [eventHandlers]);

  // Loading state
  if (loading && !reactFlowData) {
    return (
      <div 
        className={className}
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          height: '400px',
          background: '#f5f5f5',
          border: '1px solid #ddd',
          borderRadius: '8px',
          ...style
        }}
      >
        <div style={{ textAlign: 'center', color: '#666' }}>
          <div style={{ fontSize: '24px', marginBottom: '8px' }}>🔄</div>
          <div>Running layout...</div>
        </div>
      </div>
    );
  }

  // Error state
  if (error) {
    return (
      <div 
        className={className}
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          height: '400px',
          background: '#ffe6e6',
          border: '1px solid #ff9999',
          borderRadius: '8px',
          ...style
        }}
      >
        <div style={{ textAlign: 'center', color: '#cc0000' }}>
          <div style={{ fontSize: '24px', marginBottom: '8px' }}>❌</div>
          <div><strong>Visualization Error:</strong></div>
          <div style={{ fontSize: '14px', marginTop: '4px' }}>{error}</div>
        </div>
      </div>
    );
  }

  // No data state
  if (!reactFlowData) {
    return (
      <div 
        className={className}
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          height: '400px',
          background: '#f9f9f9',
          border: '1px solid #ddd',
          borderRadius: '8px',
          ...style
        }}
      >
        <div style={{ textAlign: 'center', color: '#666' }}>
          <div style={{ fontSize: '24px', marginBottom: '8px' }}>📊</div>
          <div>No visualization data</div>
        </div>
      </div>
    );
  }

  // Main ReactFlow render
  return (
    <div className={className} style={{ height: '400px', ...style }}>
      <ReactFlow
        nodes={reactFlowData.nodes}
        edges={reactFlowData.edges}
        nodeTypes={nodeTypes}
        edgeTypes={edgeTypes}
        onNodeClick={onNodeClick}
        onEdgeClick={onEdgeClick}
        fitView={config.fitView !== false}
        fitViewOptions={{ padding: 0.1, maxZoom: 1.2 }}
        attributionPosition="bottom-left"
        nodesDraggable={config.nodesDraggable !== false}
        nodesConnectable={config.nodesConnectable !== false}
        elementsSelectable={config.elementsSelectable !== false}
        panOnDrag={config.enablePan !== false}
        zoomOnScroll={config.enableZoom !== false}
        minZoom={0.1}
        maxZoom={2}
      >
        <Background color="#ccc" />
        {config.enableControls !== false && <Controls />}
        {config.enableMiniMap !== false && (
          <MiniMap 
            nodeColor="#666"
            nodeStrokeWidth={2}
            position="bottom-right"
          />
        )}
      </ReactFlow>
      
      {/* Loading overlay during updates */}
      {loading && (
        <div style={{
          position: 'absolute',
          top: '10px',
          right: '10px',
          background: 'rgba(255, 255, 255, 0.9)',
          padding: '8px 12px',
          borderRadius: '4px',
          border: '1px solid #ddd',
          fontSize: '12px',
          color: '#666'
        }}>
          🔄 Updating...
        </div>
      )}
    </div>
  );
}
