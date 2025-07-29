/**
 * ReactFlow Inner Component for v12
 * 
 * Reusable ReactFlow wrapper with common configuration
 */

import React, { useCallback, useMemo } from 'react';
import { 
  ReactFlow, 
  Controls, 
  MiniMap, 
  Background, 
  addEdge
} from '@xyflow/react';
import { Handle } from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import { 
  REACTFLOW_CONFIG, 
  MINIMAP_CONFIG, 
  BACKGROUND_CONFIG,
  DEFAULT_EDGE_OPTIONS,
  getMiniMapNodeColor
} from './reactFlowConfig.js';
import { GroupNode } from './GroupNode.js';
import styles from '../../pages/visualizer.module.css';

export function ReactFlowInner({ nodes, edges, onNodesChange, onEdgesChange, colorPalette }) {
  // Warn about any group node with missing style before rendering
  nodes.forEach(n => {
    if (n.type === 'group' && (!n.style || typeof n.style.width === 'undefined' || typeof n.style.height === 'undefined' || !n.id)) {
      console.warn('[ReactFlowInner] Invalid group node detected:', n);
      console.trace('[ReactFlowInner] Stack trace for invalid group node');
    }
  });
  const onConnect = useCallback((connection) => {
    onEdgesChange(addEdge(connection, edges));
  }, [onEdgesChange, edges]);

  const miniMapNodeColor = useCallback((node) => {
    return getMiniMapNodeColor(node, colorPalette);
  }, [colorPalette]);

  // Define custom node types: only use GroupNode for type 'group'
  // Fallback to default for others - pure gradient with no internal rectangles
  const DefaultNode = ({ data, style }) => (
    <div style={{ 
      ...style,
      // Override any internal styling that creates rectangles
      padding: 0,
      margin: 0,
      border: 'none',
      borderRadius: style?.borderRadius || '6px',
      background: style?.background || style?.gradient || '#f0f0f0',
      color: style?.color || '#fff',
      fontSize: style?.fontSize || '13px',
      fontWeight: style?.fontWeight || '500',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      textAlign: 'center',
      boxShadow: 'none',
      // Ensure no internal backgrounds
      backgroundColor: 'transparent',
      outline: 'none',
      // Remove any default ReactFlow node styling
      '--rfnode-color': 'transparent',
    }}>
      <span style={{ 
        // Ensure the text span has no styling that creates rectangles
        background: 'none',
        backgroundColor: 'transparent',
        border: 'none',
        outline: 'none',
        padding: style?.padding || '6px 10px',
        margin: 0,
        display: 'block',
        width: '100%',
        height: '100%',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
      }}>
        {data?.label || 'Node'}
      </span>
      <Handle type="source" position="right" style={{ background: '#666', border: 'none', width: 8, height: 8 }} />
      <Handle type="target" position="left" style={{ background: '#666', border: 'none', width: 8, height: 8 }} />
    </div>
  );
  const nodeTypes = useMemo(() => ({
    group: GroupNode,
    default: DefaultNode,
  }), []);

  // All console logs removed for focused debugging

  return (
    <div className={styles.reactflowWrapper}>
      <ReactFlow
        nodes={nodes.map(n => {
          if (n.type === 'group' && (!n.style || typeof n.style.width === 'undefined' || typeof n.style.height === 'undefined')) {
            // Fallback to default type if style is missing
            return { ...n, type: 'default' };
          }
          return { ...n, type: n.type === 'group' ? 'group' : 'default' };
        })}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onConnect={onConnect}
        nodeTypes={nodeTypes}
        defaultEdgeOptions={DEFAULT_EDGE_OPTIONS}
        {...REACTFLOW_CONFIG}
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
