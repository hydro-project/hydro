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
import { ContainerNode } from './CustomNodes.js';
import { generateLocationBorderColor, generateNodeColors } from './colorUtils.js';
import styles from '../../pages/visualizer.module.css';

export function ReactFlowInner({ nodes, edges, onNodesChange, onEdgesChange, locationData, colorPalette, onContainerToggle }) {
  const onConnect = useCallback((connection) => {
    onEdgesChange(addEdge(connection, edges));
  }, [onEdgesChange, edges]);

  // ReactFlow v12: Enhanced node types with better custom components
  const nodeTypes = useMemo(() => ({
    container: ContainerNode,
    // label: removed - using built-in 'default' type with custom styling
    // Use default type for most nodes to leverage v12 improvements
  }), []);

  // ReactFlow v12: Enhanced default edge options
  const defaultEdgeOptions = useMemo(() => ({
    type: 'smoothstep', // Better routing in v12
    animated: false,
    zIndex: 1000, // Ensure edges render above nodes
    // CRITICAL: Ensure edges render properly within parent containers
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
    // CRITICAL: Force edge rendering
    hidden: false,
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
        // ReactFlow v12: Sub-flow support
        elevateEdgesOnSelect={true}
        // CRITICAL: Enable sub-flow edge rendering for parent-child relationships
        disableKeyboardA11y={false}
        // CRITICAL: Ensure parent-child edge rendering works
        onlyRenderVisibleElements={false}
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
