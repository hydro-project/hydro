/**
 * @fileoverview Bridge-Based Edge Components
 * 
 * ReactFlow edge components for standard and hyper edges
 */

import React from 'react';
import { BaseEdge, EdgeProps, getStraightPath, getBezierPath, getSmoothStepPath } from '@xyflow/react';
import FloatingEdge from './FloatingEdge';
import { useStyleConfig } from './StyleConfigContext';

/**
 * Standard graph edge component - uses ReactFlow's automatic routing
 */
export function StandardEdge(props: EdgeProps) {
  const styleCfg = useStyleConfig();

  let edgePath: string;
  if (styleCfg.edgeStyle === 'straight') {
    [edgePath] = getStraightPath({
      sourceX: props.sourceX,
      sourceY: props.sourceY,
      targetX: props.targetX,
      targetY: props.targetY,
    });
  } else if (styleCfg.edgeStyle === 'smoothstep') {
    [edgePath] = getSmoothStepPath({
      sourceX: props.sourceX,
      sourceY: props.sourceY,
      targetX: props.targetX,
      targetY: props.targetY,
      sourcePosition: props.sourcePosition,
      targetPosition: props.targetPosition,
    });
  } else {
    [edgePath] = getBezierPath({
      sourceX: props.sourceX,
      sourceY: props.sourceY,
      targetX: props.targetX,
      targetY: props.targetY,
      sourcePosition: props.sourcePosition,
      targetPosition: props.targetPosition,
    });
  }

  return (
    <BaseEdge
      path={edgePath}
      markerEnd={props.markerEnd}
      style={{ 
        stroke: styleCfg.edgeColor || '#1976d2', 
        strokeWidth: styleCfg.edgeWidth ?? 2,
        strokeDasharray: styleCfg.edgeDashed ? '6,6' : undefined
      }}
    />
  );
}

/**
 * HyperEdge component
 */
export function HyperEdge(props: EdgeProps) {
  const styleCfg = useStyleConfig();

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
        stroke: styleCfg.edgeColor || '#ff5722', 
        strokeWidth: (styleCfg.edgeWidth ?? 3), 
        strokeDasharray: styleCfg.edgeDashed ? '5,5' : '5,5' 
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
