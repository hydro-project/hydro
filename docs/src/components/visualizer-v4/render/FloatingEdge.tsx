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

  // Validate all coordinates before calculations to prevent NaN propagation
  const safeIntersectionPos = {
    x: (typeof intersectionNodePosition?.x === 'number' && !isNaN(intersectionNodePosition.x) && isFinite(intersectionNodePosition.x)) ? intersectionNodePosition.x : 0,
    y: (typeof intersectionNodePosition?.y === 'number' && !isNaN(intersectionNodePosition.y) && isFinite(intersectionNodePosition.y)) ? intersectionNodePosition.y : 0
  };
  
  const safeTargetPos = {
    x: (typeof targetPosition?.x === 'number' && !isNaN(targetPosition.x) && isFinite(targetPosition.x)) ? targetPosition.x : 0,
    y: (typeof targetPosition?.y === 'number' && !isNaN(targetPosition.y) && isFinite(targetPosition.y)) ? targetPosition.y : 0
  };

  const safeIntersectionWidth = (typeof intersectionNodeWidth === 'number' && !isNaN(intersectionNodeWidth) && isFinite(intersectionNodeWidth) && intersectionNodeWidth > 0) ? intersectionNodeWidth : 120;
  const safeIntersectionHeight = (typeof intersectionNodeHeight === 'number' && !isNaN(intersectionNodeHeight) && isFinite(intersectionNodeHeight) && intersectionNodeHeight > 0) ? intersectionNodeHeight : 40;
  const safeTargetWidth = (typeof targetNodeWidth === 'number' && !isNaN(targetNodeWidth) && isFinite(targetNodeWidth) && targetNodeWidth > 0) ? targetNodeWidth : 120;
  const safeTargetHeight = (typeof targetNodeHeight === 'number' && !isNaN(targetNodeHeight) && isFinite(targetNodeHeight) && targetNodeHeight > 0) ? targetNodeHeight : 40;

  const w = safeIntersectionWidth / 2;
  const h = safeIntersectionHeight / 2;

  const x2 = safeIntersectionPos.x + w;
  const y2 = safeIntersectionPos.y + h;
  const x1 = safeTargetPos.x + safeTargetWidth / 2;
  const y1 = safeTargetPos.y + safeTargetHeight / 2;

  const xx1 = (x1 - x2) / (2 * w) - (y1 - y2) / (2 * h);
  const yy1 = (x1 - x2) / (2 * w) + (y1 - y2) / (2 * h);
  
  // BUG FIX: Prevent division by zero when nodes are at the same position
  const denominator = Math.abs(xx1) + Math.abs(yy1);
  if (denominator === 0) {
    // Nodes are at the same position - return a default offset to avoid NaN
    return { 
      x: safeIntersectionPos.x + safeIntersectionWidth / 4, // Small offset to the right
      y: safeIntersectionPos.y 
    };
  }
  
  const a = 1 / denominator;
  const xx3 = a * xx1;
  const yy3 = a * yy1;
  const x = w * (xx3 + yy3) + x2;
  const y = h * (-xx3 + yy3) + y2;

  // Final safety check (should rarely be needed now)
  const safeX = (typeof x === 'number' && !isNaN(x) && isFinite(x)) ? x : safeIntersectionPos.x;
  const safeY = (typeof y === 'number' && !isNaN(y) && isFinite(y)) ? y : safeIntersectionPos.y;

  return { x: safeX, y: safeY };
}

// Get edge position (which side of the node)
function getEdgePosition(node: any, intersectionPoint: any) {
  const nodePos = node.internals?.positionAbsolute;
  const nodeWidth = node.measured?.width || 120;
  const nodeHeight = node.measured?.height || 40;
  
  // Validate node position and dimensions
  const safeNodePos = {
    x: (typeof nodePos?.x === 'number' && !isNaN(nodePos.x) && isFinite(nodePos.x)) ? nodePos.x : 0,
    y: (typeof nodePos?.y === 'number' && !isNaN(nodePos.y) && isFinite(nodePos.y)) ? nodePos.y : 0
  };
  
  const safeWidth = (typeof nodeWidth === 'number' && !isNaN(nodeWidth) && isFinite(nodeWidth) && nodeWidth > 0) ? nodeWidth : 120;
  const safeHeight = (typeof nodeHeight === 'number' && !isNaN(nodeHeight) && isFinite(nodeHeight) && nodeHeight > 0) ? nodeHeight : 40;
  
  // Validate intersection point
  const safeIntersectionPoint = {
    x: (typeof intersectionPoint?.x === 'number' && !isNaN(intersectionPoint.x) && isFinite(intersectionPoint.x)) ? intersectionPoint.x : safeNodePos.x,
    y: (typeof intersectionPoint?.y === 'number' && !isNaN(intersectionPoint.y) && isFinite(intersectionPoint.y)) ? intersectionPoint.y : safeNodePos.y
  };
  
  const n = { 
    ...safeNodePos, 
    width: safeWidth,
    height: safeHeight
  };
  const nx = Math.round(n.x);
  const ny = Math.round(n.y);
  const px = Math.round(safeIntersectionPoint.x);
  const py = Math.round(safeIntersectionPoint.y);

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

  // Basic validation of edge coordinates
  const safeSx = (typeof sx === 'number' && !isNaN(sx) && isFinite(sx)) ? sx : 0;
  const safeSy = (typeof sy === 'number' && !isNaN(sy) && isFinite(sy)) ? sy : 0;
  const safeTx = (typeof tx === 'number' && !isNaN(tx) && isFinite(tx)) ? tx : 100;
  const safeTy = (typeof ty === 'number' && !isNaN(ty) && isFinite(ty)) ? ty : 100;

  const [edgePath] = getBezierPath({
    sourceX: safeSx,
    sourceY: safeSy,
    targetX: safeTx,
    targetY: safeTy,
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
