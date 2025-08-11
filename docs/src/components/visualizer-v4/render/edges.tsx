/**
 * @fileoverview Bridge-Based Edge Components
 * 
 * ReactFlow edge components for standard and hyper edges
 */

import React from 'react';
import { BaseEdge, EdgeProps, getStraightPath, getBezierPath } from '@xyflow/react';
import FloatingEdge from './FloatingEdge';

/**
 * Standard graph edge component - uses ReactFlow's automatic routing
 */
export function StandardEdge(props: EdgeProps) {
  // Use ReactFlow's automatic routing for consistent coordinate system
  console.log(`[StandardEdge] DEBUG Edge ${props.id}:`, {
    sourceX: props.sourceX,
    sourceY: props.sourceY,
    targetX: props.targetX,
    targetY: props.targetY,
    sourcePosition: props.sourcePosition,
    targetPosition: props.targetPosition
  });
  
  // Try Bezier path for better edge routing
  const [edgePath] = getBezierPath({
    sourceX: props.sourceX,
    sourceY: props.sourceY,
    targetX: props.targetX,
    targetY: props.targetY,
    sourcePosition: props.sourcePosition,
    targetPosition: props.targetPosition,
  });

  console.log(`[StandardEdge] Generated path for ${props.id}: ${edgePath}`);

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
  hyper: HyperEdge,
  floating: FloatingEdge
};
