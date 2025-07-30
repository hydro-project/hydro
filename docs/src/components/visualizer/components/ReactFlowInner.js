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
} from '../utils/reactFlowConfig.js';
import { GroupNode } from './GroupNode.js';
import { CollapsedContainerNode } from '../containers/CollapsedContainerNode.js';
import { enforceHandleConsistency, REQUIRED_HANDLE_IDS } from '../utils/handleValidation.js';
import styles from '../../../pages/visualizer.module.css';

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
        {/* 
          CRITICAL: These handle IDs must match GroupNode and CollapsedContainerNode
          to prevent ReactFlow "Couldn't create edge for handle id" errors
        */}
        <Handle id={REQUIRED_HANDLE_IDS.source} type="source" position="right" style={{ background: '#666', border: 'none', width: 8, height: 8 }} />
        <Handle id={REQUIRED_HANDLE_IDS.target} type="target" position="left" style={{ background: '#666', border: 'none', width: 8, height: 8 }} />
        <Handle id={REQUIRED_HANDLE_IDS.sourceBottom} type="source" position="bottom" style={{ background: '#666', border: 'none', width: 8, height: 8 }} />
        <Handle id={REQUIRED_HANDLE_IDS.targetTop} type="target" position="top" style={{ background: '#666', border: 'none', width: 8, height: 8 }} />
      </div>
    );
  }, []);

  const nodeTypes = useMemo(() => {
    const types = {
      group: GroupNode,
      collapsedContainer: CollapsedContainerNode,
      default: DefaultNode,
    };
    
    // CRITICAL: Log handle requirements during development
    // This helps prevent ReactFlow handle errors by documenting the requirements
    if (process.env.NODE_ENV === 'development') {
      enforceHandleConsistency(types);
    }
    
    return types;
  }, [DefaultNode]);

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
