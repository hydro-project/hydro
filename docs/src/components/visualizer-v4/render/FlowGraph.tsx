/**
 * @fileoverview Bridge-Based FlowGraph Component
 * 
 */

import React, { useRef, forwardRef, useImperativeHandle } from 'react';
import { ReactFlow, Background, Controls, MiniMap, ReactFlowProvider } from '@xyflow/react';
import '@xyflow/react/dist/style.css';

import { DEFAULT_RENDER_CONFIG } from './config';
import { nodeTypes } from './nodes';
import { edgeTypes } from './edges';
import { StyleConfigProvider } from './StyleConfigContext';
import { GraphDefs } from './GraphDefs';
import { LoadingView, ErrorView, EmptyView, UpdatingOverlay } from './FallbackViews';
import { useFlowGraphController } from '../hooks/useFlowGraphController';
import type { VisualizationState } from '../core/VisualizationState';
import type { RenderConfig, FlowGraphEventHandlers, LayoutConfig } from '../core/types';

export interface FlowGraphProps {
  visualizationState: VisualizationState;
  config?: RenderConfig;
  layoutConfig?: LayoutConfig;
  eventHandlers?: FlowGraphEventHandlers;
  className?: string;
  style?: React.CSSProperties;
  fillViewport?: boolean; // New prop to control viewport sizing
}

export interface FlowGraphRef {
  fitView: () => void;
  refreshLayout: () => Promise<void>;
}

// Internal component that uses ReactFlow hooks
const FlowGraphInternal = forwardRef<FlowGraphRef, FlowGraphProps>(({
  visualizationState,
  config = DEFAULT_RENDER_CONFIG,
  layoutConfig,
  eventHandlers,
  className,
  style,
  fillViewport = false
}, ref) => {
  const {
    reactFlowData,
    loading,
    error,
    refreshLayout,
    fitOnce,
    onNodeClick,
    onEdgeClick,
    onNodeDrag,
    onNodeDragStop,
    onNodesChange,
  } = useFlowGraphController({ visualizationState, config, layoutConfig, eventHandlers });

  useImperativeHandle(ref, () => ({
    fitView: () => { fitOnce(); },
    refreshLayout: async () => { await refreshLayout(); }
  }));

  // Calculate container styles based on fillViewport prop
  const getContainerStyle = (): React.CSSProperties => {
    if (fillViewport) {
      return {
        width: '100vw',
        height: '100vh',
        maxWidth: '100vw',
        maxHeight: '100vh',
        overflow: 'hidden',
        ...style
      };
    }
    return {
      width: '100%',
      height: '100%',
      minHeight: '400px',
      ...style
    };
  };

  // Loading state
  if (loading && !reactFlowData) {
    return <LoadingView className={className} containerStyle={getContainerStyle()} />;
  }

  // Error state
  if (error) {
    return <ErrorView className={className} containerStyle={getContainerStyle()} message={error} />;
  }

  // No data state
  if (!reactFlowData) {
    return <EmptyView className={className} containerStyle={getContainerStyle()} />;
  }

  // Main ReactFlow render
  return (
    <StyleConfigProvider value={{
      edgeStyle: config.edgeStyle,
      edgeColor: config.edgeColor,
      edgeWidth: config.edgeWidth,
      edgeDashed: config.edgeDashed,
      nodeBorderRadius: config.nodeBorderRadius,
      nodePadding: config.nodePadding,
      nodeFontSize: config.nodeFontSize,
      containerBorderRadius: config.containerBorderRadius,
      containerBorderWidth: config.containerBorderWidth,
      containerShadow: config.containerShadow
    }}>
    <div className={className} style={getContainerStyle()}>
  {/* Invisible SVG defs for edge filters/markers */}
  <GraphDefs />
      <ReactFlow
        nodes={reactFlowData?.nodes || []}
        edges={reactFlowData?.edges || []}
        nodeTypes={nodeTypes}
        edgeTypes={edgeTypes}
        onNodeClick={onNodeClick}
          onEdgeClick={onEdgeClick}
          onNodeDrag={onNodeDrag}
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
  {loading && <UpdatingOverlay />}
  </div>
  </StyleConfigProvider>
  );
});

FlowGraphInternal.displayName = 'FlowGraphInternal';

// Main FlowGraph component that provides ReactFlow context
export const FlowGraph = forwardRef<FlowGraphRef, FlowGraphProps>((props, ref) => {
  const flowGraphRef = useRef<FlowGraphRef>(null);
  
  // Expose fitView and refreshLayout methods through ref
  useImperativeHandle(ref, () => ({
    fitView: () => {
      if (flowGraphRef.current) {
        flowGraphRef.current.fitView();
      }
    },
    refreshLayout: async () => {
      if (flowGraphRef.current) {
        await flowGraphRef.current.refreshLayout();
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
