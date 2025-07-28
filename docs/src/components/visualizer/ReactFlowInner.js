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
  const onConnect = useCallback((connection) => {
    onEdgesChange(addEdge(connection, edges));
  }, [onEdgesChange, edges]);

  const miniMapNodeColor = useCallback((node) => {
    return getMiniMapNodeColor(node, colorPalette);
  }, [colorPalette]);

  // Define custom node types including our group node
  const nodeTypes = useMemo(() => ({
    group: GroupNode,
  }), []);

  // CRITICAL DEBUG: Log what ReactFlow is actually receiving
  console.log('=== REACTFLOW INNER COMPONENT DEBUG ===');
  console.log('ReactFlow receiving nodes:', nodes.map(n => ({
    id: n.id,
    type: n.type,
    parentId: n.parentId, // FIXED: ReactFlow v12 uses parentId
    position: n.position,
    hasStyle: !!n.style,
    hasData: !!n.data,
    dataLabel: n.data?.label,
  })));
  
  const groupNodes = nodes.filter(n => n.type === 'group');
  console.log('ReactFlow group nodes parent-child check:', groupNodes.map(n => {
    const parent = nodes.find(p => p.id === n.parentId);
    return {
      id: n.id,
      label: n.data?.label,
      parentId: n.parentId, // FIXED: ReactFlow v12 uses parentId
      parentExists: !!parent,
      parentLabel: parent?.data?.label,
    };
  }));

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
