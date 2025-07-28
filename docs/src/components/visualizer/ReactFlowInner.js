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
  // Fallback to default for others
  const DefaultNode = ({ data }) => (
    <div style={{ padding: 10, border: '1px solid #ccc', borderRadius: 8, background: '#fff' }}>
      {data?.label || 'Node'}
      <Handle type="source" position="right" />
      <Handle type="target" position="left" />
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
