/**
 * @fileoverview Bridge-Based Edge Components
 * 
 * ReactFlow edge components for standard and hyper edges
 */

import React from 'react';
import { BaseEdge, EdgeProps, getStraightPath } from '@xyflow/react';

/**
 * Standard graph edge component
 */
export function StandardEdge(props: EdgeProps) {
  const [edgePath] = getStraightPath({
    sourceX: props.sourceX,
    sourceY: props.sourceY,
    targetX: props.targetX,
    targetY: props.targetY,
  });

  return (
    <BaseEdge
      path={edgePath}
      markerEnd={props.markerEnd}
      style={{ stroke: '#1976d2', strokeWidth: 2 }}
    />
  );
}

/**
 * HyperEdge component
 */
export function HyperEdge(props: EdgeProps) {
  const [edgePath] = getStraightPath({
    sourceX: props.sourceX,
    sourceY: props.sourceY,
    targetX: props.targetX,
    targetY: props.targetY,
  });

  return (
    <BaseEdge
      path={edgePath}
      markerEnd={props.markerEnd}
      style={{ 
        stroke: '#ff5722', 
        strokeWidth: 3, 
        strokeDasharray: '5,5' 
      }}
    />
  );
}

// Export map for ReactFlow edgeTypes
export const edgeTypes = {
  standard: StandardEdge,
  hyper: HyperEdge
};
