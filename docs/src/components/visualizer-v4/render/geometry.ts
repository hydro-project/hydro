/**
 * @fileoverview Geometry helpers for edge rendering
 */

/** Calculate intersection point on node rectangle */
export function getNodeIntersection(intersectionNode: any, targetNode: any) {
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
    x: typeof intersectionNodePosition?.x === 'number' && !isNaN(intersectionNodePosition.x) && isFinite(intersectionNodePosition.x) ? intersectionNodePosition.x : 0,
    y: typeof intersectionNodePosition?.y === 'number' && !isNaN(intersectionNodePosition.y) && isFinite(intersectionNodePosition.y) ? intersectionNodePosition.y : 0,
  };

  const safeTargetPos = {
    x: typeof targetPosition?.x === 'number' && !isNaN(targetPosition.x) && isFinite(targetPosition.x) ? targetPosition.x : 0,
    y: typeof targetPosition?.y === 'number' && !isNaN(targetPosition.y) && isFinite(targetPosition.y) ? targetPosition.y : 0,
  };

  const safeIntersectionWidth = typeof intersectionNodeWidth === 'number' && !isNaN(intersectionNodeWidth) && isFinite(intersectionNodeWidth) && intersectionNodeWidth > 0 ? intersectionNodeWidth : 120;
  const safeIntersectionHeight = typeof intersectionNodeHeight === 'number' && !isNaN(intersectionNodeHeight) && isFinite(intersectionNodeHeight) && intersectionNodeHeight > 0 ? intersectionNodeHeight : 40;
  const safeTargetWidth = typeof targetNodeWidth === 'number' && !isNaN(targetNodeWidth) && isFinite(targetNodeWidth) && targetNodeWidth > 0 ? targetNodeWidth : 120;
  const safeTargetHeight = typeof targetNodeHeight === 'number' && !isNaN(targetNodeHeight) && isFinite(targetNodeHeight) && targetNodeHeight > 0 ? targetNodeHeight : 40;

  const w = safeIntersectionWidth / 2;
  const h = safeIntersectionHeight / 2;

  const x2 = safeIntersectionPos.x + w;
  const y2 = safeIntersectionPos.y + h;
  const x1 = safeTargetPos.x + safeTargetWidth / 2;
  const y1 = safeTargetPos.y + safeTargetHeight / 2;

  const xx1 = (x1 - x2) / (2 * w) - (y1 - y2) / (2 * h);
  const yy1 = (x1 - x2) / (2 * w) + (y1 - y2) / (2 * h);

  // Prevent division by zero when nodes are at the same position
  const denominator = Math.abs(xx1) + Math.abs(yy1);
  if (denominator === 0) {
    return {
      x: safeIntersectionPos.x + safeIntersectionWidth / 4,
      y: safeIntersectionPos.y,
    };
  }

  const a = 1 / denominator;
  const xx3 = a * xx1;
  const yy3 = a * yy1;
  const x = w * (xx3 + yy3) + x2;
  const y = h * (-xx3 + yy3) + y2;

  const safeX = typeof x === 'number' && !isNaN(x) && isFinite(x) ? x : safeIntersectionPos.x;
  const safeY = typeof y === 'number' && !isNaN(y) && isFinite(y) ? y : safeIntersectionPos.y;

  return { x: safeX, y: safeY };
}

/** Determine which side of a node a point lies on */
export function getEdgePosition(node: any, intersectionPoint: any) {
  const nodePos = node.internals?.positionAbsolute;
  const nodeWidth = node.measured?.width || 120;
  const nodeHeight = node.measured?.height || 40;

  const safeNodePos = {
    x: typeof nodePos?.x === 'number' && !isNaN(nodePos.x) && isFinite(nodePos.x) ? nodePos.x : 0,
    y: typeof nodePos?.y === 'number' && !isNaN(nodePos.y) && isFinite(nodePos.y) ? nodePos.y : 0,
  };

  const safeWidth = typeof nodeWidth === 'number' && !isNaN(nodeWidth) && isFinite(nodeWidth) && nodeWidth > 0 ? nodeWidth : 120;
  const safeHeight = typeof nodeHeight === 'number' && !isNaN(nodeHeight) && isFinite(nodeHeight) && nodeHeight > 0 ? nodeHeight : 40;

  const safeIntersectionPoint = {
    x: typeof intersectionPoint?.x === 'number' && !isNaN(intersectionPoint.x) && isFinite(intersectionPoint.x) ? intersectionPoint.x : safeNodePos.x,
    y: typeof intersectionPoint?.y === 'number' && !isNaN(intersectionPoint.y) && isFinite(intersectionPoint.y) ? intersectionPoint.y : safeNodePos.y,
  };

  const n = { ...safeNodePos, width: safeWidth, height: safeHeight } as const;
  const nx = Math.round(n.x);
  const ny = Math.round(n.y);
  const px = Math.round(safeIntersectionPoint.x);
  const py = Math.round(safeIntersectionPoint.y);

  if (px <= nx + 1) return 'left';
  if (px >= nx + n.width - 1) return 'right';
  if (py <= ny + 1) return 'top';
  if (py >= ny + n.height - 1) return 'bottom';

  return 'top';
}
