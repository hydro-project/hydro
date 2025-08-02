/**
 * @fileoverview Main ReactFlow Visualization Component
 * 
 * The primary component that renders generic graphs using ReactFlow.
 * Independent of any specific framework - receives data via JSON/props.
 */

import React, { useState, useCallback, useEffect, useMemo, useRef } from 'react';
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

import { VisualizationState } from '../shared/types';
import { ELKLayoutEngine } from '../layout/ELKLayoutEngine';
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

  // Layout and render the graph
  const layoutAndRender = useCallback(async () => {
    try {
      setIsLayouting(true);
      
      console.log('Starting layout process...');
      
      // Get visible elements from visualization state
      const visibleNodes = visualizationState.visibleNodes;
      const visibleEdges = visualizationState.visibleEdges;
      const visibleContainers = visualizationState.visibleContainers;
      const hyperEdges = visualizationState.allHyperEdges;

      console.log('Visible elements:', {
        nodes: visibleNodes,
        edges: visibleEdges,
        containers: visibleContainers,
        hyperEdges: hyperEdges
      });

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
        'Set3', // TODO: Make this configurable
        metadata?.nodeTypeConfig
      );
      
      console.log('Layout result:', layoutResult);
      console.log('ReactFlow data:', reactFlowData);
      console.log('Styled nodes:', styledNodes);
      
      // Update nodes and edges
      setNodes(styledNodes);
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
    onError
  ]);

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
      console.log('Visualization state changed, triggering layout');
      lastStateRef.current = stateKey;
      layoutAndRender();
    } else {
      console.log('Visualization state unchanged, skipping layout');
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
