/**
 * @fileoverview Bridge-Based FlowGraph Component
 * 
 * Complete replacement for alpha FlowGraph using o        console.log('[FlowGraph] ✅ Updated ReactFlow data:', {
          nodes: dataWithManualPositions.nodes.length,
          edges: dataWithManualPositions.edges.length
        });
        
        setReactFlowData(dataWithManualPositions);ture.
 * Maintains identical API while using the new VisualizationEngine internally.
 */

import React, { useEffect, useState, useCallback, useRef, forwardRef, useImperativeHandle } from 'react';
import { ReactFlow, Background, Controls, MiniMap, useReactFlow, ReactFlowProvider, applyNodeChanges } from '@xyflow/react';
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

// Internal component that uses ReactFlow hooks
const FlowGraphInternal = forwardRef<FlowGraphRef, FlowGraphProps>(({
  visualizationState,
  config = DEFAULT_RENDER_CONFIG,
  layoutConfig,
  eventHandlers,
  className,
  style
}, ref) => {
  const [reactFlowData, setReactFlowData] = useState<ReactFlowData | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  
  // ReactFlow instance for programmatic control
  const { fitView } = useReactFlow();
  
  // Expose fitView method through ref
  useImperativeHandle(ref, () => ({
    fitView: () => {
      try {
        fitView({ padding: 0.1, maxZoom: 1.2, duration: 300 });
        console.log('[FlowGraph] 🎯 Manual fit view called');
      } catch (err) {
        console.warn('[FlowGraph] ⚠️ Manual fit view failed:', err);
      }
    }
  }));
  
  // Track the last fit operation to prevent excessive fits
  const lastFitTimeRef = useRef<number>(0);
  const autoFitTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  
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

  // Cleanup timeout on unmount
  useEffect(() => {
    return () => {
      if (autoFitTimeoutRef.current) {
        clearTimeout(autoFitTimeoutRef.current);
      }
    };
  }, []);

  // Listen to layout config changes
  useEffect(() => {
    // Only run if we have all dependencies and data already exists
    if (layoutConfig && engine && converter && visualizationState && reactFlowData) {
      console.log('[FlowGraph] 🔧 Layout config changed, updating engine and re-running layout...');
      engine.updateLayoutConfig(layoutConfig, false); // Update config first
      
      // Trigger a re-layout with the new algorithm
      const runLayoutUpdate = async () => {
        try {
          setLoading(true);
          setError(null);
          
          // Run layout with new algorithm
          await engine.runLayout();
          
          // Convert to ReactFlow format
          const baseData = converter.convert(visualizationState);
          
          // Store the base data for reference
          baseReactFlowDataRef.current = baseData;
          
          // Apply any existing manual positions
          const dataWithManualPositions = applyManualPositions(baseData, visualizationState.getAllManualPositions());
          
          setReactFlowData(dataWithManualPositions);
          
          console.log('[FlowGraph] ✅ Layout change applied successfully');
          
        } catch (err) {
          console.error('[FlowGraph] ❌ Failed to apply layout change:', err);
          setError(err instanceof Error ? err.message : String(err));
        } finally {
          setLoading(false);
        }
      };
      
      runLayoutUpdate();
    }
  }, [layoutConfig]);

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
        
        // Auto-fit if enabled (with debouncing to prevent excessive fits)
        if (config.fitView !== false) {
          const now = Date.now();
          const timeSinceLastFit = now - lastFitTimeRef.current;
          
          // Clear any existing timeout
          if (autoFitTimeoutRef.current) {
            clearTimeout(autoFitTimeoutRef.current);
          }
          
          // Only fit if enough time has passed or this is a significant layout change
          const shouldFit = timeSinceLastFit > 500; // Minimum 500ms between fits
          
          autoFitTimeoutRef.current = setTimeout(() => {
            try {
              fitView({ padding: 0.1, maxZoom: 1.2, duration: 300 });
              lastFitTimeRef.current = Date.now();
              console.log('[FlowGraph] 🎯 Auto-fit applied');
            } catch (err) {
              console.warn('[FlowGraph] ⚠️ Auto-fit failed:', err);
            }
          }, shouldFit ? 100 : 300); // Short delay for immediate fits, longer for recent ones
        }
        
        // DEBUG: Log all container node data to find differences
        const containerNodes = dataWithManualPositions.nodes.filter(n => n.type === 'container');
        console.log('[FlowGraph] 🔍 CONTAINER NODES BEING PASSED TO REACTFLOW:');
        containerNodes.forEach(node => {
          console.log(`[FlowGraph] 📦 ${node.id}:`, {
            position: node.position,
            data: node.data,
            // extent: node.extent, // REMOVED: No longer using extent
            parentId: node.parentId
          });
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
    if (baseReactFlowDataRef.current && visualizationState.hasAnyManualPositions()) {
      const updatedData = applyManualPositions(baseReactFlowDataRef.current, visualizationState.getAllManualPositions());
      setReactFlowData(updatedData);
    }
  }, [visualizationState, applyManualPositions]);

  // Handle node events
  const onNodeClick = useCallback((event: any, node: any) => {
    eventHandlers?.onNodeClick?.(event, node);
  }, [eventHandlers]);

  // Handle edge events
  const onEdgeClick = useCallback((event: any, edge: any) => {
    eventHandlers?.onEdgeClick?.(event, edge);
  }, [eventHandlers]);

  // Handle node drag events for debugging
  const onNodeDrag = useCallback((event: any, node: any) => {
    // Don't update positions during drag - let ReactFlow handle the visual updates
    // We'll only store the final position on drag stop
    eventHandlers?.onNodeDrag?.(event, node);
  }, [eventHandlers]);

  const onNodeDragStart = useCallback((event: any, node: any) => {
    // Drag start - no action needed
  }, []);

  const onNodeDragStop = useCallback((event: any, node: any) => {
    // Store the manual position in VisualizationState
    visualizationState.setManualPosition(node.id, node.position.x, node.position.y);
    
    // Auto-fit if enabled (after a brief delay to let the position update settle)
    if (config.fitView !== false) {
      const now = Date.now();
      const timeSinceLastFit = now - lastFitTimeRef.current;
      
      // Clear any existing timeout
      if (autoFitTimeoutRef.current) {
        clearTimeout(autoFitTimeoutRef.current);
      }
      
      // Only auto-fit if enough time has passed since the last fit
      if (timeSinceLastFit > 500) {
        autoFitTimeoutRef.current = setTimeout(() => {
          try {
            fitView({ padding: 0.1, maxZoom: 1.2, duration: 300 });
            lastFitTimeRef.current = Date.now();
            console.log('[FlowGraph] 🎯 Auto-fit applied after drag');
          } catch (err) {
            console.warn('[FlowGraph] ⚠️ Auto-fit after drag failed:', err);
          }
        }, 200); // Brief delay to let drag position settle
      }
    }
  }, [visualizationState, config.fitView, fitView]);

  // Handle ReactFlow node changes (including drag position updates)
  const onNodesChange = useCallback((changes: any[]) => {
    // Apply changes using ReactFlow's built-in function
    if (reactFlowData) {
      setReactFlowData(prev => {
        if (!prev) return prev;
        
        // Use ReactFlow's built-in applyNodeChanges function
        const updatedNodes = applyNodeChanges(changes, prev.nodes);
        
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
          <div style={{ 
            width: '40px',
            height: '40px',
            margin: '0 auto 16px',
            border: '4px solid #f3f3f3',
            borderTop: '4px solid #3498db',
            borderRadius: '50%',
            animation: 'modernSpin 1s linear infinite'
          }}></div>
          <div style={{ fontSize: '18px', marginBottom: '8px' }}>
            Processing Graph Layout...
          </div>
          <div style={{ fontSize: '14px', color: '#999' }}>
            Large graphs may take a moment to compute
          </div>
        </div>
        <style>
          {`
            @keyframes modernSpin {
              0% { transform: rotate(0deg); }
              100% { transform: rotate(360deg); }
            }
          `}
        </style>
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
          fitView={false}
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
});

FlowGraphInternal.displayName = 'FlowGraphInternal';

// Main FlowGraph component that provides ReactFlow context
export const FlowGraph = forwardRef<FlowGraphRef, FlowGraphProps>((props, ref) => {
  const flowGraphRef = useRef<FlowGraphRef>(null);
  
  // Expose fitView method through ref
  useImperativeHandle(ref, () => ({
    fitView: () => {
      if (flowGraphRef.current) {
        flowGraphRef.current.fitView();
      }
    }
  }));

  return (
    <ReactFlowProvider>
      <FlowGraphInternal ref={flowGraphRef} {...props} />
    </ReactFlowProvider>
  );
});

FlowGraph.displayName = 'FlowGraph';
