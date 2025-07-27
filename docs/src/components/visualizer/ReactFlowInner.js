/**
 * ReactFlow Inner Component
 * 
 * Inner component that uses ReactFlow hooks and renders the actual graph
 */

import React, { useCallback } from 'react';
import { 
  ReactFlow, 
  Controls, 
  MiniMap, 
  Background, 
  addEdge 
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import { ContainerNode, LabelNode } from './CustomNodes.js';
import CustomEdge from './CustomEdge';
import { generateLocationBorderColor, generateNodeColors } from './colorUtils.js';
import styles from '../../pages/visualizer.module.css';

export function ReactFlowInner({ nodes, edges, onNodesChange, onEdgesChange, locationData, colorPalette, onContainerToggle }) {
  const onConnect = useCallback((connection) => {
    onEdgesChange(addEdge(connection, edges));
  }, [onEdgesChange, edges]);

  const nodeTypes = {
    label: LabelNode,
    container: ContainerNode,
  };

  const edgeTypes = {
    custom: CustomEdge,
  };

  return (
    <div className={styles.reactflowWrapper}>
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onConnect={onConnect}
        nodeTypes={nodeTypes}
        edgeTypes={edgeTypes}
        fitView
        attributionPosition="bottom-left"
        nodesDraggable={true}
        nodesConnectable={true}
        elementsSelectable={true}
      >
        <Controls />
        <MiniMap 
          nodeColor={(node) => {
            if (node.data?.isContainer) {
              const locationId = node.data.locationId;
              return generateLocationBorderColor(locationId, locationData?.size || 1, colorPalette);
            }
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
