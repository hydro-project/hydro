/**
 * @fileoverview Floating Edge Component
 * 
 * Custom edge that calculates dynamic attachment points on node perimeters
 * Based on ReactFlow's Simple Floating Edges example
 */

import React, { useCallback } from 'react';
import { getBezierPath, getStraightPath, getSmoothStepPath, useStore, EdgeProps } from '@xyflow/react';
import { useStyleConfig } from './StyleConfigContext';
import { getStroke, getHaloColor, stripHaloStyle } from './edgeStyle';
import { getNodeIntersection, getEdgePosition } from './geometry';

// Utility function to get edge parameters for floating connection
function getEdgeParams(source: Parameters<typeof getNodeIntersection>[0], target: Parameters<typeof getNodeIntersection>[1]) {
  const sourceIntersectionPoint = getNodeIntersection(source, target);
  const targetIntersectionPoint = getNodeIntersection(target, source);

  const sourcePos = getEdgePosition(source, sourceIntersectionPoint);
  const targetPos = getEdgePosition(target, targetIntersectionPoint);

  return {
    sx: sourceIntersectionPoint.x,
    sy: sourceIntersectionPoint.y,
    tx: targetIntersectionPoint.x,
    ty: targetIntersectionPoint.y,
    sourcePos,
    targetPos,
  };
}

// geometry helpers moved to ./geometry

export default function FloatingEdge({ id, source, target, style = {}, markerEnd }: EdgeProps) {
  const styleCfg = useStyleConfig();
  const sourceNode = useStore(useCallback((store) => store.nodeLookup.get(source), [source]));
  const targetNode = useStore(useCallback((store) => store.nodeLookup.get(target), [target]));

  if (!sourceNode || !targetNode) {
    return null;
  }

  const { sx, sy, tx, ty } = getEdgeParams(sourceNode, targetNode);

  // Basic validation of edge coordinates
  const safeSx = (typeof sx === 'number' && !isNaN(sx) && isFinite(sx)) ? sx : 0;
  const safeSy = (typeof sy === 'number' && !isNaN(sy) && isFinite(sy)) ? sy : 0;
  const safeTx = (typeof tx === 'number' && !isNaN(tx) && isFinite(tx)) ? tx : 100;
  const safeTy = (typeof ty === 'number' && !isNaN(ty) && isFinite(ty)) ? ty : 100;

  let edgePath: string = '';
  if (styleCfg.edgeStyle === 'straight') {
    [edgePath] = getStraightPath({
      sourceX: safeSx,
      sourceY: safeSy,
      targetX: safeTx,
      targetY: safeTy,
    });
  } else if (styleCfg.edgeStyle === 'smoothstep') {
    [edgePath] = getSmoothStepPath({
      sourceX: safeSx,
      sourceY: safeSy,
      targetX: safeTx,
      targetY: safeTy,
      sourcePosition: undefined,
      targetPosition: undefined,
    });
  } else {
    [edgePath] = getBezierPath({
      sourceX: safeSx,
      sourceY: safeSy,
      targetX: safeTx,
      targetY: safeTy,
    });
  }

  const { stroke, strokeWidth, strokeDasharray } = getStroke(styleCfg, style);
  const haloColor = getHaloColor(style);

  // Simple rendering for edges without halos
  if (!haloColor) {
    return (
      <path
        id={id}
        className="react-flow__edge-path"
        d={edgePath}
        markerEnd={markerEnd}
        style={{
          ...style,
          stroke,
          strokeWidth,
          strokeDasharray,
        }}
      />
    );
  }

  // Complex rendering for edges with halos
  return (
    <g>
      {/* Render halo layer */}
      <path
        className="react-flow__edge-path"
        d={edgePath}
  style={{
          stroke: haloColor,
          strokeWidth: strokeWidth + 4,
          strokeDasharray,
          strokeLinecap: 'round',
          opacity: 0.6,
          fill: 'none'
        }}
      />
      
      {/* Render main edge */}
      <path
        id={id}
        className="react-flow__edge-path"
        d={edgePath}
        markerEnd={markerEnd}
        style={{
          ...(stripHaloStyle(style as unknown as Record<string, unknown>)),
          stroke,
          strokeWidth,
          strokeDasharray,
        } as unknown as React.CSSProperties}
      />
    </g>
  );
}

// Internal memoized variant for use in edgeTypes to reduce re-renders
export const MemoFloatingEdge = React.memo(FloatingEdge);
