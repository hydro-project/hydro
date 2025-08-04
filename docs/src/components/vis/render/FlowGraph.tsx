/**
 * @fileoverview Bridge-Based FlowGraph Component
 * 
 * Complete replacement for alpha FlowGraph using our bridge architecture.
 * Maintains identical API while using the new VisualizationEngine internally.
 */

import React, { useEffect, useState, useCallback, useRef, forwardRef, useImperativeHandle } from 'react';
import { ReactFlow, Background, Controls, MiniMap, useReactFlow, ReactFlowProvider } from '@xyflow/react';
import '@xyflow/react/dist/style.css';

import { createVisualizationEngine } from '../core/VisualizationEngine';
import { ReactFlowConverter } from './ReactFlowConverter';
import { DEFAULT_RENDER_CONFIG } from './config';
import { nodeTypes } from './nodes';
import { edgeTypes } from './edges';
import type { VisualizationState } from '../core/VisState';
import type { ReactFlowData } from '../bridges/ReactFlowBridge';
import type { RenderConfig, FlowGraphEventHandlers, LayoutConfig } from '../core/types';

export interface FlowGraphProps {
  visualizationState: VisualizationState;
  config?: RenderConfig;
  layoutConfig?: LayoutConfig;
  eventHandlers?: FlowGraphEventHandlers;
  className?: string;
  style?: React.CSSProperties;
}

export interface FlowGraphRef {
  fitView: () => void;
}

export function FlowGraph({
  visualizationState,
  config = DEFAULT_RENDER_CONFIG,
  layoutConfig,
  eventHandlers,
  className,
  style
}: FlowGraphProps): JSX.Element {
  const [reactFlowData, setReactFlowData] = useState<ReactFlowData | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  
  // Store manual drag positions to preserve user positioning
  const [manualPositions, setManualPositions] = useState<Map<string, { x: number; y: number }>>(new Map());
  
  // Ref to track the base layout data (before manual positioning)
  const baseReactFlowDataRef = useRef<ReactFlowData | null>(null);

  // Create converter and engine
  const [converter] = useState(() => new ReactFlowConverter());
  const [engine] = useState(() => createVisualizationEngine(visualizationState, {
    autoLayout: true, // Always auto-layout for alpha compatibility
    enableLogging: false,
    layoutConfig: layoutConfig
  }));

  // Function to apply manual positions to existing ReactFlow data
  const applyManualPositions = useCallback((baseData: ReactFlowData, manualPosMap: Map<string, { x: number; y: number }>) => {
    if (manualPosMap.size === 0) return baseData;
    
    return {
      ...baseData,
      nodes: baseData.nodes.map(node => {
        const manualPos = manualPosMap.get(node.id);
        if (manualPos) {
          return {
            ...node,
            position: { x: manualPos.x, y: manualPos.y }
          };
        }
        return node;
      })
    };
  }, []);

  // Listen to layout config changes
  useEffect(() => {
    if (layoutConfig) {
      console.log('[FlowGraph] 🔧 Layout config changed, updating engine...');
      engine.updateLayoutConfig(layoutConfig, false); // Don't auto re-layout yet
    }
  }, [layoutConfig, engine]);

  // Listen to visualization state changes
  useEffect(() => {
    const handleStateChange = async () => {
      try {
        setLoading(true);
        setError(null);

        console.log('[FlowGraph] 🔄 Visualization state changed, updating...');
        
        // Run layout
        await engine.runLayout();
        
        // Convert to ReactFlow format
        const baseData = converter.convert(visualizationState);
        
        // Store the base data for reference
        baseReactFlowDataRef.current = baseData;
        
        // Apply any existing manual positions
        const dataWithManualPositions = applyManualPositions(baseData, manualPositions);
        
        setReactFlowData(dataWithManualPositions);
        
        console.log('[FlowGraph] ✅ Updated ReactFlow data:', {
          nodes: dataWithManualPositions.nodes.length,
          edges: dataWithManualPositions.edges.length
        });
        
      } catch (err) {
        console.error('[FlowGraph] ❌ Failed to update visualization:', err);
        setError(err instanceof Error ? err.message : String(err));
      } finally {
        setLoading(false);
      }
    };

    // Initial render
    handleStateChange();

    // For alpha compatibility, we just do initial render
    // Real change detection would be implemented with proper state listeners
    
  }, [visualizationState, engine, converter, applyManualPositions]);

  // Separate effect to update positions when manual positions change (without re-running layout)
  useEffect(() => {
    if (baseReactFlowDataRef.current && manualPositions.size > 0) {
      console.log('[FlowGraph] 📍 Applying updated manual positions');
      const updatedData = applyManualPositions(baseReactFlowDataRef.current, manualPositions);
      setReactFlowData(updatedData);
    }
  }, [manualPositions, applyManualPositions]);

  // Handle node events
  const onNodeClick = useCallback((event: any, node: any) => {
    console.log('[FlowGraph] 🖱️ Node clicked:', node.id);
    eventHandlers?.onNodeClick?.(event, node);
  }, [eventHandlers]);

  // Handle edge events
  const onEdgeClick = useCallback((event: any, edge: any) => {
    console.log('[FlowGraph] 🖱️ Edge clicked:', edge.id);
    eventHandlers?.onEdgeClick?.(event, edge);
  }, [eventHandlers]);

  // Handle node drag events for debugging
  const onNodeDrag = useCallback((event: any, node: any) => {
    // Don't update positions during drag - let ReactFlow handle the visual updates
    // We'll only store the final position on drag stop
    eventHandlers?.onNodeDrag?.(event, node);
  }, [eventHandlers]);

  const onNodeDragStart = useCallback((event: any, node: any) => {
    console.log('[FlowGraph] 🖱️ Node drag start:', node.id);
  }, []);

  const onNodeDragStop = useCallback((event: any, node: any) => {
    console.log('[FlowGraph] 🖱️ Node drag stop:', node.id, 'final position:', node.position);
    
    // Store the manual position so it persists across re-renders
    setManualPositions(prev => {
      const newMap = new Map(prev);
      newMap.set(node.id, { x: node.position.x, y: node.position.y });
      return newMap;
    });
  }, []);

  // Handle ReactFlow node changes (including drag position updates)
  const onNodesChange = useCallback((changes: any[]) => {
    console.log('[FlowGraph] 📝 Nodes changing:', changes.length, 'changes');
    
    // Apply changes to current ReactFlow data
    if (reactFlowData) {
      setReactFlowData(prev => {
        if (!prev) return prev;
        
        const updatedNodes = prev.nodes.map(node => {
          // Find position changes for this node
          const positionChange = changes.find(change => 
            change.id === node.id && change.type === 'position'
          );
          
          if (positionChange && positionChange.position) {
            return {
              ...node,
              position: positionChange.position
            };
          }
          
          return node;
        });
        
        return {
          ...prev,
          nodes: updatedNodes
        };
      });
    }
  }, [reactFlowData]);

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
        nodes={reactFlowData?.nodes || []}
        edges={reactFlowData?.edges || []}
        nodeTypes={nodeTypes}
        edgeTypes={edgeTypes}
        onNodeClick={onNodeClick}
        onEdgeClick={onEdgeClick}
        onNodeDrag={onNodeDrag}
        onNodeDragStart={onNodeDragStart}
        onNodeDragStop={onNodeDragStop}
        onNodesChange={onNodesChange}
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
