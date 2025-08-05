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
  ConnectionMode,
  MarkerType
} from '@xyflow/react';

import '@xyflow/react/dist/style.css';
import './styles.css';

import { VisualizationState } from '../core/VisState';
import { getVisualizationService, isEncapsulatedReactFlowData } from '../services/VisualizationService';
import { LayoutConfig, DEFAULT_LAYOUT_CONFIG } from '../layout/index';
import { applyNodeStyling } from './nodeStyler';
import { GraphStandardNode, GraphContainerNode } from './nodes';
import { GraphStandardEdge, GraphHyperEdge } from './edges';
import { 
  RenderConfig, 
  FlowGraphEventHandlers
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
export interface FlowGraphProps {
  visualizationState: VisualizationState;
  metadata?: {
    nodeTypeConfig?: any;
    [key: string]: any;
  };
  layoutConfig?: Partial<LayoutConfig>;
  renderConfig?: Partial<RenderConfig>;
  eventHandlers?: Partial<FlowGraphEventHandlers>;
  onLayoutComplete?: () => void;
  onError?: (error: Error) => void;
  className?: string;
  style?: React.CSSProperties;
}

// Internal component that uses ReactFlow hooks
const FlowGraphInternal: React.FC<FlowGraphProps> = ({
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
  const [visualizationService] = useState(() => getVisualizationService());
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

  // Layout and render the graph using ONLY VisState
  const layoutAndRender = useCallback(async () => {
    try {
      setIsLayouting(true);
      
      // Only log layout start in development
      if (process.env.NODE_ENV === 'development') {
        console.log('[FlowGraph] 🎯 Starting layout process using VisualizationService...');
      }
      
      // CRITICAL: Use service to ensure VisState-only data flow
      const reactFlowData = await visualizationService.layoutAndRender(
        visualizationState,
        finalLayoutConfig
      );

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
      
      // 🔥 FINAL REACTFLOW DATA LOGGING - What actually gets passed to ReactFlow
      console.log('[FlowGraph] 🔥 FINAL DATA PASSED TO REACTFLOW:');
      console.log(`  📊 SUMMARY: ${nodesWithCallbacks.length} nodes, ${reactFlowData.edges.length} edges`);
      
      // Log hyperedge nodes and edges that ReactFlow will actually render
      const finalHyperEdges = reactFlowData.edges.filter(e => e.type === 'hyper');
      const finalHyperEdgeNodeIds = new Set();
      finalHyperEdges.forEach(edge => {
        finalHyperEdgeNodeIds.add(edge.source);
        finalHyperEdgeNodeIds.add(edge.target);
      });
      
      console.log(`  🔘 FINAL NODES involved in hyperedges:`);
      nodesWithCallbacks.forEach(node => {
        if (finalHyperEdgeNodeIds.has(node.id)) {
          console.log(`    ${node.id} (${node.type}): pos=(${node.position?.x || 0}, ${node.position?.y || 0}), size=${node.width || 'auto'}x${node.height || 'auto'}`);
        }
      });
      
      console.log(`  🔥 FINAL HYPEREDGES that ReactFlow will render:`);
      finalHyperEdges.forEach(edge => {
        console.log(`    ${edge.id}: ${edge.source} → ${edge.target}`);
        
        // Find the actual final nodes
        const finalSourceNode = nodesWithCallbacks.find(n => n.id === edge.source);
        const finalTargetNode = nodesWithCallbacks.find(n => n.id === edge.target);
        
        if (finalSourceNode && finalTargetNode) {
          const finalSourcePos = finalSourceNode.position || { x: 0, y: 0 };
          const finalTargetPos = finalTargetNode.position || { x: 0, y: 0 };
          console.log(`      📍 FINAL REACTFLOW POSITIONS: ${edge.source}(${finalSourcePos.x}, ${finalSourcePos.y}) → ${edge.target}(${finalTargetPos.x}, ${finalTargetPos.y})`);
          
          // Calculate distance
          const dx = finalTargetPos.x - finalSourcePos.x;
          const dy = finalTargetPos.y - finalSourcePos.y;
          const distance = Math.sqrt(dx * dx + dy * dy);
          console.log(`      📏 FINAL DISTANCE: ${distance.toFixed(2)}px`);
          
          if (distance < 10) {
            console.log(`      ⚠️  FINAL WARNING: Hyperedge endpoints are very close/overlapping in final ReactFlow data!`);
          }
        } else {
          console.log(`      ❌ FINAL ERROR: Could not find final ReactFlow nodes for hyperedge endpoints`);
          console.log(`        Final Source ${edge.source}: ${finalSourceNode ? 'FOUND' : 'NOT FOUND'}`);
          console.log(`        Final Target ${edge.target}: ${finalTargetNode ? 'FOUND' : 'NOT FOUND'}`);
        }
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
      console.error('[FlowGraph] Layout error:', error);
      onError?.(error instanceof Error ? error : new Error('Layout failed'));
    } finally {
      setIsLayouting(false);
    }
  }, [
    visualizationState,
    visualizationService,
    finalLayoutConfig,
    fitView,
    finalRenderConfig.fitView,
    onLayoutComplete,
    onError,
    metadata,
    handleContainerCollapse,
    handleContainerExpand
  ]);

  // Selective layout for container changes (sets up VisState flags, then calls regular layout)
  const layoutAndRenderSelective = useCallback(async (changedContainerId: string) => {
    try {
      setIsLayouting(true);
      
      if (process.env.NODE_ENV === 'development') {
        console.log(`Starting selective layout for container: ${changedContainerId}`);
      }
      
      // STEP 1: Set up fixed/free flags in VisState using VisState APIs
      if (process.env.NODE_ENV === 'development') {
        console.log(`Setting up position fixing for container: ${changedContainerId}`);
      }
      
      // Use VisState API to set up container position fixing
      visualizationState.getContainersRequiringLayout(changedContainerId);
      
      // STEP 2: Call regular layout (which will read the fixed/free flags from VisState)
      const reactFlowData = await visualizationService.layoutAndRender(
        visualizationState,
        finalLayoutConfig
        // NO changedContainerId - just regular layout that reads VisState flags
      );

      // Apply node styling
      const styledNodes = applyNodeStyling(
        reactFlowData.nodes, 
        'Set2',
        metadata?.nodeTypeConfig
      );
      
      // Add collapse/expand callbacks to container nodes
      const nodesWithCallbacks = styledNodes.map(node => {
        if (node.type === 'container') {
          return {
            ...node,
            data: {
              ...node.data,
              onContainerCollapse: (id: string) => handleContainerCollapse(id).then(() => layoutAndRenderSelective(id)),
              onContainerExpand: (id: string) => handleContainerExpand(id).then(() => layoutAndRenderSelective(id))
            }
          };
        }
        return node;
      });

      // 🔥 FINAL REACTFLOW DATA LOGGING - What actually gets passed to ReactFlow (SELECTIVE)
      console.log('[FlowGraph] 🔥 FINAL DATA PASSED TO REACTFLOW (SELECTIVE):');
      console.log(`  📊 SUMMARY: ${nodesWithCallbacks.length} nodes, ${reactFlowData.edges.length} edges`);
      
      // Log hyperedge nodes and edges that ReactFlow will actually render
      const finalHyperEdges = reactFlowData.edges.filter(e => e.type === 'hyper');
      const finalHyperEdgeNodeIds = new Set();
      finalHyperEdges.forEach(edge => {
        finalHyperEdgeNodeIds.add(edge.source);
        finalHyperEdgeNodeIds.add(edge.target);
      });
      
      console.log(`  🔘 FINAL NODES involved in hyperedges:`);
      nodesWithCallbacks.forEach(node => {
        if (finalHyperEdgeNodeIds.has(node.id)) {
          console.log(`    ${node.id} (${node.type}): pos=(${node.position?.x || 0}, ${node.position?.y || 0}), size=${node.width || 'auto'}x${node.height || 'auto'}`);
        }
      });
      
      console.log(`  🔥 FINAL HYPEREDGES that ReactFlow will render:`);
      finalHyperEdges.forEach(edge => {
        console.log(`    ${edge.id}: ${edge.source} → ${edge.target}`);
        
        // Find the actual final nodes
        const finalSourceNode = nodesWithCallbacks.find(n => n.id === edge.source);
        const finalTargetNode = nodesWithCallbacks.find(n => n.id === edge.target);
        
        if (finalSourceNode && finalTargetNode) {
          const finalSourcePos = finalSourceNode.position || { x: 0, y: 0 };
          const finalTargetPos = finalTargetNode.position || { x: 0, y: 0 };
          console.log(`      📍 FINAL REACTFLOW POSITIONS: ${edge.source}(${finalSourcePos.x}, ${finalSourcePos.y}) → ${edge.target}(${finalTargetPos.x}, ${finalTargetPos.y})`);
          
          // Calculate distance
          const dx = finalTargetPos.x - finalSourcePos.x;
          const dy = finalTargetPos.y - finalSourcePos.y;
          const distance = Math.sqrt(dx * dx + dy * dy);
          console.log(`      📏 FINAL DISTANCE: ${distance.toFixed(2)}px`);
          
          if (distance < 10) {
            console.log(`      ⚠️  FINAL WARNING: Hyperedge endpoints are very close/overlapping in final ReactFlow data!`);
          }
        } else {
          console.log(`      ❌ FINAL ERROR: Could not find final ReactFlow nodes for hyperedge endpoints`);
          console.log(`        Final Source ${edge.source}: ${finalSourceNode ? 'FOUND' : 'NOT FOUND'}`);
          console.log(`        Final Target ${edge.target}: ${finalTargetNode ? 'FOUND' : 'NOT FOUND'}`);
        }
      });

      // Update the graph
      setNodes(nodesWithCallbacks);
      setEdges(reactFlowData.edges);

      // Auto-fit if enabled
      if (finalRenderConfig.fitView) {
        setTimeout(() => {
          fitView({ duration: 500, padding: 0.1 });
        }, 100);
      }

      onLayoutComplete?.();

    } catch (error) {
      console.error('Selective layout error:', error);
      onError?.(error instanceof Error ? error : new Error('Selective layout failed'));
    } finally {
      setIsLayouting(false);
    }
  }, [
    visualizationState,
    visualizationService,
    finalLayoutConfig,
    finalRenderConfig.fitView,
    fitView,
    onLayoutComplete,
    onError,
    metadata,
    handleContainerCollapse,
    handleContainerExpand
  ]);

  // Update container handlers to trigger selective re-layout after state change
  const handleContainerCollapseWithLayout = useCallback(async (containerId: string) => {
    await handleContainerCollapse(containerId);
    await layoutAndRenderSelective(containerId);
  }, [handleContainerCollapse]);

  const handleContainerExpandWithLayout = useCallback(async (containerId: string) => {
    await handleContainerExpand(containerId);
    await layoutAndRenderSelective(containerId);
  }, [handleContainerExpand]);

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
          style: { strokeWidth: 2 },
          markerEnd: {
            type: MarkerType.ArrowClosed,
            width: 15,
            height: 15,
            color: '#999'
          }
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
export const FlowGraph: React.FC<FlowGraphProps> = (props) => {
  return (
    <ReactFlowProvider>
      <FlowGraphInternal {...props} />
    </ReactFlowProvider>
  );
};

export default FlowGraph;
