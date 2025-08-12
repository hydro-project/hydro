/**
 * @fileoverview Floating Edge Component
 * 
 * Custom edge that calculates dynamic attachment points on node perimeters
 * Based on ReactFlow's Simple Floating Edges example
 */

import { useCallback } from 'react';
import { getBezierPath, useStore, EdgeProps } from '@xyflow/react';

// Utility function to get edge parameters for floating connection
function getEdgeParams(source: any, target: any) {
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

// Calculate intersection point on node rectangle
function getNodeIntersection(intersectionNode: any, targetNode: any) {
  const {
    measured: { width: intersectionNodeWidth = 120, height: intersectionNodeHeight = 40 },
    internals: { positionAbsolute: intersectionNodePosition },
  } = intersectionNode;
  const {
    measured: { width: targetNodeWidth = 120, height: targetNodeHeight = 40 },
    internals: { positionAbsolute: targetPosition },
  } = targetNode;

  const w = intersectionNodeWidth / 2;
  const h = intersectionNodeHeight / 2;

  const x2 = intersectionNodePosition.x + w;
  const y2 = intersectionNodePosition.y + h;
  const x1 = targetPosition.x + targetNodeWidth / 2;
  const y1 = targetPosition.y + targetNodeHeight / 2;

  const xx1 = (x1 - x2) / (2 * w) - (y1 - y2) / (2 * h);
  const yy1 = (x1 - x2) / (2 * w) + (y1 - y2) / (2 * h);
  const a = 1 / (Math.abs(xx1) + Math.abs(yy1));
  const xx3 = a * xx1;
  const yy3 = a * yy1;
  const x = w * (xx3 + yy3) + x2;
  const y = h * (-xx3 + yy3) + y2;

  return { x, y };
}

// Get edge position (which side of the node)
function getEdgePosition(node: any, intersectionPoint: any) {
  const n = { 
    ...node.internals.positionAbsolute, 
    width: node.measured.width || 120,
    height: node.measured.height || 40
  };
  const nx = Math.round(n.x);
  const ny = Math.round(n.y);
  const px = Math.round(intersectionPoint.x);
  const py = Math.round(intersectionPoint.y);

  if (px <= nx + 1) {
    return 'left';
  }
  if (px >= nx + n.width - 1) {
    return 'right';
  }
  if (py <= ny + 1) {
    return 'top';
  }
  if (py >= ny + n.height - 1) {
    return 'bottom';
  }

  return 'top';
}

export default function FloatingEdge({ id, source, target, markerEnd, style }: EdgeProps) {
  const sourceNode = useStore(useCallback((store) => store.nodeLookup.get(source), [source]));
  const targetNode = useStore(useCallback((store) => store.nodeLookup.get(target), [target]));

  if (!sourceNode || !targetNode) {
    return null;
  }

  const { sx, sy, tx, ty } = getEdgeParams(sourceNode, targetNode);

  const [edgePath] = getBezierPath({
    sourceX: sx,
    sourceY: sy,
    targetX: tx,
    targetY: ty,
  });

  return (
    <path
      id={id}
      className="react-flow__edge-path"
      d={edgePath}
      markerEnd={markerEnd}
      style={style}
    />
  );
}
