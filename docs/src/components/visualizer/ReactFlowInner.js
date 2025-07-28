/**
 * ReactFlow Inner Component for v12
 * 
 * Leverages ReactFlow v12 features including sub-flows, better edge routing,
 * and improved state management
 */

import React, { useCallback, useMemo } from 'react';
import { 
  ReactFlow, 
  Controls, 
  MiniMap, 
  Background, 
  addEdge,
  useReactFlow 
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import { generateLocationBorderColor, generateNodeColors } from './colorUtils.js';
import styles from '../../pages/visualizer.module.css';

export function ReactFlowInner({ nodes, edges, onNodesChange, onEdgesChange, locationData, colorPalette }) {
  const onConnect = useCallback((connection) => {
    onEdgesChange(addEdge(connection, edges));
  }, [onEdgesChange, edges]);

  // ReactFlow v12: Using built-in node types only
  const nodeTypes = useMemo(() => ({}), []);

  // ReactFlow v12: Enhanced default edge options
  const defaultEdgeOptions = useMemo(() => ({
    type: 'smoothstep', // Better routing in v12
    animated: false,
    style: {
      strokeWidth: 2,
      stroke: '#666666',
    },
    markerEnd: {
      type: 'arrowclosed',
      width: 20,
      height: 20,
      color: '#666666',
    },
  }), []);

  return (
    <div className={styles.reactflowWrapper}>
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onConnect={onConnect}
        nodeTypes={nodeTypes}
        defaultEdgeOptions={defaultEdgeOptions}
        fitView
        attributionPosition="bottom-left"
        nodesDraggable={true}
        nodesConnectable={true}
        elementsSelectable={true}
        // ReactFlow v12: Better performance options
        nodeOrigin={[0.5, 0.5]} // Center node positioning
        maxZoom={2}
        minZoom={0.1}
        elevateEdgesOnSelect={true}
        disableKeyboardA11y={false}
      >
        <Controls />
        <MiniMap 
          nodeColor={(node) => {
            const nodeColors = generateNodeColors(node.data?.type || 'Transform', colorPalette);
            return nodeColors.primary;
          }}
          nodeStrokeWidth={2}
          nodeStrokeColor="#666"
          maskColor="rgba(240, 240, 240, 0.6)"
        />
        <Background color="#f5f5f5" gap={20} />
      </ReactFlow>
    </div>
  );
}
