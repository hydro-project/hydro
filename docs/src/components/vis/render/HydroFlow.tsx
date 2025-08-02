/**
 * @fileoverview Main ReactFlow Visualization Component
 * 
 * The primary component that renders the Hydro graph using ReactFlow.
 */

import React, { useCallback, useMemo, useState, useEffect } from 'react';
import ReactFlow, {
  Controls,
  MiniMap,
  Background,
  useNodesState,
  useEdgesState,
  addEdge,
  Connection,
  Edge,
  Node,
  ReactFlowProvider,
  Panel,
  useReactFlow
} from 'reactflow';

import 'reactflow/dist/style.css';

import { VisualizationState as IVisualizationState } from '../shared/types.js';
import { ELKLayoutEngine } from '../layout/ELKLayoutEngine.js';
import { LayoutConfig, DEFAULT_LAYOUT_CONFIG } from '../layout/index.js';
import { ReactFlowConverter } from './ReactFlowConverter.js';
import { HydroStandardNode, HydroContainerNode } from './nodes.js';
import { HydroStandardEdge, HydroHyperEdge } from './edges.js';
import { 
  RenderConfig, 
  HydroFlowEventHandlers,
  HYDRO_NODE_TYPES,
  HYDRO_EDGE_TYPES
} from './types.js';
import { DEFAULT_RENDER_CONFIG } from './config.js';

// Node and Edge type definitions for ReactFlow
const nodeTypes = {
  [HYDRO_NODE_TYPES.STANDARD]: HydroStandardNode,
  [HYDRO_NODE_TYPES.CONTAINER]: HydroContainerNode
};

const edgeTypes = {
  [HYDRO_EDGE_TYPES.STANDARD]: HydroStandardEdge,
  [HYDRO_EDGE_TYPES.HYPER]: HydroHyperEdge
};

// Props for the main component
export interface HydroFlowProps {
  visualizationState: IVisualizationState;
  layoutConfig?: Partial<LayoutConfig>;
  renderConfig?: Partial<RenderConfig>;
  eventHandlers?: Partial<HydroFlowEventHandlers>;
  onLayoutComplete?: () => void;
  onError?: (error: Error) => void;
  className?: string;
  style?: React.CSSProperties;
}

// Internal component that uses ReactFlow hooks
const HydroFlowInternal: React.FC<HydroFlowProps> = ({
  visualizationState,
  layoutConfig = {},
  renderConfig = {},
  eventHandlers = {},
  onLayoutComplete,
  onError,
  className,
  style
}) => {
  const [nodes, setNodes, onNodesChange] = useNodesState([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState([]);
  const [isLayouting, setIsLayouting] = useState(false);
  const [layoutEngine] = useState(() => new ELKLayoutEngine());
  const { fitView } = useReactFlow();

  // Merge configs with defaults
  const finalLayoutConfig = useMemo(() => ({
    ...DEFAULT_LAYOUT_CONFIG,
    ...layoutConfig
  }), [layoutConfig]);

  const finalRenderConfig = useMemo(() => ({
    ...DEFAULT_RENDER_CONFIG,
    ...renderConfig
  }), [renderConfig]);

  // Layout and render the graph
  const layoutAndRender = useCallback(async () => {
    try {
      setIsLayouting(true);
      
      // Get visible elements from visualization state
      const visibleNodes = visualizationState.getVisibleNodes();
      const visibleEdges = visualizationState.getVisibleEdges();
      const visibleContainers = visualizationState.getVisibleContainers();
      const hyperEdges = visualizationState.getHyperEdges();

      // Run layout
      const layoutResult = await layoutEngine.layout(
        visibleNodes,
        visibleEdges,
        visibleContainers,
        hyperEdges,
        finalLayoutConfig
      );

      // Convert to ReactFlow format
      const reactFlowData = ReactFlowConverter.convert(layoutResult, eventHandlers);
      
      // Update nodes and edges
      setNodes(reactFlowData.nodes);
      setEdges(reactFlowData.edges);

      // Fit view after a short delay to ensure rendering is complete
      setTimeout(() => {
        if (finalRenderConfig.fitView) {
          fitView({ padding: 0.1 });
        }
        onLayoutComplete?.();
      }, 100);

    } catch (error) {
      console.error('Layout error:', error);
      onError?.(error instanceof Error ? error : new Error('Layout failed'));
    } finally {
      setIsLayouting(false);
    }
  }, [
    visualizationState,
    layoutEngine,
    finalLayoutConfig,
    eventHandlers,
    fitView,
    finalRenderConfig.fitView,
    onLayoutComplete,
    onError,
    setNodes,
    setEdges
  ]);

  // Trigger layout when visualization state changes
  useEffect(() => {
    layoutAndRender();
  }, [layoutAndRender]);

  // Handle connection creation (if enabled)
  const onConnect = useCallback(
    (params: Connection) => {
      if (eventHandlers.onConnect) {
        eventHandlers.onConnect(params);
      } else {
        setEdges((eds) => addEdge(params, eds));
      }
    },
    [eventHandlers, setEdges]
  );

  // Handle selection changes
  const onSelectionChange = useCallback(
    (selection: { nodes: Node[]; edges: Edge[] }) => {
      if (eventHandlers.onSelectionChange) {
        eventHandlers.onSelectionChange(selection);
      }
    },
    [eventHandlers]
  );

  return (
    <div className={`hydro-flow ${className || ''}`} style={{ width: '100%', height: '100%', ...style }}>
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onConnect={onConnect}
        onSelectionChange={onSelectionChange}
        onNodeClick={eventHandlers.onNodeClick}
        onNodeDoubleClick={eventHandlers.onNodeDoubleClick}
        onNodeContextMenu={eventHandlers.onNodeContextMenu}
        onNodeDrag={eventHandlers.onNodeDrag}
        onNodeDragStop={eventHandlers.onNodeDragStop}
        onEdgeClick={eventHandlers.onEdgeClick}
        onEdgeContextMenu={eventHandlers.onEdgeContextMenu}
        onEdgeUpdate={eventHandlers.onEdgeUpdate}
        onPaneClick={eventHandlers.onPaneClick}
        onPaneContextMenu={eventHandlers.onPaneContextMenu}
        nodeTypes={nodeTypes}
        edgeTypes={edgeTypes}
        fitView={finalRenderConfig.fitView}
        snapToGrid={finalRenderConfig.snapToGrid}
        snapGrid={[finalRenderConfig.gridSize, finalRenderConfig.gridSize]}
        nodesDraggable={finalRenderConfig.nodesDraggable}
        nodesConnectable={finalRenderConfig.nodesConnectable}
        elementsSelectable={finalRenderConfig.elementsSelectable}
        zoomOnScroll={finalRenderConfig.enableZoom}
        panOnScroll={finalRenderConfig.enablePan}
        selectNodesOnDrag={finalRenderConfig.enableSelection}
        deleteKeyCode={null} // Disable delete key
        multiSelectionKeyCode={['Meta', 'Ctrl']}
      >
        {finalRenderConfig.enableControls && (
          <Controls
            position="top-left"
            showZoom={finalRenderConfig.enableZoom}
            showFitView={finalRenderConfig.fitView}
            showInteractive={false}
          />
        )}
        
        {finalRenderConfig.enableMiniMap && (
          <MiniMap
            position="bottom-right"
            nodeStrokeColor="#374151"
            nodeColor="#e5e7eb"
            nodeBorderRadius={4}
            pannable
            zoomable
          />
        )}
        
        {finalRenderConfig.showBackground && (
          <Background
            variant={finalRenderConfig.backgroundVariant as any}
            gap={finalRenderConfig.gridSize}
            size={1}
            color="#f3f4f6"
            style={{ backgroundColor: finalRenderConfig.backgroundColor }}
          />
        )}

        {/* Layout status panel */}
        {isLayouting && (
          <Panel position="top-center">
            <div style={{
              background: 'rgba(0, 123, 255, 0.1)',
              border: '1px solid #007bff',
              borderRadius: '6px',
              padding: '8px 16px',
              fontSize: '14px',
              color: '#007bff',
              fontWeight: 500
            }}>
              Computing layout...
            </div>
          </Panel>
        )}
      </ReactFlow>
    </div>
  );
};

// Main exported component with ReactFlow provider
export const HydroFlow: React.FC<HydroFlowProps> = (props) => {
  return (
    <ReactFlowProvider>
      <HydroFlowInternal {...props} />
    </ReactFlowProvider>
  );
};

export default HydroFlow;
