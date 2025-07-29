/**
 * ReactFlow Inner Compoexport function ReactFlowInner({ nodes, edges, onNodesChange, onEdgesChange, onNodeClick, colorPalette }) {
  const reactFlowInstance = useReactFlow();
  
  // Store instance globally for access from Visualizer
  React.useEffect(() => {
    window.reactFlowInstance = reactFlowInstance;
  }, [reactFlowInstance]);

  const onConnect = useCallback((connection) => {
    onEdgesChange(addEdge(connection, edges));
  }, [onEdgesChange, edges]);
 * 
 * Core ReactFlow integration with custom node types
 */

import React, { useCallback, useMemo, useRef } from 'react';
import {
  ReactFlow,
  Controls,
  MiniMap,
  Background,
  Handle,
  addEdge,
  useReactFlow,
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import {
  DEFAULT_EDGE_OPTIONS,
  REACTFLOW_CONFIG,
  BACKGROUND_CONFIG,
  MINIMAP_CONFIG,
  getMiniMapNodeColor
} from './reactFlowConfig.js';
import { GroupNode } from './GroupNode.js';
import { CollapsedContainerNode } from './CollapsedContainerNode.js';
import styles from '../../pages/visualizer.module.css';

export function ReactFlowInner({ nodes, edges, onNodesChange, onEdgesChange, colorPalette, onNodeClick }) {
  const onConnect = useCallback((connection) => {
    onEdgesChange(addEdge(connection, edges));
  }, [onEdgesChange, edges]);

  const miniMapNodeColor = useCallback((node) => {
    return getMiniMapNodeColor(node, colorPalette);
  }, [colorPalette]);

  // Custom default node component - simplified to fill the container
  const DefaultNode = useCallback((props) => {
    const { data } = props;
    const nodeStyle = data?.nodeStyle || props.style || {};
    
    // Use the background from nodeStyle
    const background = nodeStyle.gradient || nodeStyle.background || '#f0f0f0';
    
    return (
      <div style={{ 
        background: background,
        width: '100%',
        height: '100%',
        borderRadius: '8px',
        color: nodeStyle.color || '#fff',
        fontSize: '13px',
        fontWeight: '600',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        textAlign: 'center',
        cursor: 'grab',
        padding: '6px 10px',
        boxSizing: 'border-box',
      }}>
        {data?.label || 'Node'}
        <Handle type="source" position="right" style={{ background: '#666', border: 'none', width: 8, height: 8 }} />
        <Handle type="target" position="left" style={{ background: '#666', border: 'none', width: 8, height: 8 }} />
      </div>
    );
  }, []);

  const nodeTypes = useMemo(() => ({
    group: GroupNode,
    collapsedContainer: CollapsedContainerNode,
    default: DefaultNode,
  }), [DefaultNode]);

  return (
    <div className={styles.reactflowWrapper}>
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onConnect={onConnect}
        onNodeClick={onNodeClick}
        nodeTypes={nodeTypes}
        defaultEdgeOptions={DEFAULT_EDGE_OPTIONS}
        {...REACTFLOW_CONFIG}
        nodesDraggable={true}
      >
        <Controls />
        <MiniMap 
          nodeColor={miniMapNodeColor}
          {...MINIMAP_CONFIG}
        />
        <Background {...BACKGROUND_CONFIG} />
      </ReactFlow>
    </div>
  );
}
