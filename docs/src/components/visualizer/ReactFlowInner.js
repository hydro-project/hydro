/**
 * ReactFlow Inner Component
 * 
 * Core ReactFlow integration with custom node types
 */

import React, { useCallback, useMemo } from 'react';
import {
  ReactFlow,
  Controls,
  MiniMap,
  Background,
  Handle,
  addEdge,
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
import styles from '../../pages/visualizer.module.css';

export function ReactFlowInner({ nodes, edges, onNodesChange, onEdgesChange, colorPalette }) {
  const onConnect = useCallback((connection) => {
    onEdgesChange(addEdge(connection, edges));
  }, [onEdgesChange, edges]);

  const miniMapNodeColor = useCallback((node) => {
    return getMiniMapNodeColor(node, colorPalette);
  }, [colorPalette]);

  // Custom default node component to apply styling
  const DefaultNode = useCallback((props) => {
    const { data } = props;
    const nodeStyle = data?.nodeStyle || props.style || {};
    
    // Enhanced styling with subtle gradients and polish
    const baseBackground = nodeStyle.gradient || nodeStyle.background || '#f0f0f0';
    
    return (
      <div style={{ 
        background: baseBackground,
        width: nodeStyle.width || 200,
        height: nodeStyle.height || 60,
        borderRadius: '8px',
        color: nodeStyle.color || '#fff',
        fontSize: '13px',
        fontWeight: '600',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        textAlign: 'center',
        padding: '6px 10px',
        border: 'none',
        boxShadow: '0 4px 12px rgba(0,0,0,0.15), 0 2px 4px rgba(0,0,0,0.1)',
        transition: 'all 0.2s ease',
        cursor: 'grab',
        position: 'relative',
        overflow: 'hidden',
      }}>
        {/* Subtle highlight overlay for extra polish */}
        <div style={{
          position: 'absolute',
          top: 0,
          left: 0,
          right: 0,
          height: '50%',
          background: 'linear-gradient(to bottom, rgba(255,255,255,0.2), rgba(255,255,255,0))',
          borderRadius: '8px 8px 0 0',
          pointerEvents: 'none',
        }} />
        
        <span style={{ 
          position: 'relative',
          zIndex: 1,
          background: 'none',
          backgroundColor: 'transparent',
          border: 'none',
          outline: 'none',
          padding: '6px 10px',
          margin: 0,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          width: '100%',
          height: '100%',
          textShadow: '0 1px 2px rgba(0,0,0,0.3)',
        }}>
          {data?.label || 'Node'}
        </span>
        <Handle type="source" position="right" style={{ background: '#666', border: 'none', width: 8, height: 8, boxShadow: '0 2px 4px rgba(0,0,0,0.2)' }} />
        <Handle type="target" position="left" style={{ background: '#666', border: 'none', width: 8, height: 8, boxShadow: '0 2px 4px rgba(0,0,0,0.2)' }} />
      </div>
    );
  }, []);

  const nodeTypes = useMemo(() => ({
    group: GroupNode,
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
