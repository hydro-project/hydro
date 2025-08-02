/**
 * @fileoverview Main ReactFlow Visualization Component
 * 
 * The primary component that renders generic graphs using ReactFlow.
 * Independent of any specific framework - receives data via JSON/props.
 */

import React, { useState, useCallback, useEffect, useMemo, useRef } from 'react';
import {
  ReactFlow,
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
  useReactFlow,
  ConnectionLineType,
  ConnectionMode
} from '@xyflow/react';

import '@xyflow/react/dist/style.css';
import './styles.css';

import { VisualizationState } from '../shared/types';
import { ELKLayoutEngine } from '../layout/index';
import { LayoutConfig, DEFAULT_LAYOUT_CONFIG } from '../layout/index';
import { ReactFlowConverter } from './ReactFlowConverter';
import { applyNodeStyling } from './nodeStyler';
import { GraphStandardNode, GraphContainerNode } from './nodes';
import { GraphStandardEdge, GraphHyperEdge } from './edges';
import { 
  RenderConfig, 
  GraphFlowEventHandlers
} from './types';
import { DEFAULT_RENDER_CONFIG } from './config';
import { MINIMAP_CONFIG, PANEL_COLORS, TYPOGRAPHY } from '../shared/config';

// Node and Edge type definitions for ReactFlow
const nodeTypes = {
  'standard': GraphStandardNode,
  'container': GraphContainerNode
};

const edgeTypes = {
  'standard': GraphStandardEdge,
  'hyper': GraphHyperEdge
};

// Props for the main component
export interface GraphFlowProps {
  visualizationState: VisualizationState;
  metadata?: {
    nodeTypeConfig?: any;
    [key: string]: any;
  };
  layoutConfig?: Partial<LayoutConfig>;
  renderConfig?: Partial<RenderConfig>;
  eventHandlers?: Partial<GraphFlowEventHandlers>;
  onLayoutComplete?: () => void;
  onError?: (error: Error) => void;
  className?: string;
  style?: React.CSSProperties;
}

// Internal component that uses ReactFlow hooks
const GraphFlowInternal: React.FC<GraphFlowProps> = ({
  visualizationState,
  metadata,
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
  const lastStateRef = useRef<string>('');

  // Merge configs with defaults
  const finalLayoutConfig = useMemo(() => ({
    ...DEFAULT_LAYOUT_CONFIG,
    ...layoutConfig
  }), [layoutConfig]);

  const finalRenderConfig = useMemo(() => ({
    ...DEFAULT_RENDER_CONFIG,
    ...renderConfig
  }), [renderConfig]);

  // Container collapse handler - defined early to be used in layoutAndRender
  const handleContainerCollapse = useCallback(async (containerId: string) => {
    try {
      // Collapse the container in the visualization state
      visualizationState.collapseContainer(containerId);
      
    } catch (error) {
      console.error('Container collapse error:', error);
      onError?.(error instanceof Error ? error : new Error('Container collapse failed'));
    }
  }, [visualizationState, onError]);

  // Container expand handler - defined early to be used in layoutAndRender  
  const handleContainerExpand = useCallback(async (containerId: string) => {
    try {
      // Only log container operations for debugging purposes
      // Only log container operations in development
      if (process.env.NODE_ENV === 'development') {
        console.log(`Expanding container: ${containerId}`);
      }
      
      // Expand the container in the visualization state
      visualizationState.expandContainer(containerId);
      
    } catch (error) {
      console.error('Container expand error:', error);
      onError?.(error instanceof Error ? error : new Error('Container expand failed'));
    }
  }, [visualizationState, onError]);

  // Layout and render the graph
  const layoutAndRender = useCallback(async () => {
    try {
      setIsLayouting(true);
      
      // Only log layout start in development
      if (process.env.NODE_ENV === 'development') {
        console.log('Starting layout process...');
      }
      
      // Get visible elements from visualization state
      const visibleNodes = visualizationState.visibleNodes;
      const visibleEdges = visualizationState.visibleEdges;
      const visibleContainers = visualizationState.visibleContainers;
      const hyperEdges = visualizationState.allHyperEdges;

      // Only log layout details in development
      if (process.env.NODE_ENV === 'development') {
        console.log('Visible elements:', {
          nodes: visibleNodes.length,
          containers: visibleContainers.length,
          edges: visibleEdges.length,
          hyperEdges: hyperEdges.length
        });
      }

      // Run layout
      const layoutResult = await layoutEngine.layout(
        visibleNodes,
        visibleEdges,
        visibleContainers,
        hyperEdges,
        finalLayoutConfig
      );

      // Convert to ReactFlow format
      const reactFlowData = ReactFlowConverter.convert(layoutResult);
      
      // Apply node styling with nodeTypeConfig (similar to visualizer approach)
      const styledNodes = applyNodeStyling(
        reactFlowData.nodes, 
        'Set2', // Use Set2 for better contrast and legibility
        metadata?.nodeTypeConfig
      );
      
      // Add collapse/expand callbacks to container nodes
      const nodesWithCallbacks = styledNodes.map(node => {
        if (node.type === 'container') {
          return {
            ...node,
            data: {
              ...node.data,
              onContainerCollapse: handleContainerCollapseWithLayout,
              onContainerExpand: handleContainerExpandWithLayout
            }
          };
        }
        return node;
      });
      
      // Update nodes and edges
      setNodes(nodesWithCallbacks);
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
    fitView,
    finalRenderConfig.fitView,
    onLayoutComplete,
    onError,
    metadata,
    handleContainerCollapse,
    handleContainerExpand
  ]);

  // Update container handlers to trigger re-layout after state change
  const handleContainerCollapseWithLayout = useCallback(async (containerId: string) => {
    await handleContainerCollapse(containerId);
    await layoutAndRender();
  }, [handleContainerCollapse, layoutAndRender]);

  const handleContainerExpandWithLayout = useCallback(async (containerId: string) => {
    await handleContainerExpand(containerId);
    await layoutAndRender();
  }, [handleContainerExpand, layoutAndRender]);

  // Trigger layout when visualization state changes
  useEffect(() => {
    // Create a serializable representation of the state to detect actual changes
    const stateKey = JSON.stringify({
      nodes: visualizationState.visibleNodes.map(n => ({ id: n.id, hidden: n.hidden })),
      edges: visualizationState.visibleEdges.map(e => ({ id: e.id, hidden: e.hidden })),
      containers: visualizationState.visibleContainers.map(c => ({ 
        id: c.id, 
        hidden: c.hidden, 
        collapsed: c.collapsed,
        children: Array.from(c.children) 
      }))
    });
    
    // Only run layout if state actually changed
    if (stateKey !== lastStateRef.current) {
      // Only log state changes for container collapse debugging
      // Only log state changes in development
      if (process.env.NODE_ENV === 'development') {
        console.log('Visualization state changed, triggering layout');
      }
      lastStateRef.current = stateKey;
      layoutAndRender();
    } else if (process.env.NODE_ENV === 'development') {
      // Only log skipped layouts in development
      if (process.env.NODE_ENV === 'development') {
        console.log('Visualization state unchanged, skipping layout');
      }
    }
  }, [visualizationState.visibleNodes, visualizationState.visibleEdges, visualizationState.visibleContainers, layoutAndRender]);

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

  // Debug logging for ReactFlow data
  if (process.env.NODE_ENV === 'development' && edges.length > 0) {
    console.log(`📊 [GRAPHFLOW DEBUG] Rendering ReactFlow with ${nodes.length} nodes and ${edges.length} edges`);
    console.log(`🏹 [GRAPHFLOW DEBUG] Sample edge:`, edges[0]);
    console.log(`🏹 [GRAPHFLOW DEBUG] All edges:`, edges.map(e => ({ 
      id: e.id, 
      type: e.type, 
      hasMarkerEnd: !!e.markerEnd,
      markerEnd: e.markerEnd 
    })));
  }

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
        connectionLineType={ConnectionLineType.SmoothStep} // Smooth edge routing for better handle utilization
        connectionMode={ConnectionMode.Loose} // Allow connections to any handle on a node
        defaultEdgeOptions={{
          type: 'smoothstep',
          animated: false,
          style: { strokeWidth: 2 }
        }}
      >
        {finalRenderConfig.enableControls && (
          <Controls
            position="bottom-left"
            showZoom={true}
            showFitView={true}
            showInteractive={false}
            style={{ zIndex: 1000 }}
          />
        )}
        
        {finalRenderConfig.enableMiniMap && (
          <MiniMap
            position="bottom-right"
            nodeStrokeColor={MINIMAP_CONFIG.NODE_STROKE_COLOR}
            nodeColor={MINIMAP_CONFIG.NODE_COLOR}
            nodeBorderRadius={MINIMAP_CONFIG.NODE_BORDER_RADIUS}
            pannable
            zoomable
          />
        )}
        
        {/* Background component - temporarily simplified */}
        <Background />

        {/* Layout status panel */}
        {isLayouting && (
          <Panel position="top-center">
            <div style={{
              background: PANEL_COLORS.BACKGROUND,
              border: `1px solid ${PANEL_COLORS.BORDER}`,
              borderRadius: '6px',
              padding: '8px 16px',
              fontSize: TYPOGRAPHY.FONT_SIZES.MD,
              color: PANEL_COLORS.TEXT,
              fontWeight: TYPOGRAPHY.FONT_WEIGHTS.MEDIUM
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
export const GraphFlow: React.FC<GraphFlowProps> = (props) => {
  return (
    <ReactFlowProvider>
      <GraphFlowInternal {...props} />
    </ReactFlowProvider>
  );
};

export default GraphFlow;
